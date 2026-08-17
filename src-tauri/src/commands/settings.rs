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

/// Write share-image pages into `dir`, returning the paths written.
///
/// Multi-page exports are numbered; an existing file is never overwritten — the
/// name gains a `-2`, `-3`, … suffix instead, so re-exporting the same answer in
/// the same minute cannot silently replace the earlier images.
#[tauri::command]
pub fn save_png_files(
    dir: String,
    base_name: String,
    pngs_base64: Vec<String>,
) -> Result<Vec<String>, String> {
    use base64::Engine;

    let lang = terrain_core::current_language();
    let dir = std::path::PathBuf::from(dir);
    if !dir.is_dir() {
        return Err(lang
            .tr(
                &format!("目录不存在：{}", dir.display()),
                &format!("Directory does not exist: {}", dir.display()),
            )
            .to_string());
    }
    if pngs_base64.is_empty() {
        return Err(lang.tr("没有可导出的图片", "No images to export").to_string());
    }

    let stem = sanitize_file_stem(&base_name);
    let numbered = pngs_base64.len() > 1;
    let mut written = Vec::with_capacity(pngs_base64.len());

    for (index, png) in pngs_base64.iter().enumerate() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(png.trim())
            .map_err(|e| e.to_string())?;
        let name = if numbered {
            format!("{stem}-{:02}", index + 1)
        } else {
            stem.clone()
        };
        let path = unique_png_path(&dir, &name);
        std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path.to_string_lossy().to_string());
    }

    Ok(written)
}

fn sanitize_file_stem(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    let stem: String = trimmed.chars().take(64).collect();
    if stem.is_empty() {
        "terrain-ask".to_string()
    } else {
        stem
    }
}

fn unique_png_path(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let first = dir.join(format!("{name}.png"));
    if !first.exists() {
        return first;
    }
    for n in 2..1000 {
        let candidate = dir.join(format!("{name}-{n}.png"));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}
