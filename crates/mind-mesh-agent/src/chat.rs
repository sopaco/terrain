use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use adk_core::{Content, Part, RunConfig, StreamingMode};
use adk_runner::Runner;
use adk_session::{CreateRequest, GetRequest, InMemorySessionService, SessionService};
use anyhow::{Context, Result};
use futures::StreamExt;
use mind_mesh_core::{
    AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS, AgentPackMeta, KnowledgePaths, KnowledgeSearch,
    SearchOptions, SourceCitation, agent_pack_ready, build_context_overview,
    extract_source_citations, merge_citations, prepare_chat_markdown, read_agent_context_status,
    read_json, resolve_project_repo_path,
};
use serde::Serialize;

use tokio::time::{Duration, Instant, timeout};

use crate::acp::{
    acp_available, acp_spawn_command, build_acp_config, default_ask_acp_skill_dir,
    resolve_acp_settings,
};
use crate::agent_assets::{
    AgentAssetsEnsureReport, ensure_agent_assets, prepare_agent_assets_for_ask,
};
use crate::builder::{AgentConfig, build_agent};
use crate::context_generator::AgentContextGenerator;
use crate::model::{ModelConfig, build_llm, ensure_llm};
use crate::settings::{AcpSettings, AskExecution};
use crate::tool_session_cache::truncate_with_notice;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatToolCallStatus {
    Running,
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub status: ChatToolCallStatus,
    /// Unix epoch milliseconds when the tool call started.
    pub started_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ChatTokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatPhase {
    Thinking,
    Tools,
    Generating,
    Streaming,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub answer: String,
    pub citations: Vec<SourceCitation>,
    pub tool_calls: Vec<ChatToolCallRecord>,
    pub usage: ChatTokenUsage,
    pub completed_at: u64,
}

struct ToolCallTracker {
    records: Vec<ChatToolCallRecord>,
    by_id: HashMap<String, usize>,
    clocks: HashMap<String, Instant>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ToolCallTracker {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            by_id: HashMap::new(),
            clocks: HashMap::new(),
        }
    }

    fn ingest_event(&mut self, event: &adk_core::Event) -> bool {
        let Some(content) = &event.llm_response.content else {
            return false;
        };

        let mut changed = false;
        for part in &content.parts {
            match part {
                Part::FunctionCall { name, args, id, .. } => {
                    // Streaming providers may emit the same call across partial chunks.
                    if event.llm_response.partial {
                        if let Some(idx) = self.records.iter().rposition(|r| {
                            r.name == *name && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            self.records[idx].arguments = args.clone();
                            if let Some(call_id) = id {
                                self.records[idx].id = call_id.clone();
                                self.by_id.insert(call_id.clone(), idx);
                            }
                            changed = true;
                            continue;
                        }
                    }

                    let call_id = id.clone().unwrap_or_else(|| {
                        if let Some(idx) = self.records.iter().rposition(|r| {
                            r.name == *name && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            return self.records[idx].id.clone();
                        }
                        format!("{name}-{}", self.records.len())
                    });

                    if let Some(&idx) = self.by_id.get(&call_id) {
                        if self.records[idx].arguments != *args {
                            self.records[idx].arguments = args.clone();
                            changed = true;
                        }
                        continue;
                    }

                    let idx = self.records.len();
                    self.by_id.insert(call_id.clone(), idx);
                    self.clocks.insert(call_id.clone(), Instant::now());
                    self.records.push(ChatToolCallRecord {
                        id: call_id,
                        name: name.clone(),
                        arguments: args.clone(),
                        result: None,
                        error: None,
                        status: ChatToolCallStatus::Running,
                        started_at: now_ms(),
                        completed_at: None,
                        duration_ms: None,
                    });
                    changed = true;
                }
                Part::FunctionResponse {
                    function_response,
                    id,
                } => {
                    let mut matched = false;
                    if let Some(call_id) = id {
                        if let Some(&idx) = self.by_id.get(call_id) {
                            apply_tool_result(self, idx, &function_response.response);
                            matched = true;
                            changed = true;
                        }
                    }
                    if !matched {
                        let call_id = function_response.name.clone();
                        if let Some(&idx) = self.by_id.get(&call_id) {
                            if matches!(self.records[idx].status, ChatToolCallStatus::Running) {
                                apply_tool_result(self, idx, &function_response.response);
                                matched = true;
                                changed = true;
                            }
                        }
                    }
                    if !matched {
                        if let Some(idx) = self.records.iter().position(|r| {
                            r.name == function_response.name
                                && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            apply_tool_result(self, idx, &function_response.response);
                            changed = true;
                        } else {
                            let call_id =
                                id.clone().unwrap_or_else(|| function_response.name.clone());
                            let idx = self.records.len();
                            self.by_id.insert(call_id.clone(), idx);
                            self.clocks.insert(call_id.clone(), Instant::now());
                            self.records.push(ChatToolCallRecord {
                                id: call_id,
                                name: function_response.name.clone(),
                                arguments: serde_json::json!({}),
                                result: Some(function_response.response.clone()),
                                error: None,
                                status: ChatToolCallStatus::Ok,
                                started_at: now_ms(),
                                completed_at: Some(now_ms()),
                                duration_ms: Some(0),
                            });
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
        changed
    }

    fn records(&self) -> &[ChatToolCallRecord] {
        &self.records
    }

    fn has_running(&self) -> bool {
        self.records
            .iter()
            .any(|r| matches!(r.status, ChatToolCallStatus::Running))
    }

    fn has_any(&self) -> bool {
        !self.records.is_empty()
    }

    fn all_done(&self) -> bool {
        self.has_any() && !self.has_running()
    }
}

fn apply_tool_result(tracker: &mut ToolCallTracker, idx: usize, response: &serde_json::Value) {
    let call_id = tracker.records[idx].id.clone();
    let completed = now_ms();
    if let Some(exec_ms) = crate::throttle::take_tool_execution_ms(&call_id) {
        tracker.records[idx].duration_ms = Some(exec_ms);
    } else if let Some(started) = tracker.clocks.remove(&call_id) {
        tracker.records[idx].duration_ms = Some(started.elapsed().as_millis() as u64);
    }
    tracker.records[idx].completed_at = Some(completed);
    if let Some(err) = response.get("error").and_then(|v| v.as_str()) {
        tracker.records[idx].error = Some(err.to_string());
        tracker.records[idx].status = ChatToolCallStatus::Error;
    } else {
        tracker.records[idx].result = Some(response.clone());
        tracker.records[idx].status = ChatToolCallStatus::Ok;
    }
}

const CHAT_APP_NAME: &str = "mindmesh";
const CHAT_USER_ID: &str = "mindmesh-user";
const ASK_TIMEOUT: Duration = Duration::from_secs(1200);
const ASK_INJECT_DIR_TREE_MAX_CHARS: usize = 2_000;

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

fn finalize_usage(usage: &mut ChatTokenUsage, prompt_chars: usize, answer: &str) {
    if usage.total_tokens > 0 {
        return;
    }
    let input = ((prompt_chars as u32).saturating_add(3)) / 4;
    let output = ((answer.len() as u32).saturating_add(3)) / 4;
    if input == 0 && output == 0 {
        return;
    }
    usage.input_tokens = input.max(1);
    usage.output_tokens = if answer.is_empty() { 0 } else { output.max(1) };
    usage.total_tokens = usage.input_tokens + usage.output_tokens;
    usage.estimated = true;
}

fn sanitize_answer_text(text: &str) -> String {
    prepare_chat_markdown(text)
}

struct ChatContextGenerator {
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

struct NativeBackend {
    runner: Runner,
    session_service: Arc<InMemorySessionService>,
}

pub struct ChatEngine {
    paths: KnowledgePaths,
    model_config: ModelConfig,
    acp_settings: AcpSettings,
    native: Option<NativeBackend>,
}

impl ChatEngine {
    pub fn new(paths: KnowledgePaths, model_config: ModelConfig) -> Result<Self> {
        Self::with_settings(paths, model_config, resolve_acp_settings())
    }

    /// Always uses native LLM tools (for agent context generation, SDD LLM phases).
    pub fn new_native(paths: KnowledgePaths, model_config: ModelConfig) -> Result<Self> {
        let mut acp = resolve_acp_settings();
        acp.ask_execution = AskExecution::Native;
        Self::with_settings(paths, model_config, acp)
    }

    pub fn with_settings(
        paths: KnowledgePaths,
        model_config: ModelConfig,
        acp_settings: AcpSettings,
    ) -> Result<Self> {
        let native = if acp_settings.ask_execution == AskExecution::Native {
            Some(build_native_backend(&paths, &model_config)?)
        } else {
            #[cfg(feature = "opencode")]
            {
                if !acp_available(&acp_settings) {
                    anyhow::bail!(
                        "ACP agent not found on PATH: {}",
                        acp_spawn_command(&acp_settings)
                    );
                }
                None
            }
            #[cfg(not(feature = "opencode"))]
            {
                let _ = (&paths, &model_config);
                anyhow::bail!("ACP Ask mode requires opencode feature");
            }
        };

        Ok(Self {
            paths,
            model_config,
            acp_settings,
            native,
        })
    }

    pub fn acp_settings(&self) -> &AcpSettings {
        &self.acp_settings
    }

    fn native(&self) -> Result<&NativeBackend> {
        self.native
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("native chat backend not initialized"))
    }
}

fn build_native_backend(
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

impl ChatEngine {
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

    pub fn model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    pub async fn ask(
        &self,
        session_id: &str,
        query: &str,
        project: Option<&str>,
        repo_path: Option<&str>,
        on_chunk: impl FnMut(&str),
        on_tool_calls: impl FnMut(&[ChatToolCallRecord]),
        on_phase: impl FnMut(ChatPhase),
        on_usage: impl FnMut(&ChatTokenUsage),
    ) -> Result<ChatReply> {
        if let Some(slug) = project {
            if !agent_pack_ready(&self.paths, slug)
                || !mind_mesh_core::agent_context_ready(&self.paths, slug)
            {
                prepare_agent_assets_for_ask(
                    &self.paths,
                    &self.model_config,
                    slug,
                    repo_path.filter(|r| !r.is_empty()),
                )
                .await?;
            }
        }

        self.run_turn(
            session_id,
            query,
            project,
            repo_path,
            on_chunk,
            on_tool_calls,
            on_phase,
            on_usage,
        )
        .await
    }

    /// Run one agent turn without preparing agent assets (used by context generation).
    pub(crate) async fn run_turn(
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
        if self.acp_settings.ask_execution == AskExecution::Acp
            && !session_id.starts_with("agent-ctx-")
        {
            return self
                .run_turn_acp(
                    session_id,
                    query,
                    project,
                    repo_path,
                    on_chunk,
                    on_tool_calls,
                    on_phase,
                    on_usage,
                )
                .await;
        }

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

    #[cfg(feature = "opencode")]
    async fn run_turn_acp(
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
        use adk_acp::prompt_agent;
        use std::collections::HashMap;

        let prompt = build_ask_acp_prompt(query, project, &self.paths)?;
        let prompt_chars = prompt.len();

        on_phase(ChatPhase::Thinking);
        on_phase(ChatPhase::Tools);

        let started = now_ms();
        let spawn = acp_spawn_command(&self.acp_settings);
        let acp_record = ChatToolCallRecord {
            id: format!("acp-{session_id}"),
            name: "acp_agent".into(),
            arguments: serde_json::json!({ "command": spawn, "session_id": session_id }),
            result: None,
            error: None,
            status: ChatToolCallStatus::Running,
            started_at: started,
            completed_at: None,
            duration_ms: None,
        };
        on_tool_calls(&[acp_record.clone()]);

        let repo = repo_path
            .filter(|r| !r.is_empty())
            .map(str::to_string)
            .or_else(|| {
                project.and_then(|slug| resolve_project_repo_path(&self.paths, slug, None).ok())
            });

        let skill_dir = default_ask_acp_skill_dir();
        let mut env = HashMap::new();
        env.insert(
            "MIND_MESH_ASK_SKILL".into(),
            skill_dir.display().to_string(),
        );
        env.insert(
            "MIND_MESH_KNOWLEDGE_ROOT".into(),
            self.paths
                .knowledge_root_for(project)
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        if let Some(slug) = project {
            env.insert("MIND_MESH_PROJECT_SLUG".into(), slug.to_string());
        }
        if let Some(ref r) = repo {
            env.insert("MIND_MESH_REPO_PATH".into(), r.clone());
        }

        let config = build_acp_config(&self.acp_settings, repo.as_deref(), env);

        let response = prompt_agent(&config, &prompt)
            .await
            .map_err(|e| anyhow::anyhow!("ACP Ask failed: {e}"))?;

        let completed = now_ms();
        let mut done_record = acp_record;
        done_record.result = Some(serde_json::json!({
            "mode": "acp",
            "response_chars": response.len(),
        }));
        done_record.status = ChatToolCallStatus::Ok;
        done_record.completed_at = Some(completed);
        done_record.duration_ms = Some(completed.saturating_sub(started));
        on_tool_calls(&[done_record.clone()]);

        on_phase(ChatPhase::Generating);
        on_phase(ChatPhase::Streaming);

        for chunk in response.chars().collect::<Vec<_>>().chunks(48) {
            let s: String = chunk.iter().collect();
            on_chunk(&s);
        }

        let raw_answer = response.clone();
        let answer = sanitize_answer_text(&response);
        self.paths.write_debug_file("last-ask-raw.md", &raw_answer);
        self.paths
            .write_debug_file("last-ask-sanitized.md", &answer);

        let mut usage = ChatTokenUsage::default();
        finalize_usage(&mut usage, prompt_chars, &answer);
        on_usage(&usage);

        let mut citations = self
            .search_citations(query, project, repo_path)
            .unwrap_or_default();
        citations = merge_citations(
            citations,
            extract_source_citations(&answer, repo_path.as_deref()),
        );

        let answer = if answer.trim().is_empty() {
            "ACP agent returned empty text.".into()
        } else {
            answer
        };

        Ok(ChatReply {
            answer,
            citations,
            tool_calls: vec![done_record],
            usage,
            completed_at: now_ms(),
        })
    }

    fn search_citations(
        &self,
        query: &str,
        project: Option<&str>,
        repo_path: Option<&str>,
    ) -> Result<Vec<SourceCitation>> {
        let hits = KnowledgeSearch::new(&self.paths).search(
            query,
            SearchOptions {
                project: project.map(str::to_string),
                doc_type: None,
                limit: 5,
            },
        )?;

        Ok(hits
            .iter()
            .map(|h| SourceCitation {
                kind: if h.path.contains("/human/") {
                    mind_mesh_core::CitationKind::HumanDoc
                } else {
                    mind_mesh_core::CitationKind::StructuredDoc
                },
                title: h.title.clone().unwrap_or_else(|| h.path.clone()),
                path: h.path.clone(),
                repo_path: repo_path.map(str::to_string),
                start_line: None,
                end_line: None,
                excerpt: Some(h.snippet.clone()),
            })
            .collect())
    }
}

fn build_ask_acp_prompt(
    query: &str,
    project: Option<&str>,
    paths: &KnowledgePaths,
) -> Result<String> {
    let skill_dir = default_ask_acp_skill_dir();
    let skill_dir_s = skill_dir.display().to_string();
    let knowledge_root = paths
        .knowledge_root_for(project)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let base = build_ask_prompt(query, project, paths)?;

    Ok(format!(
        "You are MindMesh Ask running in **ACP mode**. Native function tools are NOT available.\n\
         Read the skill at `{skill_dir_s}/SKILL.md` and use the **`mind-mesh tools`** CLI \
         (subprocess) to query knowledge — same capabilities as native tools.\n\n\
         Environment:\n\
         - MIND_MESH_KNOWLEDGE_ROOT={knowledge_root}\n\
         - MIND_MESH_ASK_SKILL={skill_dir_s}\n\
         - Project slug: {}\n\n\
         Workflow: macro context below → `mind-mesh tools read-context` / `grep-pack` / \
         `read-pack-file` as needed → answer with citations (paths:line).\n\n\
         {base}",
        project.unwrap_or("(none — pass --project to tools commands)"),
    ))
}

fn build_ask_prompt(query: &str, project: Option<&str>, paths: &KnowledgePaths) -> Result<String> {
    let Some(slug) = project else {
        return Ok(query.to_string());
    };

    let repo_path = resolve_project_repo_path(paths, slug, None).unwrap_or_default();
    let pack_meta = read_json::<AgentPackMeta>(paths.agent_pack_meta(slug)).ok();

    let mut sections = vec![format!(
        "Current project slug: {slug}\n\
Repository path (citations / UI only): {repo_path}\n\n\
TOOL RULES — three layers:\n\
• Macro: architecture overview preloaded below (do NOT call read_agent_context without section)\n\
• Meso: read_agent_context(section=\"…\") for a specific heading when needed\n\
• Micro: grep_agent_pack → read_agent_pack_file for source code\n\
Do NOT call read_agent_pack_meta when pack metadata is preloaded."
    )];

    if agent_pack_ready(paths, slug) {
        if let Some(meta) = pack_meta.as_ref() {
            let top_files = meta
                .top_files_by_tokens
                .iter()
                .take(8)
                .map(|f| format!("  - {} ({} tokens)", f.path, f.tokens))
                .collect::<Vec<_>>()
                .join("\n");
            let (dir_preview, dir_truncated) =
                truncate_with_notice(&meta.directory_structure, ASK_INJECT_DIR_TREE_MAX_CHARS);
            sections.push(format!(
                "## Preloaded repomix pack metadata (do NOT call read_agent_pack_meta)\n\
synced_at: {}\nfiles: {}\ntokens: {}\nstrategy: {}\n\n\
Top files by tokens:\n{}\n\n\
Directory tree (preview{}):\n{}\n",
                meta.synced_at,
                meta.total_files,
                meta.total_tokens,
                meta.pack_strategy,
                if top_files.is_empty() {
                    "  (none)".into()
                } else {
                    top_files
                },
                if dir_truncated { ", truncated" } else { "" },
                dir_preview,
            ));
        }
    } else {
        sections.push(
            "## Repomix pack: NOT READY\n\
Assets will be auto-generated; you may call read_agent_context once if needed.\n"
                .to_string(),
        );
    }

    let ctx = read_agent_context_status(paths, slug);
    if ctx.ready {
        let context_path = paths.agent_context_main(slug);
        let body = mind_mesh_core::read_doc(&context_path)
            .map(|d| d.body)
            .unwrap_or_default();
        let overview = build_context_overview(&body, AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS);
        sections.push(format!(
            "## Preloaded architecture context (macro layer)\n\
Total stored: {} chars in {} sections{}. Do NOT re-fetch overview.\n\n\
{}\n",
            overview.total_chars,
            overview.section_titles.len(),
            if overview.size_capped {
                " (overview capped)"
            } else {
                ""
            },
            overview.macro_markdown,
        ));
    } else {
        sections.push(
            "## Architecture context: NOT READY\n\
Call read_agent_context (no section) once to load or auto-generate context.md.\n"
                .to_string(),
        );
    }

    sections.push(format!(
        "## Source code (micro layer)\n\
grep_agent_pack → read_agent_pack_file (≤150 lines, pass start_line/end_line). \
Do not read the live repository.\n\n\
## User question\n{query}"
    ));

    Ok(sections.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use adk_core::{Content, Event};

    fn function_call_event(partial: bool, name: &str, args: serde_json::Value) -> Event {
        let mut event = Event::new("inv-1");
        event.llm_response.partial = partial;
        event.llm_response.content = Some(Content {
            role: "model".into(),
            parts: vec![Part::FunctionCall {
                name: name.into(),
                args,
                id: None,
                thought_signature: None,
            }],
        });
        event
    }

    fn function_response_event(name: &str, response: serde_json::Value) -> Event {
        let mut event = Event::new("inv-1");
        event.llm_response.content = Some(Content {
            role: "function".into(),
            parts: vec![Part::FunctionResponse {
                function_response: adk_core::FunctionResponseData::new(name, response),
                id: None,
            }],
        });
        event
    }

    #[test]
    fn dedupes_partial_function_calls() {
        let mut tracker = ToolCallTracker::new();
        assert!(tracker.ingest_event(&function_call_event(
            true,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "fo" }),
        )));
        assert!(tracker.ingest_event(&function_call_event(
            true,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "foo" }),
        )));
        assert_eq!(tracker.records().len(), 1);
        assert_eq!(
            tracker.records()[0].arguments,
            serde_json::json!({ "pattern": "foo" })
        );
    }

    #[test]
    fn matches_response_to_running_call_by_name() {
        let mut tracker = ToolCallTracker::new();
        tracker.ingest_event(&function_call_event(
            false,
            "read_agent_pack_meta",
            serde_json::json!({ "project": "demo" }),
        ));
        tracker.ingest_event(&function_response_event(
            "read_agent_pack_meta",
            serde_json::json!({ "meta": { "total_tokens": 1 } }),
        ));
        assert!(matches!(
            tracker.records()[0].status,
            ChatToolCallStatus::Ok
        ));
        assert!(tracker.records()[0].result.is_some());
    }

    #[test]
    fn all_done_after_each_tool_batch() {
        let mut tracker = ToolCallTracker::new();
        tracker.ingest_event(&function_call_event(
            false,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "a" }),
        ));
        assert!(tracker.has_running());
        assert!(!tracker.all_done());

        tracker.ingest_event(&function_response_event(
            "grep_agent_pack",
            serde_json::json!({ "matches": [] }),
        ));
        assert!(!tracker.has_running());
        assert!(tracker.all_done());

        // Second tool round
        tracker.ingest_event(&function_call_event(
            false,
            "read_agent_pack_file",
            serde_json::json!({ "file_path": "src/main.rs" }),
        ));
        assert!(tracker.has_running());
        assert!(!tracker.all_done());

        tracker.ingest_event(&function_response_event(
            "read_agent_pack_file",
            serde_json::json!({ "content": "fn main() {}" }),
        ));
        assert!(tracker.all_done());
        assert_eq!(tracker.records().len(), 2);
    }
}
