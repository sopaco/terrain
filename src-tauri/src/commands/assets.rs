use std::sync::Arc;

use terrain_agent::{
    agent_execution_ready, execution_uses_native_llm, prepare_litho_generation,
    run_agent_context_generation, run_litho_generation, AgentContextGenerationResult, ChatEngine,
    LithoGenerationJob, LithoProgress, LithoRunMode,
};
use terrain_core::{
    build_generation_plan, pack_agent_assets, plan_litho_generation, AgentPackReport,
    AssetGenerationPlan, LithoPlan,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{LithoDonePayload, LithoProgressPayload};
use super::util::validate_repo;
use super::{resolved_acp_settings, resolved_knowledge_settings, slugify_repo};

#[tauri::command]
pub async fn pack_agent_assets_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<AgentPackReport, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let report = pack_agent_assets(state.paths(), &slug, &repo_path)
        .await
        .map_err(|e| e.to_string())?;
    let _ = terrain_core::compute_freshness(state.paths(), &slug, &repo_path);
    Ok(report)
}

#[tauri::command]
pub fn plan_litho_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<LithoPlan, String> {
    let path = validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(plan_litho_generation(
        state.paths(),
        &slug,
        &path,
    ))
}

#[tauri::command]
pub fn plan_assets_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<AssetGenerationPlan, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(build_generation_plan(state.paths(), &slug, &repo_path))
}

#[tauri::command]
pub fn generate_human_docs_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<LithoGenerationJob, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(prepare_litho_generation(
        state.paths(),
        &slug,
        &repo_path,
        &resolved_acp_settings(),
    ))
}

#[tauri::command]
pub async fn run_litho_generation_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
    force_refresh: Option<bool>,
) -> Result<(), String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths().clone();

    let emit_progress = |p: LithoProgress| {
        let _ = app.emit(
            "litho-progress",
            LithoProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let acp = resolved_acp_settings();
    // `force_refresh` is the UI's 「重新生成」 — an explicit rebuild bypasses incremental update.
    let result = run_litho_generation(
        &paths,
        &slug,
        &repo_path,
        &acp,
        &resolved_knowledge_settings(),
        LithoRunMode::from_force_refresh(force_refresh.unwrap_or(false)),
        emit_progress,
    )
    .await
    .map_err(|e| e.to_string())?;

    app.emit(
        "litho-done",
        LithoDonePayload {
            project_slug: slug,
            result,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn run_agent_context_generation_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
    force_full: Option<bool>,
) -> Result<AgentContextGenerationResult, String> {
    validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let acp = resolved_acp_settings();
    let model_config = state.model_config();
    agent_execution_ready(&acp, &model_config).map_err(|e| e.to_string())?;
    let engine = if execution_uses_native_llm(&acp) {
        Some(Arc::new(
            ChatEngine::new_native(state.paths().clone(), model_config).map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    run_agent_context_generation(
        state.paths(),
        engine,
        &acp,
        &slug,
        &repo_path,
        &resolved_knowledge_settings(),
        force_full.unwrap_or(false),
    )
    .await
    .map_err(|e| e.to_string())
}
