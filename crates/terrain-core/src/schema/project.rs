//! Project overview and asset health types.

use serde::{Deserialize, Serialize};

use super::asset::AgentPackMeta;
use super::freshness::FreshnessSummary;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocCounts {
    pub human: usize,
    pub interfaces: usize,
    pub routes: usize,
    pub modules: usize,
    pub events: usize,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithoStatus {
    pub human_doc_count: usize,
    pub has_human_docs: bool,
    pub human_docs_complete: bool,
    pub has_research_artifacts: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextMeta {
    pub project: String,
    pub repo_path: String,
    pub output_file: String,
    pub generated_at: String,
    pub section_count: usize,
    pub char_count: usize,
    /// Git HEAD when context was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_git_head: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextStatus {
    pub ready: bool,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    pub section_count: usize,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEnvStatus {
    pub ready: bool,
    pub integrated_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub detail: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTrackHealth {
    pub track: String,
    pub label: String,
    pub ready: bool,
    pub summary: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness_score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectOverview {
    pub slug: String,
    pub name: String,
    pub repo_path: String,
    pub tech_stack: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    pub collectors: Vec<String>,
    pub doc_counts: DocCounts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_pack: Option<AgentPackMeta>,
    pub litho: LithoStatus,
    pub agent_context: AgentContextStatus,
    pub asset_health: Vec<AssetTrackHealth>,
    pub agent_env: AgentEnvStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structure_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<FreshnessSummary>,
    /// Human-editable remark from `.terrain/project-note.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_remark: Option<String>,
}
