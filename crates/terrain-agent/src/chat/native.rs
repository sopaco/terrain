use std::collections::HashMap;
use std::sync::Arc;

use adk_core::{Content, RunConfig, StreamingMode};
use adk_runner::Runner;
use adk_session::{CreateRequest, GetRequest, InMemorySessionService, SessionService};
use anyhow::{Context, Result};
use futures::StreamExt;
use terrain_core::{
    extract_source_citations, merge_citations, KnowledgePaths,
};
use tokio::time::{timeout, Instant};

use crate::acp::resolve_acp_settings;
use crate::agent_assets::{ensure_agent_assets, AgentAssetsEnsureReport};
use crate::builder::{build_agent, AgentConfig};
use crate::context_generator::AgentContextGenerator;
use crate::model::{build_llm, ensure_llm, ModelConfig};

use super::{
    finalize_usage, sanitize_answer_text, CHAT_APP_NAME, CHAT_USER_ID, ASK_TIMEOUT,
};
use super::prompt::build_ask_prompt;
use super::tracker::{now_ms, ToolCallTracker};
use super::types::{ChatPhase, ChatReply, ChatTokenUsage, ChatToolCallRecord};

pub(crate) struct ChatContextGenerator {
    paths: KnowledgePaths,
    model_config: ModelConfig,
}

#[async_trait::async_trait]
impl AgentContextGenerator for ChatContextGenerator {
    async fn ensure_ready(
        &self,
        project_slug: &str,
        repo_path: Option<&str>,
    ) -> anyhow::Result<AgentAssetsEnsureReport> {
        ensure_agent_assets(&self.paths, &self.model_config, project_slug, repo_path).await
    }
}

pub(crate) struct NativeBackend {
    pub(crate) runner: Runner,
    pub(crate) session_service: Arc<InMemorySessionService>,
}

pub(crate) fn build_native_backend(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
) -> Result<NativeBackend> {
    ensure_llm(model_config)?;
    let llm = build_llm(model_config)?;
    let context_generator: Arc<dyn AgentContextGenerator> = Arc::new(ChatContextGenerator {
        paths: paths.clone(),
        model_config: model_config.clone(),
    });
    let agent = build_agent(
        llm,
        AgentConfig {
            knowledge_root: paths.clone(),
            project_cwd: None,
            acp_settings: resolve_acp_settings(),
            enable_acp_delegate: false,
            ask_mode: true,
            context_generator: Some(context_generator),
        },
    )?;

    let session_service = Arc::new(InMemorySessionService::new());
    let runner = Runner::builder()
        .app_name(CHAT_APP_NAME)
        .agent(agent)
        .session_service(session_service.clone())
        .run_config(
            RunConfig::builder()
                .streaming_mode(StreamingMode::SSE)
                .build(),
        )
        .build()
        .context("failed to build chat runner")?;

    Ok(NativeBackend {
        runner,
        session_service,
    })
}

