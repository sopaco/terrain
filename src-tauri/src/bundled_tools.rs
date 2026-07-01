//! Resolve bundled tool paths from Tauri sidecars and resource dir.

use terrain_core::{find_codegraph_wrapper_under, init_bundled_tools, resolve_sidecar_next_to_exe};
use tauri::{AppHandle, Manager};

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
        if let Some(codegraph) =
            find_codegraph_wrapper_under(&resource_dir.join("tools/codegraph"))
        {
            tools.codegraph = Some(codegraph);
        }
    }

    init_bundled_tools(tools);

    std::thread::spawn(|| {
        match terrain_core::deploy_agent_toolchain_with_options(Default::default()) {
            Ok(paths) => {
                tracing::info!(
                    bin_dir = %paths.bin_dir,
                    "deployed Terrain agent toolchain for external Coding Agents"
                );
            }
            Err(e) => tracing::warn!(error = %e, "agent toolchain deploy skipped or failed"),
        }
    });
}
