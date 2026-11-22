//! Dashboard commands — fleet metrics, batch updates, multi-project registration.
//!
//! Thin Tauri wrappers over the `terrain-dashboard` crate; batch progress is
//! emitted as its `BatchProgressEvent` payload on `dashboard-batch-progress`.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use terrain_dashboard::{
    AddProjectsResult, BatchUpdateResult, DashboardSummary, add_projects, batch_update_graphs,
    scan_dashboard,
};

use crate::AppState;

#[tauri::command]
pub async fn get_dashboard_summary_cmd(
    state: State<'_, AppState>,
) -> Result<DashboardSummary, String> {
    let paths = state.paths().clone();
    tauri::async_runtime::spawn_blocking(move || scan_dashboard(&paths, None))
        .await
        .map_err(|e| format!("dashboard scan failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_dashboard_batch_cmd(
    app: AppHandle,
    project_slugs: Option<Vec<String>>,
    concurrency: Option<usize>,
) -> Result<BatchUpdateResult, String> {
    let app_for_cb = app.clone();
    let on_progress: Arc<dyn Fn(terrain_dashboard::BatchProgressEvent) + Send + Sync> =
        Arc::new(move |event| {
            let _ = app_for_cb.emit("dashboard-batch-progress", event);
        });

    let result = batch_update_graphs(project_slugs.unwrap_or_default(), concurrency, on_progress)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit("dashboard-batch-done", &result);
    Ok(result)
}

#[tauri::command]
pub async fn add_projects_cmd(
    state: State<'_, AppState>,
    repo_paths: Vec<String>,
) -> Result<AddProjectsResult, String> {
    let paths = state.paths().clone();
    add_projects(&paths, repo_paths)
        .await
        .map_err(|e| e.to_string())
}
