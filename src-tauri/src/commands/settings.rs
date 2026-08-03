use terrain_agent::{
    acp_available, acp_spawn_command, llm_status, load_model_settings, model_settings_from_config,
    resolve_model_config, save_model_settings, ModelSettings,
};
use terrain_core::{list_all_registry_projects, AppBootstrap};
use tauri::State;

use crate::AppState;

use super::resolved_acp_settings;

#[tauri::command]
pub fn bootstrap_app(state: State<'_, AppState>) -> AppBootstrap {
    let settings =
        load_model_settings().unwrap_or_else(|| model_settings_from_config(&state.model_config()));
    let registry_projects = list_all_registry_projects(state.paths()).unwrap_or_default();
    let llm_status = llm_status(&state.model_config());
    let acp_ok = acp_available(&resolved_acp_settings());

    AppBootstrap {
        model_settings: settings,
        registry_projects,
        llm_status,
        acp_ok,
    }
}

#[tauri::command]
pub fn get_knowledge_root(
    state: State<'_, AppState>,
    project_slug: Option<String>,
) -> Result<String, String> {
    if let Some(slug) = project_slug.filter(|s| !s.trim().is_empty()) {
        return state
            .paths()
            .try_project_dir(&slug)
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string());
    }
    Ok(String::new())
}

#[tauri::command]
pub fn check_acp() -> bool {
    acp_available(&resolved_acp_settings())
}

#[tauri::command]
pub fn acp_spawn_command_cmd() -> String {
    acp_spawn_command(&resolved_acp_settings())
}

/// Backward-compatible alias.
#[tauri::command]
pub fn check_opencode() -> bool {
    check_acp()
}

#[tauri::command]
pub fn check_llm(state: State<'_, AppState>) -> terrain_agent::LlmStatus {
    llm_status(&state.model_config())
}

#[tauri::command]
pub fn get_model_settings(state: State<'_, AppState>) -> ModelSettings {
    load_model_settings().unwrap_or_else(|| model_settings_from_config(&state.model_config()))
}

#[tauri::command]
pub fn save_model_settings_cmd(
    state: State<'_, AppState>,
    settings: ModelSettings,
) -> Result<terrain_agent::LlmStatus, String> {
    save_model_settings(&settings).map_err(|e| e.to_string())?;
    let config = resolve_model_config();
    state.set_model_config(config);
    Ok(llm_status(&state.model_config()))
}

#[tauri::command]
pub fn copy_image_to_clipboard(png_base64: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_image(arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}
