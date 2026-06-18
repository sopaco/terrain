//! YAML frontmatter schema for knowledge documents.
//!
//! See `idea.md` §4.2 for the full convention.

use serde::{Deserialize, Serialize};

/// Top-level document kinds stored under `{repo}/.mind-mesh/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Project,
    Module,
    Interface,
    Route,
    Event,
    /// Litho-generated narrative docs under `human/`.
    Human,
    /// Agent architecture narrative under `agent/context.md`.
    #[serde(rename = "agent_context")]
    AgentContext,
}

impl DocType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Module => "module",
            Self::Interface => "interface",
            Self::Route => "route",
            Self::Event => "event",
            Self::Human => "human",
            Self::AgentContext => "agent_context",
        }
    }

    pub fn subdir(self) -> Option<&'static str> {
        match self {
            Self::Project => None,
            Self::Module => Some("modules"),
            Self::Interface => Some("interfaces"),
            Self::Route => Some("routes"),
            Self::Event => Some("events"),
            Self::Human => Some("human"),
            Self::AgentContext => None,
        }
    }
}

/// Shared frontmatter fields across all knowledge documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFrontmatter {
    #[serde(rename = "type")]
    pub doc_type: DocType,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub repo_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tech_stack: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceMeta {
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMeta {
    pub uri: String,
    pub handler: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub middleware: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeta {
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMeta {
    pub project: String,
    pub repo_path: String,
    pub synced_at: String,
    pub collectors: Vec<String>,
}

/// Knowledge asset track — who primarily consumes the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetTrack {
    /// Litho C4 docs for humans (`human/`).
    Human,
    /// Repomix pack for agents (`agent/`).
    Agent,
    /// OpenAPI-derived structured docs (`interfaces/`, `routes/`). Module maps come from developer meta + Agent context.
    Structured,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHeavyFile {
    pub path: String,
    pub tokens: usize,
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithoPlan {
    pub project_slug: String,
    pub repo_path: String,
    pub skill_dir: String,
    pub human_output_dir: String,
    pub litho_workspace_dir: String,
    pub skill_ready: bool,
}

/// Citation attached to a DeepWiki-style Q&A reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CitationKind {
    HumanDoc,
    StructuredDoc,
    SourceCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCitation {
    pub kind: CitationKind,
    pub title: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDocEntry {
    pub path: String,
    pub title: String,
    pub relative_path: String,
    /// Tree section: `human` (Litho docs) or `agent` (architecture context, etc.).
    #[serde(default = "default_human_section")]
    pub section: String,
}

fn default_human_section() -> String {
    "human".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSlice {
    pub repo_path: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocCounts {
    pub human: usize,
    pub interfaces: usize,
    pub routes: usize,
    pub modules: usize,
    pub events: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithoStatus {
    pub human_doc_count: usize,
    pub has_human_docs: bool,
    pub human_docs_complete: bool,
    pub has_research_artifacts: bool,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEnvStatus {
    pub ready: bool,
    pub integrated_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub detail: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessDriftFactor {
    pub id: String,
    /// `high` | `medium` | `low` | `info`
    pub severity: String,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points_lost: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessSummary {
    pub overall_score: u8,
    pub overall_stale: bool,
    pub commits_since_baseline: u32,
    pub changed_files_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_git_head: Option<String>,
    pub working_tree_dirty: bool,
    pub is_git_repo: bool,
    pub last_computed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
    pub agent_pack_score: u8,
    pub agent_context_score: u8,
    pub human_docs_score: u8,
    pub macro_preload_allowed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drift_factors: Vec<FreshnessDriftFactor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sample_changed_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_baseline_short: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_baseline_short: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessBaseline {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    pub git_head_at: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFreshness {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_git_head: Option<String>,
    pub stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_reason: Option<String>,
    pub freshness_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessAssets {
    pub agent_pack: AssetFreshness,
    pub agent_context: AssetFreshness,
    pub human_docs: AssetFreshness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessDrift {
    pub commits_since_baseline: u32,
    pub changed_files_since_baseline: u32,
    pub sample_changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessLedger {
    pub version: u32,
    pub project: String,
    pub repo_path: String,
    pub baseline: FreshnessBaseline,
    pub assets: FreshnessAssets,
    pub drift: FreshnessDrift,
    pub summary: FreshnessSummary,
    pub last_computed_at: String,
}

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
    /// Human-editable remark from `.mind-mesh/project-note.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_remark: Option<String>,
}

/// SDD standardized workflow phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SddPhase {
    Requirements,
    TechDesign,
    CodeGen,
    CodeReview,
}

impl SddPhase {
    pub fn all() -> [Self; 4] {
        [
            Self::Requirements,
            Self::TechDesign,
            Self::CodeGen,
            Self::CodeReview,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Requirements => "需求澄清",
            Self::TechDesign => "技术方案",
            Self::CodeGen => "代码生成",
            Self::CodeReview => "Code Review",
        }
    }

    pub fn output_filename(self) -> &'static str {
        match self {
            Self::Requirements => "1.requirements.md",
            Self::TechDesign => "2.tech-design.md",
            Self::CodeGen => "3.implementation.md",
            Self::CodeReview => "4.code-review.md",
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Self::Requirements => 0,
            Self::TechDesign => 1,
            Self::CodeGen => 2,
            Self::CodeReview => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddSessionInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPlan {
    pub project_slug: String,
    pub session_id: String,
    pub repo_path: String,
    pub skill_dir: String,
    pub sdd_workspace_dir: String,
    pub sdd_output_dir: String,
    pub human_output_dir: String,
    pub agent_pack_path: String,
    pub skill_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPhaseInfo {
    pub phase: SddPhase,
    pub label: String,
    pub output_path: String,
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddStatus {
    pub project_slug: String,
    pub skill_ready: bool,
    pub workspace_dir: String,
    pub output_dir: String,
    pub phases: Vec<SddPhaseInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<SddPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    pub sessions: Vec<SddSessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPhaseResult {
    pub phase: SddPhase,
    pub output_path: String,
    pub response_excerpt: String,
}
