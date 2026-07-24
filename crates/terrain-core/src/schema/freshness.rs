//! Freshness ledger and summary types.

use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
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

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
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

/// Result of scan + repack + optional agent context regeneration (desktop quick refresh).
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickRefreshResult {
    pub project_slug: String,
    pub scan_files_written: usize,
    pub pack_tokens: Option<usize>,
    pub agent_context_regenerated: bool,
    pub notes: Vec<String>,
    pub freshness: FreshnessSummary,
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
