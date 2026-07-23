use terrain_agent::{
    ask_knowledge, execution_pure_acp, run_sdd_phase, AskStreamEvent, ChatReply,
};
use terrain_core::{
    ipc_string, resolve_sdd_session_id, SddPhase, SddPhaseResult, SddStatus,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{
    ChatChunkPayload, ChatDonePayload, ChatPhasePayload, ChatToolCallsPayload, ChatUsagePayload,
    SddDonePayload, SddProgressPayload,
};
use super::util::validate_repo;
use super::{resolved_acp_settings, slugify_repo};

#[tauri::command]
pub async fn ask_knowledge_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    repo_path: Option<String>,
    request_id: Option<String>,
) -> Result<ChatReply, String> {
    let stream_id = request_id.unwrap_or_else(|| format!("ask-{}", query.len()));
    let app_emit = app.clone();
    let sid = stream_id.clone();

    let on_event = move |event: AskStreamEvent| match event {
        AskStreamEvent::Chunk { text } => {
            let _ = app_emit.emit(
                "chat-chunk",
                ChatChunkPayload {
                    session_id: sid.clone(),
                    text,
                },
            );
        }
        AskStreamEvent::ToolCalls { tool_calls } => {
            let _ = app_emit.emit(
                "chat-tool-calls",
                ChatToolCallsPayload {
                    session_id: sid.clone(),
                    tool_calls,
                },
            );
        }
        AskStreamEvent::Phase { phase } => {
            let _ = app_emit.emit(
                "chat-phase",
                ChatPhasePayload {
                    session_id: sid.clone(),
                    phase,
                },
            );
        }
        AskStreamEvent::Usage { usage } => {
            let _ = app_emit.emit(
                "chat-usage",
                ChatUsagePayload {
                    session_id: sid.clone(),
                    usage,
                },
            );
        }
        AskStreamEvent::Done { reply } => {
            let _ = app_emit.emit(
                "chat-done",
                ChatDonePayload {
                    session_id: sid.clone(),
                    answer: reply.answer.clone(),
                    citations: reply.citations.clone(),
                    tool_calls: reply.tool_calls.clone(),
                    usage: reply.usage.clone(),
                    completed_at: reply.completed_at,
                },
            );
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
    validate_repo(&repo_path).map_err(ipc_string)?;
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
