//! Terrain fleet dashboard — cross-project metrics, batch updates, registration.
//!
//! Pure Rust (no Tauri, no LLM): consumable from the desktop app, the CLI,
//! and tests. The Tauri command layer wraps this crate and maps its progress
//! callbacks into Tauri events.

pub mod dashboard;
pub mod graphify;
pub mod ua;

pub use dashboard::{
    AddProjectError, AddProjectOk, AddProjectsResult, BatchProgressEvent, BatchUpdateResult,
    CachedGraphInfo, DashboardRow, DashboardSummary, DashboardTotals, DocsCoverage,
    LastBatchRecord, ProjectActivity, ProjectCollaboration, ProjectUpdateResult, add_projects,
    batch_update_graphs, scan_dashboard, staleness_of, sync_vault_link, vault_link_current,
    vault_link_path, vault_root,
};
pub use graphify::{
    GraphifyAction, GraphifyCommunity, GraphifyGraphInfo, GraphifyRunOutcome, git_head,
    install_hint, read_graph_info, resolve_graphify_bin, run_graphify_action,
};
pub use ua::{
    UaAction, UaStatus, parse_viewer_banner, pick_viewer_port, port_alive, resolve_ua_repo,
    resolve_ua_viewer, run_ua_action, ua_status,
};
