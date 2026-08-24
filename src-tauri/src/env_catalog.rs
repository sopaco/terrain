//! Resolve bundled env catalog from Tauri resources and deploy to ~/.terrain/.

use terrain_core::{
    deploy_env_catalog_to_home, discover_env_catalog_runtime, init_env_catalog_root,
};
use tauri::{AppHandle, Manager};

/// Load env catalog from app resources (or dev tree).
pub fn init_app_env_catalog(app: &AppHandle) {
    let mut root = None;

    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("env-catalog");
        if bundled.join("catalog.json").is_file() {
            root = Some(bundled);
        }
    }

    if root.is_none() {
        root = discover_env_catalog_runtime();
    }

    let Some(root) = root else {
        tracing::warn!("env catalog not found in app resources or dev tree");
        return;
    };

    init_env_catalog_root(root.clone());
    tracing::info!(root = %root.display(), "initialized Terrain env catalog");

    std::thread::spawn(move || {
        match deploy_env_catalog_to_home() {
            Ok(dest) => tracing::info!(dest = %dest.display(), "deployed env catalog to home"),
            Err(e) => tracing::warn!(error = %e, "env catalog home deploy skipped or failed"),
        }
    });
}
