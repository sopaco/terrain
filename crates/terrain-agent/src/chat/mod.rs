#[cfg(feature = "opencode")]
mod acp;
mod native;
mod prompt;
mod tracker;
mod types;

pub use types::{
    ChatPhase, ChatReply, ChatTokenUsage, ChatToolCallRecord, ChatToolCallStatus,
};

use std::time::Duration;

use anyhow::Result;
use terrain_core::{
    agent_context_fresh, agent_pack_fresh, normalize_repo_hint, resolve_project_repo_path,
    KnowledgePaths, KnowledgeSearch, SearchOptions, SourceCitation, prepare_chat_markdown,
};

use crate::acp::{
    acp_available, acp_spawn_command, execution_pure_acp, execution_uses_native_llm,
    resolve_acp_settings,
};
use crate::agent_assets::prepare_agent_assets_for_ask;
use crate::model::ModelConfig;
use crate::settings::{AcpSettings, AgentExecution};

use native::{build_native_backend, NativeBackend};

pub(crate) const CHAT_APP_NAME: &str = "terrain";
pub(crate) const CHAT_USER_ID: &str = "terrain-user";
pub(crate) const ASK_TIMEOUT: Duration = Duration::from_secs(1200);

pub(crate) fn finalize_usage(usage: &mut ChatTokenUsage, prompt_chars: usize, answer: &str) {
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

pub(crate) fn sanitize_answer_text(text: &str) -> String {
    prepare_chat_markdown(text)
}

pub struct ChatEngine {
    pub(crate) paths: KnowledgePaths,
    pub(crate) model_config: ModelConfig,
    pub(crate) acp_settings: AcpSettings,
    native: Option<NativeBackend>,
}

impl ChatEngine {
    pub fn new(paths: KnowledgePaths, model_config: ModelConfig) -> Result<Self> {
        Self::with_settings(paths, model_config, resolve_acp_settings())
    }

    /// Force native LLM backend for hybrid workloads (context generation, SDD doc phases).
    pub fn new_native(paths: KnowledgePaths, model_config: ModelConfig) -> Result<Self> {
        let mut acp = resolve_acp_settings();
        acp.agent_execution = AgentExecution::AcpNative;
        Self::with_settings(paths, model_config, acp)
    }

    pub fn with_settings(
        paths: KnowledgePaths,
        model_config: ModelConfig,
        acp_settings: AcpSettings,
    ) -> Result<Self> {
        let native = if execution_uses_native_llm(&acp_settings) {
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
            }
            #[cfg(not(feature = "opencode"))]
            {
                let _ = (&paths, &model_config);
                anyhow::bail!("ACP mode requires opencode feature");
            }
            None
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
            let repo = resolve_project_repo_path(
                &self.paths,
                slug,
                normalize_repo_hint(repo_path),
            )
            .unwrap_or_default();
            if !agent_pack_fresh(&self.paths, slug, &repo)
                || !agent_context_fresh(&self.paths, slug, &repo)
            {
                prepare_agent_assets_for_ask(
                    &self.paths,
                    &self.model_config,
                    slug,
                    normalize_repo_hint(repo_path),
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
        on_chunk: impl FnMut(&str),
        on_tool_calls: impl FnMut(&[ChatToolCallRecord]),
        on_phase: impl FnMut(ChatPhase),
        on_usage: impl FnMut(&ChatTokenUsage),
    ) -> Result<ChatReply> {
        if execution_pure_acp(&self.acp_settings) {
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

        self.run_turn_native(
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

    pub(crate) fn search_citations(
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
                    terrain_core::CitationKind::HumanDoc
                } else {
                    terrain_core::CitationKind::StructuredDoc
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
