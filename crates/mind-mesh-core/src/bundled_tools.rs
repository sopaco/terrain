//! Resolve MindMesh-bundled CLI tools (Tauri sidecars / `packages/` prebuilts).
//!
//! Priority when executing or checking tools: **bundled → project-local → PATH**.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const PLATFORM_DIR: &str = "darwin-arm64";

static BUNDLED: OnceLock<BundledTools> = OnceLock::new();

/// Paths to prebuilt tools injected by the desktop app or discovered under `packages/`.
#[derive(Debug, Clone, Default)]
pub struct BundledTools {
    pub rtk: Option<PathBuf>,
    pub mind_mesh_cli: Option<PathBuf>,
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
        mind_mesh_cli: None,
        codegraph: None,
    };
    BUNDLED.get().unwrap_or(&EMPTY)
}

/// Root of `packages/` in the MindMesh source tree.
pub fn packages_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages")
}

/// Discover macOS arm64 prebuilts from `packages/{rtk,mind-mesh,codegraph}/darwin-arm64/`.
pub fn discover_bundled_tools_from_packages() -> BundledTools {
    let root = packages_root();
    BundledTools {
        rtk: executable_if_exists(root.join("rtk").join(PLATFORM_DIR).join("rtk")),
        mind_mesh_cli: executable_if_exists(
            root.join("mind-mesh").join(PLATFORM_DIR).join("mind-mesh"),
        ),
        codegraph: executable_if_exists(
            root
                .join("codegraph")
                .join(PLATFORM_DIR)
                .join("bin")
                .join("codegraph"),
        ),
    }
}

/// Ensure bundled tools are initialized (packages discovery when unset).
pub fn ensure_bundled_tools_initialized() {
    if BUNDLED.get().is_some() {
        return;
    }
    let discovered = discover_bundled_tools_from_packages();
    if discovered.rtk.is_some()
        || discovered.mind_mesh_cli.is_some()
        || discovered.codegraph.is_some()
    {
        let _ = BUNDLED.set(discovered);
    }
}

/// Sidecar binaries are staged next to the main executable as `{name}-{target-triple}`.
pub fn resolve_sidecar_next_to_exe(exe_dir: &Path, name: &str) -> Option<PathBuf> {
    let triple = option_env!("MIND_MESH_TARGET_TRIPLE").unwrap_or("aarch64-apple-darwin");
    let candidates = [
        exe_dir.join(format!("{name}-{triple}")),
        exe_dir.join(format!("binaries/{name}-{triple}")),
        exe_dir.join(name),
    ];
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

pub fn bundled_mind_mesh_cli() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    bundled_tools().mind_mesh_cli.clone()
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

fn executable_if_exists(path: PathBuf) -> Option<PathBuf> {
    if is_executable_file(&path) {
        Some(path)
    } else {
        None
    }
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
        let rtk = packages_root()
            .join("rtk")
            .join(PLATFORM_DIR)
            .join("rtk");
        if rtk.is_file() {
            assert_eq!(tools.rtk.as_deref(), Some(rtk.as_path()));
        }
    }
}
