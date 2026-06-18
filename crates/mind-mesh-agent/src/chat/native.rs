use std::collections::HashMap;
use std::sync::Arc;

use adk_core::{Content, RunConfig, StreamingMode};
use adk_runner::Runner;
use adk_session::{CreateRequest, GetRequest, InMemorySessionService, SessionService};
use anyhow::{Context, Result};
use futures::StreamExt;
use mind_mesh_core::{
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

    pub(crate) async fn run_turn_native(
        &self,
        session_id: &str,
        query: &str,
        project: Option<&str>,
        repo_path: Option<&str>,
        mut on_chunk: impl FnMut(&str),
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

        let mut answer = String::new();
        let mut tool_tracker = ToolCallTracker::new();
        let mut usage = ChatTokenUsage::default();
        let deadline = Instant::now() + ASK_TIMEOUT;
        let mut phase = ChatPhase::Thinking;
        on_phase(phase);
        let mut post_tool_answer = String::new();
        // Collect model text only after the most recent tool batch finished.
        let mut collect_final_answer = false;

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
                on_tool_calls(tool_tracker.records());
                if tool_tracker.has_running() {
                    collect_final_answer = false;
                    if phase != ChatPhase::Tools {
                        phase = ChatPhase::Tools;
                        on_phase(phase);
                    }
                } else if tool_tracker.all_done() {
                    // Agent may run multiple tool rounds; emit generating after each batch.
                    collect_final_answer = true;
                    post_tool_answer.clear();
                    if phase != ChatPhase::Generating {
                        phase = ChatPhase::Generating;
                        on_phase(phase);
                    }
                }
            }

            if collect_final_answer {
                append_event_text(&event, &mut answer, &mut |text| {
                    post_tool_answer.push_str(text);
                    on_chunk(text);
                    if phase != ChatPhase::Streaming {
                        phase = ChatPhase::Streaming;
                        on_phase(phase);
                    }
                });
            } else {
                append_event_text(&event, &mut answer, &mut |text| {
                    if !tool_tracker.has_any() {
                        on_chunk(text);
                        if phase != ChatPhase::Streaming {
                            phase = ChatPhase::Streaming;
                            on_phase(phase);
                        }
                    }
                });
            }
        }

        if tool_tracker.has_any() {
            answer = post_tool_answer;
        }
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

        let mut citations = self
            .search_citations(query, project, repo_path)
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "citation search failed; continuing without search hits");
                Vec::new()
            });
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

fn append_event_text(
    event: &adk_core::Event,
    answer: &mut String,
    on_chunk: &mut impl FnMut(&str),
) {
    let Some(content) = &event.llm_response.content else {
        return;
    };
    for part in &content.parts {
        let text = part.text().unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        on_chunk(text);
        answer.push_str(text);
    }
}
