use terrain_agent::{ask_knowledge, execution_pure_acp, run_sdd_phase, ChatReply};
use terrain_core::{
    ipc_string, resolve_sdd_session_id, AskStreamEvent, SddPhase, SddPhaseResult, SddStatus,
};
use tauri::{ipc::Channel, AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{SddDonePayload, SddProgressPayload};
use super::{resolved_acp_settings, slugify_repo};

#[tauri::command]
pub async fn ask_knowledge_cmd(
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    repo_path: Option<String>,
    on_stream: Channel<AskStreamEvent>,
) -> Result<ChatReply, String> {
    let stream_id = format!(
        "ask-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let on_event = move |event: AskStreamEvent| {
        if let Err(err) = on_stream.send(event) {
            tracing::warn!(error = %err, "ask stream channel send failed");
        }
    };

    ask_knowledge(
        &state.runtime,
        &stream_id,
        &query,
        project.as_deref(),
        repo_path.as_deref(),
        on_event,
    )
    .await
    .map_err(ipc_string)
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
    super::util::validate_repo(&repo_path).map_err(ipc_string)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths();
    let session_id = session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| resolve_sdd_session_id(paths, &slug));
    let input = user_input.unwrap_or_default();

    let acp = resolved_acp_settings();
    let engine = if execution_pure_acp(&acp) || phase == SddPhase::CodeGen {
        None
    } else {
        Some(
            state
                .runtime
                .chat_engine()
                .map_err(ipc_string)?,
        )
    };

    let app_emit = app.clone();
    let slug_emit = slug.clone();
    let result = run_sdd_phase(
        paths,
        engine,
        &slug,
        &repo_path,
        &session_id,
        phase,
        &input,
        &acp,
        move |p| {
            let _ = app_emit.emit(
                "sdd-progress",
                SddProgressPayload {
                    project_slug: slug_emit.clone(),
                    phase,
                    stage: p.stage.clone(),
                    message: p.message.clone(),
                },
            );
        },
    )
    .await
    .map_err(ipc_string)?;

    app.emit(
        "sdd-done",
        SddDonePayload {
            project_slug: slug,
            result: result.clone(),
        },
    )
    .map_err(ipc_string)?;

    Ok(result)
}

#[tauri::command]
pub fn get_sdd_status_cmd(state: State<'_, AppState>, project_slug: String) -> Result<SddStatus, String> {
    Ok(terrain_core::get_sdd_status(state.paths(), &project_slug))
}