impl super::ChatEngine {
    pub(crate) fn native(&self) -> Result<&NativeBackend> {
        self.native
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("native chat backend not initialized"))
    }

    async fn ensure_session(&self, session_id: &str) -> Result<()> {
        let backend = self.native()?;
        let get_result = backend
            .session_service
            .get(GetRequest {
                app_name: CHAT_APP_NAME.into(),
                user_id: CHAT_USER_ID.into(),
                session_id: session_id.into(),
                num_recent_events: None,
                after: None,
            })
            .await;

        if get_result.is_ok() {
            return Ok(());
        }

        backend
            .session_service
            .create(CreateRequest {
                app_name: CHAT_APP_NAME.into(),
                user_id: CHAT_USER_ID.into(),
                session_id: Some(session_id.into()),
                state: HashMap::new(),
            })
            .await
            .context("failed to create chat session")?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn run_turn_native(
        &self,
        session_id: &str,
        query: &str,
        project: Option<&str>,
        repo_path: Option<&str>,
        mut on_chunk: impl FnMut(&str),
        mut on_thinking_chunk: impl FnMut(&str),
        mut on_tool_calls: impl FnMut(&[ChatToolCallRecord]),
        mut on_phase: impl FnMut(ChatPhase),
        mut on_usage: impl FnMut(&ChatTokenUsage),
    ) -> Result<ChatReply> {
        let context = if session_id.starts_with("agent-ctx-") {
            query.to_string()
        } else {
            build_ask_prompt(query, project, &self.paths)?
        };
        let prompt_chars = context.len();

        self.ensure_session(session_id).await?;

        let backend = self.native()?;
        let content = Content::new("user").with_text(context);
        let mut stream = backend
            .runner
            .run_str(CHAT_USER_ID, session_id, content)
            .await
            .context("agent run failed")?;

        let mut answer_collector = ModelAnswerCollector::new();
        let mut tool_tracker = ToolCallTracker::new();
        let mut usage = ChatTokenUsage::default();
        let deadline = Instant::now() + ASK_TIMEOUT;
        let mut phase = ChatPhase::Thinking;
        on_phase(phase);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                anyhow::bail!(
                    "Ask timed out after {} seconds (try a narrower question or Repack)",
                    ASK_TIMEOUT.as_secs()
                );
            }

            let next = timeout(remaining, stream.next()).await.map_err(|_| {
                anyhow::anyhow!("Ask timed out after {} seconds", ASK_TIMEOUT.as_secs())
            })?;

            let Some(event_result) = next else {
                break;
            };

            let event = event_result
                .map_err(|e| anyhow::anyhow!("Ask failed for session {session_id}: {e}"))?;

            if let Some(meta) = &event.llm_response.usage_metadata {
                usage.input_tokens = usage
                    .input_tokens
                    .saturating_add(meta.prompt_token_count.max(0) as u32);
                usage.output_tokens = usage
                    .output_tokens
                    .saturating_add(meta.candidates_token_count.max(0) as u32);
                usage.total_tokens = usage
                    .total_tokens
                    .saturating_add(meta.total_token_count.max(0) as u32);
                on_usage(&usage);
            }

            if tool_tracker.ingest_event(&event) {
                answer_collector.note_tool_activity();
                on_tool_calls(tool_tracker.records());
                if tool_tracker.has_running() {
                    answer_collector.on_tools_running();
                    if phase != ChatPhase::Tools {
                        phase = ChatPhase::Tools;
                        on_phase(phase);
                    }
                } else if tool_tracker.has_any() && phase != ChatPhase::Generating {
                    phase = ChatPhase::Generating;
                    on_phase(phase);
                }
            }

            let stream_model_text = !tool_tracker.has_running();
            let event_text = extract_event_text(&event);
            if stream_model_text {
                if !event_text.thinking.is_empty() {
                    on_thinking_chunk(&event_text.thinking);
                }
                if !event_text.visible.is_empty() {
                    if phase != ChatPhase::Generating && phase != ChatPhase::Tools {
                        phase = ChatPhase::Generating;
                        on_phase(phase);
                    }
                    answer_collector.push_visible(&event_text.visible, &mut on_chunk);
                    if phase != ChatPhase::Tools && phase != ChatPhase::Streaming {
                        phase = ChatPhase::Streaming;
                        on_phase(phase);
                    }
                }
            } else {
                if !event_text.visible.is_empty() {
                    answer_collector.push_visible_silent(&event_text.visible);
                }
            }
        }

        let mut answer = answer_collector.finalize();
        let raw_answer = answer.clone();
        answer = sanitize_answer_text(&answer);
        self.paths.write_debug_file("last-ask-raw.md", &raw_answer);
        self.paths
            .write_debug_file("last-ask-sanitized.md", &answer);

        finalize_usage(&mut usage, prompt_chars, &answer);
        on_usage(&usage);

        if !tool_tracker.records().is_empty() {
            on_tool_calls(tool_tracker.records());
        }

        let mut citations = if tool_tracker
            .records()
            .iter()
            .any(|r| matches!(r.name.as_str(), "grep_agent_pack" | "read_agent_pack_file" | "read_agent_context"))
        {
            extract_source_citations(&answer, repo_path)
        } else {
            self.search_citations(query, project, repo_path)
                .unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "citation search failed; continuing without search hits");
                    Vec::new()
                })
        };
        citations = merge_citations(citations, extract_source_citations(&answer, repo_path));

        if answer.trim().is_empty() {
            answer = if citations.is_empty() && tool_tracker.records().is_empty() {
                "I could not find relevant information in the knowledge base.".into()
            } else if citations.is_empty() {
                "I finished reviewing the knowledge base and agent pack, but the model returned no final text. Expand tool calls above for retrieved context.".into()
            } else {
                format!(
                    "I found {} reference(s) in the knowledge base. See citations below.",
                    citations.len()
                )
            };
        }

        tracing::info!(
            session_id,
            answer_chars = answer.len(),
            tool_calls = tool_tracker.records().len(),
            "ask completed"
        );

        Ok(ChatReply {
            answer,
            citations,
            tool_calls: tool_tracker.records().to_vec(),
            usage,
            completed_at: now_ms(),
        })
    }
}

