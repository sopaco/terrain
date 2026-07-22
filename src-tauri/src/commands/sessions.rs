use terrain_core::{
    clear_active_ask_session, create_ask_session, create_sdd_session, delete_ask_session,
    delete_sdd_session, discard_ask_session, get_active_ask_session, get_sdd_status,
    list_ask_sessions, load_ask_messages, save_ask_messages, save_sdd_output,
    set_active_ask_session, set_active_sdd_session, AskSessionInfo, SddSessionInfo, SddStatus,
};
use terrain_core::ipc_string;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn list_ask_sessions_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<Vec<AskSessionInfo>, String> {
    Ok(list_ask_sessions(state.paths(), &project_slug))
}

#[tauri::command]
pub fn load_ask_messages_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    load_ask_messages(state.paths(), &project_slug, &session_id).map_err(ipc_string)
}

#[tauri::command]
pub fn save_ask_messages_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
    messages: serde_json::Value,
    first_question: Option<String>,
) -> Result<AskSessionInfo, String> {
    save_ask_messages(
        state.paths(),
        &project_slug,
        &session_id,
        &messages,
        first_question.as_deref(),
    )
    .map_err(ipc_string)
}

#[tauri::command]
pub fn create_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    question: String,
) -> Result<AskSessionInfo, String> {
    create_ask_session(state.paths(), &project_slug, &question).map_err(ipc_string)
}

#[tauri::command]
pub fn set_active_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<Vec<AskSessionInfo>, String> {
    set_active_ask_session(state.paths(), &project_slug, &session_id).map_err(ipc_string)?;
    Ok(list_ask_sessions(state.paths(), &project_slug))
}

#[tauri::command]
pub fn delete_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<Vec<AskSessionInfo>, String> {
    delete_ask_session(state.paths(), &project_slug, &session_id).map_err(ipc_string)?;
    Ok(list_ask_sessions(state.paths(), &project_slug))
}

#[tauri::command]
pub fn discard_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<Vec<AskSessionInfo>, String> {
    discard_ask_session(state.paths(), &project_slug, &session_id).map_err(ipc_string)?;
    Ok(list_ask_sessions(state.paths(), &project_slug))
}

#[tauri::command]
pub fn clear_active_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<(), String> {
    clear_active_ask_session(state.paths(), &project_slug).map_err(ipc_string)
}

#[tauri::command]
pub fn get_active_ask_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<Option<String>, String> {
    Ok(get_active_ask_session(state.paths(), &project_slug))
}

#[tauri::command]
pub fn create_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    title: String,
) -> Result<SddSessionInfo, String> {
    create_sdd_session(state.paths(), &project_slug, &title).map_err(ipc_string)
}

#[tauri::command]
pub fn set_active_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    set_active_sdd_session(state.paths(), &project_slug, &session_id).map_err(ipc_string)?;
    Ok(get_sdd_status(state.paths(), &project_slug))
}

#[tauri::command]
pub fn delete_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    delete_sdd_session(state.paths(), &project_slug, &session_id).map_err(ipc_string)?;
    Ok(get_sdd_status(state.paths(), &project_slug))
}

#[tauri::command]
pub fn save_sdd_output_cmd(
    state: State<'_, AppState>,
    output_path: String,
    content: String,
) -> Result<(), String> {
    save_sdd_output(state.paths(), &output_path, &content).map_err(ipc_string)
}
