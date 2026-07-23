//! Knowledge asset track and pack metadata.

use serde::{Deserialize, Serialize};

/// Knowledge asset track — who primarily consumes the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetTrack {
    /// Litho C4 docs for humans (`human/`).
    Human,
    /// Repomix pack for agents (`agent/`).
    Agent,
    /// OpenAPI-derived structured docs (`interfaces/`, `routes/`).
    Structured,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "kebab-case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetGenerator {
    LithoSkill,
    RepomixCore,
    GitScanner,
    OpenApiImporter,
}

fn default_pack_strategy() -> String {
    "architecture-context".into()
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHeavyFile {
    pub path: String,
    pub tokens: usize,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPackMeta {
    pub project: String,
    pub repo_path: String,
    pub generator: AssetGenerator,
    /// e.g. `architecture-context` — see docs/schema.md
    #[serde(default = "default_pack_strategy")]
    pub pack_strategy: String,
    pub output_file: String,
    pub total_files: usize,
    pub total_tokens: usize,
    pub total_characters: usize,
    pub top_files_by_tokens: Vec<TokenHeavyFile>,
    pub directory_structure: String,
    pub synced_at: String,
    /// Git HEAD at pack time — used for freshness drift detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_git_head: Option<String>,
}

/// Plan for Agent + Litho skill to generate human-facing docs.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithoPlan {
    pub project_slug: String,
    pub repo_path: String,
    pub skill_dir: String,
    pub human_output_dir: String,
    pub litho_workspace_dir: String,
    pub skill_ready: bool,
}
