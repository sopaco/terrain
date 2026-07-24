//! Env integration status types.

use serde::Serialize;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvStatus {
    pub repo_path: String,
    pub ready_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub items: Vec<EnvIntegrationStatus>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvIntegrationStatus {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub description: String,
    pub integrated: bool,
    pub optional: bool,
    pub bundled: bool,
    pub locked: bool,
    pub depends_on: Vec<String>,
    pub detail: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvPlan {
    pub repo_path: String,
    pub selected_ids: Vec<String>,
    pub steps: Vec<EnvPlanStep>,
    pub skipped: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvPlanStep {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub action: String,
}
