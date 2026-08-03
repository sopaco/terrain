use terrain_agent::{
    run_project_initialization, run_quick_refresh, LithoGenerationResult, LithoProgress,
    ProjectInitProgress,
};
use terrain_core::{
    compute_freshness, get_project_overview, list_all_registry_projects, plan_litho_generation,
    read_freshness_ledger, write_project_remark, FreshnessSummary, ProjectOverview,
    ProjectRegistryEntry, ProjectSummary, QuickRefreshResult, ScanReport, ProjectScanner,
    ProjectInitResult,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{
    LithoDonePayload, LithoProgressPayload, ProjectInitDonePayload, ProjectInitProgressPayload,
};
use super::util::{map_core_err, validate_repo};
use super::{resolved_acp_settings, slugify_repo};

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, String> {
    terrain_core::KnowledgeSearch::new(state.paths())
        .list_projects()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_project(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<ScanReport, String> {
    validate_repo(&repo_path)?;
    let scanner = ProjectScanner::new(state.paths().clone());
    scanner
        .scan_repo(&repo_path, project_slug.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_registry_projects_cmd(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectRegistryEntry>, String> {
    list_all_registry_projects(state.paths()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn initialize_project_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<ProjectInitResult, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths().clone();
    let model_config = state.model_config();
    let acp = resolved_acp_settings();

    let emit_init = |p: ProjectInitProgress| {
        let _ = app.emit(
            "project-init-progress",
            ProjectInitProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let emit_litho = |p: LithoProgress| {
        let message = p.message.clone();
        let _ = app.emit(
            "litho-progress",
            LithoProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage.clone(),
                message: message.clone(),
            },
        );
        let _ = app.emit(
            "project-init-progress",
            ProjectInitProgressPayload {
                project_slug: slug.clone(),
                stage: "human_docs".into(),
                message,
            },
        );
    };

    let result = run_project_initialization(
        &paths,
        &model_config,
        &acp,
        &repo_path,
        Some(&slug),
        emit_init,
        emit_litho,
    )
    .await
    .map_err(|e| e.to_string())?;

    if result.litho_ran {
        let _ = app.emit(
            "litho-done",
            LithoDonePayload {
                project_slug: result.project_slug.clone(),
                result: LithoGenerationResult {
                    plan: plan_litho_generation(&paths, &result.project_slug, &result.repo_path),
                    response_excerpt: String::new(),
                    human_doc_count: result.human_doc_count,
                    human_docs_complete: result.human_docs_complete,
                },
            },
        );
    }

    app.emit(
        "project-init-done",
        ProjectInitDonePayload {
            project_slug: slug,
            result: result.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn remove_project_cmd(state: State<'_, AppState>, project_slug: String) -> Result<(), String> {
    terrain_core::unregister_project(&project_slug).map_err(|e| e.to_string())?;
    let _ = state;
    Ok(())
}

#[tauri::command]
pub fn get_project_overview_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<ProjectOverview, String> {
    get_project_overview(state.paths(), &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project_remark_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    remark: String,
) -> Result<ProjectOverview, String> {
    write_project_remark(state.paths(), &project_slug, &remark).map_err(|e| e.to_string())?;
    get_project_overview(state.paths(), &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compute_freshness_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    repo_path: Option<String>,
) -> Result<FreshnessSummary, String> {
    let repo = terrain_core::resolve_project_repo_path(
        state.paths(),
        &project_slug,
        repo_path.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    map_core_err(compute_freshness(state.paths(), &project_slug, &repo))
}

/// Last persisted freshness ledger (no git recompute) — for instant overview paint.
#[tauri::command]
pub fn read_project_freshness_cached_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Option<FreshnessSummary> {
    read_freshness_ledger(state.paths(), &project_slug).map(|ledger| ledger.summary)
}

/// Scan + repack + optional agent context regeneration (skips Litho).
#[tauri::command]
pub async fn run_quick_refresh_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<QuickRefreshResult, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let acp = resolved_acp_settings();
    let model_config = state.model_config();
    run_quick_refresh(
        state.paths(),
        &model_config,
        &acp,
        &repo_path,
        &slug,
    )
    .await
    .map_err(|e| e.to_string())
}
