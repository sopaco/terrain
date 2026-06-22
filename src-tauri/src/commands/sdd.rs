use terrain_agent::{execution_pure_acp, run_sdd_phase, validate_repo_path, SddProgress};
use terrain_core::{
    create_sdd_session, delete_sdd_session, get_sdd_status, resolve_sdd_session_id,
    save_sdd_output, set_active_sdd_session, SddPhase, SddPhaseResult, SddSessionInfo, SddStatus,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{SddDonePayload, SddProgressPayload};
use super::{resolved_acp_settings, slugify_repo};

#[tauri::command]
pub fn get_sdd_status_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<SddStatus, String> {
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn create_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    title: String,
) -> Result<SddSessionInfo, String> {
    create_sdd_session(&state.paths, &project_slug, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    set_active_sdd_session(&state.paths, &project_slug, &session_id).map_err(|e| e.to_string())?;
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn delete_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    delete_sdd_session(&state.paths, &project_slug, &session_id).map_err(|e| e.to_string())?;
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn save_sdd_output_cmd(
    state: State<'_, AppState>,
    output_path: String,
    content: String,
) -> Result<(), String> {
    save_sdd_output(&state.paths, &output_path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_sdd_phase_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
    session_id: Option<String>,
    phase: SddPhase,
    user_input: Option<String>,
) -> Result<SddPhaseResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let session_id = session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| resolve_sdd_session_id(&paths, &slug));
    let input = user_input.unwrap_or_default();

    let acp = resolved_acp_settings();
    let engine = if execution_pure_acp(&acp) || phase == SddPhase::CodeGen {
        None
    } else {
        Some(state.chat_engine().await.map_err(|e| e.to_string())?)
    };

    let emit_progress = |p: SddProgress| {
        let _ = app.emit(
            "sdd-progress",
            SddProgressPayload {
                project_slug: slug.clone(),
                phase,
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let result = run_sdd_phase(
        &paths,
        engine,
        &slug,
        &repo_path,
        &session_id,
        phase,
        &input,
        &acp,
        emit_progress,
    )
    .await
    .map_err(|e| e.to_string())?;

    app.emit(
        "sdd-done",
        SddDonePayload {
            project_slug: slug,
            result: result.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}
