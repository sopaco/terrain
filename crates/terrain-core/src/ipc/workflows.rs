use serde::Serialize;

use crate::registry::ProjectRegistryEntry;
use crate::schema::{AgentContextMeta, LithoPlan};
use crate::settings::ModelSettings;

/// Single IPC payload for app startup — avoids multiple round-trips on first paint.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct AppBootstrap {
    pub model_settings: ModelSettings,
    pub registry_projects: Vec<ProjectRegistryEntry>,
    pub llm_status: LlmStatus,
    pub acp_ok: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct LlmStatus {
    pub provider: String,
    pub model: String,
    pub ready: bool,
    pub message: String,
    pub base_url: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct LithoGenerationJob {
    pub plan: LithoPlan,
    pub prompt: String,
    pub acp_command: String,
    pub status: String,
}

/// What a knowledge refresh actually did — surfaced so the UI can say so instead of guessing.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRefreshMode {
    /// Regenerated from scratch.
    Full,
    /// Existing asset updated from a Git diff.
    Incremental,
    /// Already in sync — no model call was made.
    Skipped,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct LithoGenerationResult {
    pub plan: LithoPlan,
    pub response_excerpt: String,
    pub human_doc_count: usize,
    pub human_docs_complete: bool,
    pub refresh_mode: KnowledgeRefreshMode,
    /// Why the run took the mode it did, when that is not obvious (e.g. `too_many_changed_files`).
    pub refresh_reason: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInitResult {
    pub project_slug: String,
    pub repo_path: String,
    pub scan_files_written: usize,
    pub repack_tokens: Option<usize>,
    pub agent_context_generated: bool,
    pub human_doc_count: usize,
    pub human_docs_complete: bool,
    pub litho_ran: bool,
    pub notes: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct AgentContextGenerationResult {
    pub output_path: String,
    pub meta: AgentContextMeta,
    pub response_excerpt: String,
    pub refresh_mode: KnowledgeRefreshMode,
}
