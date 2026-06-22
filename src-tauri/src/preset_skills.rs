//! Resolve bundled preset skills from Tauri resources and deploy to ~/.terrain/.

use terrain_core::{
    deploy_preset_skills_to_home, discover_preset_skills_runtime, init_preset_skills_root,
};
use tauri::{AppHandle, Manager};

/// Load preset skills from app resources (or dev tree) and deploy for CLI/agents.
pub fn init_app_preset_skills(app: &AppHandle) {
    let mut root = None;

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("preset_skills");
        if bundled.join("litho-documents-skill/SKILL.md").is_file() {
            root = Some(bundled);
        }
    }

    if root.is_none() {
        root = discover_preset_skills_runtime();
    }

    let Some(root) = root else {
        tracing::warn!("preset skills not found in app resources or dev tree");
        return;
    };

    init_preset_skills_root(root.clone());
    tracing::info!(root = %root.display(), "initialized Terrain preset skills");

    std::thread::spawn(move || {
        match deploy_preset_skills_to_home() {
            Ok(dest) => tracing::info!(dest = %dest.display(), "deployed preset skills to home"),
            Err(e) => tracing::warn!(error = %e, "preset skills home deploy skipped or failed"),
        }
    });
}
