//! YAML frontmatter schema for knowledge documents.
//!
//! See `idea.md` §4.2 for the full convention.

mod asset;
mod citation;
mod doc;
mod freshness;
mod project;
mod sdd;

pub use asset::{AgentPackMeta, AssetGenerator, AssetTrack, LithoPlan, TokenHeavyFile};
pub use citation::{CitationKind, HumanDocEntry, SourceCitation, SourceSlice};
pub use doc::{
    DocFrontmatter, DocType, EventMeta, InterfaceMeta, ProjectMeta, RouteMeta, SyncMeta,
};
pub use freshness::{
    AssetFreshness, FreshnessAssets, FreshnessBaseline, FreshnessDrift, FreshnessDriftFactor,
    FreshnessLedger, FreshnessSummary, QuickRefreshResult,
};
pub use project::{
    AgentContextMeta, AgentContextStatus, AgentEnvStatus, AssetTrackHealth, DocCounts,
    LithoStatus, ProjectOverview,
};
pub use sdd::{
    AskSessionInfo, SddPhase, SddPhaseInfo, SddPhaseResult, SddPlan, SddSessionInfo, SddStatus,
};
