use std::sync::Arc;

use terrain_agent::{
    agent_execution_ready, execution_pure_acp, execution_uses_native_llm, run_agent_context_generation,
    run_project_initialization, validate_repo_path, ChatEngine, LithoGenerationResult,
    LithoProgress, ProjectInitProgress,
};
use terrain_core::{
    agent_context_fresh, agent_context_ready, compute_freshness, get_project_overview, list_stale_registry_projects,
    plan_litho_generation, read_freshness_ledger, write_project_remark,
    FreshnessSummary, ProjectOverview, ProjectScanner, ProjectSummary, QuickRefreshResult,
    ScanReport, StaleProjectSummary,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{
    LithoDonePayload, LithoProgressPayload, ProjectInitDonePayload, ProjectInitProgressPayload,
};
use super::{resolved_acp_settings, slugify_repo};

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, String> {
    terrain_core::KnowledgeSearch::new(&state.paths)
        .list_projects()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_project(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<ScanReport, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let scanner = ProjectScanner::new(state.paths.clone());
    scanner
        .scan_repo(&repo_path, project_slug.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_stale_projects_cmd() -> Result<Vec<StaleProjectSummary>, String> {
    list_stale_registry_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn initialize_project_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<terrain_agent::ProjectInitResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let model_config = state.get_model_config();
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
                stage: p.stage,
                message,
            },
        );
        let _ = app.emit(
            "project-init-progress",
            ProjectInitProgressPayload {
                project_slug: slug.clone(),
                stage: "human_docs".into(),
                message: p.message,
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
    get_project_overview(&state.paths, &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project_remark_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    remark: String,
) -> Result<ProjectOverview, String> {
    write_project_remark(&state.paths, &project_slug, &remark).map_err(|e| e.to_string())?;
    get_project_overview(&state.paths, &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compute_freshness_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    repo_path: Option<String>,
) -> Result<FreshnessSummary, String> {
    let repo = terrain_core::resolve_project_repo_path(
        &state.paths,
        &project_slug,
        repo_path.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    compute_freshness(&state.paths, &project_slug, &repo).map_err(|e| e.to_string())
}

/// Last persisted freshness ledger (no git recompute) — for instant overview paint.
#[tauri::command]
pub fn read_project_freshness_cached_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Option<FreshnessSummary> {
    read_freshness_ledger(&state.paths, &project_slug).map(|ledger| ledger.summary)
}

/// Scan + repack + optional agent context regeneration (skips Litho).
///
/// Repomix packing runs once inside [`ProjectScanner::scan_repo`]; do not call
/// [`terrain_core::pack_agent_assets`] again here.
#[tauri::command]
pub async fn run_quick_refresh_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<QuickRefreshResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let mut notes = Vec::new();

    let scanner = ProjectScanner::new(paths.clone());
    let scan = scanner
        .scan_repo(&repo_path, Some(&slug))
        .await
        .map_err(|e| e.to_string())?;

    let pack_tokens = scan.agent_pack.as_ref().map(|pack| pack.total_tokens);
    if scan.agent_pack.as_ref().is_some_and(|p| p.pack_skipped) {
        notes.push("源码索引：已与当前提交同步，已跳过".into());
    } else if pack_tokens.is_none() {
        notes.push(
            "源码索引：scan 未执行 repomix 打包（terrain-core 未启用 repomix feature）".into(),
        );
    }

    let mut agent_context_regenerated = false;
    let acp = resolved_acp_settings();
    let model_config = state.get_model_config();
    if agent_execution_ready(&acp, &model_config).is_ok() {
        if !agent_context_fresh(&paths, &slug, &repo_path) {
            let engine = if execution_uses_native_llm(&acp) {
                Some(Arc::new(
                    ChatEngine::new_native(paths.clone(), model_config).map_err(|e| e.to_string())?,
                ))
            } else {
                None
            };
            match run_agent_context_generation(&paths, engine, &acp, &slug, &repo_path).await {
                Ok(_) => agent_context_regenerated = true,
                Err(e) => notes.push(format!("Agent 知识资产：{e}")),
            }
        } else if agent_context_ready(&paths, &slug) {
            notes.push("Agent 友好的知识资产：已与当前提交同步，已跳过".into());
        }
    } else if execution_pure_acp(&acp) {
        notes.push("Agent 友好的知识资产：请先在设置中配置 ACP 代理".into());
    } else {
        notes.push("Agent 友好的知识资产：请配置 ACP 代理与 LLM".into());
    }

    let freshness = compute_freshness(&paths, &slug, &repo_path).map_err(|e| e.to_string())?;

    Ok(QuickRefreshResult {
        project_slug: slug,
        scan_files_written: scan.files_written,
        pack_tokens,
        agent_context_regenerated,
        notes,
        freshness,
    })
}