fn extract_event_text(event: &adk_core::Event) -> EventText {
    let Some(content) = &event.llm_response.content else {
        return EventText::default();
    };
    let mut visible = String::new();
    let mut thinking = String::new();
    for part in &content.parts {
        if let Some(text) = part.text() {
            if !text.is_empty() {
                visible.push_str(text);
            }
        } else if let Some(text) = part.thinking_text()
            && !text.is_empty() {
                thinking.push_str(text);
            }
    }
    EventText { visible, thinking }
}

#[derive(Default)]
struct EventText {
    visible: String,
    thinking: String,
}

/// Segments model text around tool execution windows.
struct ModelAnswerCollector {
    segments: Vec<String>,
    current: String,
    had_tools: bool,
}

impl ModelAnswerCollector {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            current: String::new(),
            had_tools: false,
        }
    }

    fn note_tool_activity(&mut self) {
        self.had_tools = true;
    }

    fn on_tools_running(&mut self) {
        self.flush_current();
    }

    fn push_visible(&mut self, text: &str, on_chunk: &mut impl FnMut(&str)) {
        on_chunk(text);
        self.current.push_str(text);
    }

    fn push_visible_silent(&mut self, text: &str) {
        self.current.push_str(text);
    }

    fn flush_current(&mut self) {
        if !self.current.is_empty() {
            self.segments.push(std::mem::take(&mut self.current));
        }
    }

    fn finalize(mut self) -> String {
        self.flush_current();
        select_model_answer(&self.segments, self.had_tools)
    }
}

fn select_model_answer(segments: &[String], had_tools: bool) -> String {
    let non_empty: Vec<&str> = segments
        .iter()
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .collect();
    if non_empty.is_empty() {
        return String::new();
    }
    if had_tools {
        non_empty.last().copied().unwrap_or("").to_string()
    } else {
        non_empty.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_model_answer_prefers_last_segment_after_tools() {
        let segments = vec![
            "Let me search.".into(),
            String::new(),
            "Here is the answer.".into(),
        ];
        assert_eq!(
            select_model_answer(&segments, true),
            "Here is the answer."
        );
    }

    #[test]
    fn select_model_answer_falls_back_to_earlier_segment_when_last_empty() {
        let segments = vec!["Draft before tools.".into(), String::new()];
        assert_eq!(
            select_model_answer(&segments, true),
            "Draft before tools."
        );
    }

    #[test]
    fn select_model_answer_joins_without_tools() {
        let segments = vec!["Hello ".into(), "world.".into()];
        assert_eq!(select_model_answer(&segments, false), "Hello world.");
    }
}
