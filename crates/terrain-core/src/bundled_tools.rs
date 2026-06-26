//! Resolve Terrain-bundled CLI tools (Tauri sidecars / `packages/` prebuilts).
//!
//! Priority when executing or checking tools: **bundled → project-local → PATH**.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::platform::{
    bundled_binary_candidates, codegraph_wrapper_candidates, platform_key,
    platform_key_for_target,
};

static BUNDLED: OnceLock<BundledTools> = OnceLock::new();

/// Paths to prebuilt tools injected by the desktop app or discovered under `packages/`.
#[derive(Debug, Clone, Default)]
pub struct BundledTools {
    pub rtk: Option<PathBuf>,
    pub terrain_cli: Option<PathBuf>,
    /// Shell wrapper (`bin/codegraph`) — requires sibling `node` + `lib/` tree.
    pub codegraph: Option<PathBuf>,
}

/// Install bundled tool paths (call once at app / CLI startup).
pub fn init_bundled_tools(tools: BundledTools) {
    let _ = BUNDLED.set(tools);
}

pub fn bundled_tools() -> &'static BundledTools {
    static EMPTY: BundledTools = BundledTools {
        rtk: None,
        terrain_cli: None,
        codegraph: None,
    };
    BUNDLED.get().unwrap_or(&EMPTY)
}

/// Root of `packages/` in the Terrain source tree.
pub fn packages_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages")
}

/// Discover prebuilts from `packages/{rtk,terrain,codegraph}/<platform>/`.
pub fn discover_bundled_tools_from_packages() -> BundledTools {
    let root = packages_root();
    let platform_dirs = platform_search_order();
    BundledTools {
        rtk: find_bundled_binary(&root, "rtk", &platform_dirs),
        terrain_cli: find_bundled_binary(&root, "terrain", &platform_dirs),
        codegraph: find_codegraph_wrapper(&root, &platform_dirs),
    }
}

fn platform_search_order() -> Vec<String> {
    let current = platform_key();
    let mut dirs = vec![current.clone()];
    if current != "darwin-arm64" {
        dirs.push("darwin-arm64".into());
    }
    dirs
}

fn find_bundled_binary(root: &Path, tool: &str, platform_dirs: &[String]) -> Option<PathBuf> {
    for platform in platform_dirs {
        let base = root.join(tool).join(platform);
        for name in bundled_binary_candidates(tool) {
            let candidate = base.join(&name);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_codegraph_wrapper(root: &Path, platform_dirs: &[String]) -> Option<PathBuf> {
    for platform in platform_dirs {
        let base = root.join("codegraph").join(platform);
        if let Some(found) = find_codegraph_wrapper_under(&base) {
            return Some(found);
        }
    }
    None
}

/// Locate `bin/codegraph` (or `.cmd` / `.exe` on Windows) under a runtime root.
pub fn find_codegraph_wrapper_under(root: &Path) -> Option<PathBuf> {
    for rel in codegraph_wrapper_candidates() {
        let candidate = root.join(rel);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Ensure bundled tools are initialized (packages discovery when unset).
pub fn ensure_bundled_tools_initialized() {
    if BUNDLED.get().is_some() {
        return;
    }
    let discovered = discover_bundled_tools_from_packages();
    if discovered.rtk.is_some()
        || discovered.terrain_cli.is_some()
        || discovered.codegraph.is_some()
    {
        let _ = BUNDLED.set(discovered);
    }
}

/// Sidecar binaries are staged next to the main executable as `{name}-{target-triple}`.
pub fn resolve_sidecar_next_to_exe(exe_dir: &Path, name: &str) -> Option<PathBuf> {
    let triple = option_env!("TERRAIN_TARGET_TRIPLE").unwrap_or_else(|| {
        if cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    });
    let platform = platform_key_for_target(triple);
    let mut candidates = vec![
        exe_dir.join(format!("{name}-{triple}")),
        exe_dir.join(format!("binaries/{name}-{triple}")),
        exe_dir.join(name),
    ];
    for bin_name in bundled_binary_candidates(name) {
        candidates.push(exe_dir.join(format!("binaries/{bin_name}")));
        candidates.push(
            exe_dir
                .join("../../packages")
                .join(name)
                .join(platform)
                .join(&bin_name),
        );
    }
    for candidate in candidates {
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn bundled_rtk() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    bundled_tools().rtk.clone()
}

pub(crate) fn bundled_codegraph() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    bundled_tools().codegraph.clone()
}

pub fn bundled_terrain_cli() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    bundled_tools().terrain_cli.clone()
}

pub(crate) fn run_bundled_check(program: &Path, args: &[&str], cwd: &Path) -> bool {
    std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_rtk_from_packages_when_present() {
        let tools = discover_bundled_tools_from_packages();
        let platform = platform_key();
        let rtk = packages_root().join("rtk").join(&platform).join("rtk");
        if rtk.is_file() {
            assert_eq!(tools.rtk.as_deref(), Some(rtk.as_path()));
            return;
        }
        let fallback = packages_root()
            .join("rtk")
            .join("darwin-arm64")
            .join("rtk");
        if fallback.is_file() {
            assert_eq!(tools.rtk.as_deref(), Some(fallback.as_path()));
        }
    }
}
