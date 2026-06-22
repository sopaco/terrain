//! Resolve bundled tool paths from Tauri sidecars and resource dir.

use terrain_core::{init_bundled_tools, resolve_sidecar_next_to_exe};
use tauri::{AppHandle, Manager};

const CODEGRAPH_RESOURCE: &str = "tools/codegraph/bin/codegraph";

/// Merge Tauri-bundled paths with `packages/` fallbacks (dev / `tauri dev`).
pub fn init_app_bundled_tools(app: &AppHandle) {
    let mut tools = terrain_core::discover_bundled_tools_from_packages();

    if let Ok(exe) = tauri::process::current_binary(&app.env()) {
        if let Some(parent) = exe.parent() {
            if let Some(rtk) = resolve_sidecar_next_to_exe(parent, "rtk") {
                tools.rtk = Some(rtk);
            }
            if let Some(cli) = resolve_sidecar_next_to_exe(parent, "terrain-cli") {
                tools.terrain_cli = Some(cli);
            }
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let codegraph = resource_dir.join(CODEGRAPH_RESOURCE);
        if codegraph.is_file() {
            tools.codegraph = Some(codegraph);
        }
    }

    init_bundled_tools(tools);

    std::thread::spawn(|| {
        match terrain_core::deploy_agent_toolchain_with_options(Default::default()) {
            Ok(paths) => {
                terrain_core::invalidate_env_status_cache();
                tracing::info!(
                    bin_dir = %paths.bin_dir,
                    "deployed Terrain agent toolchain for external Coding Agents"
                );
            }
            Err(e) => tracing::warn!(error = %e, "agent toolchain deploy skipped or failed"),
        }
    });
}
