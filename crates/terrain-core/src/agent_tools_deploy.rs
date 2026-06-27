//! Deploy Terrain-bundled CLIs where external Coding Agents can invoke them.
//!
//! App bundle / `packages/` paths are not on Agent PATH — we symlink into
//! `~/.terrain/bin/` and `~/.terrain/tools/`. Re-deploy only when missing,
//! broken, or forced.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bundled_tools::{bundled_terrain_cli, bundled_rtk, ensure_bundled_tools_initialized};
use crate::error::{CoreError, Result};
use crate::path_portable::to_tilde_path;
use crate::platform::agent_tool_filename;

const CODEGRAPH_RUNTIME_NAME: &str = "codegraph-runtime";

/// When `force` is false, keep existing valid deployments; only fill gaps.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeployOptions {
    pub force: bool,
}

/// Resolved paths after deploy (also written to JSON manifests).
#[derive(Debug, Clone, Serialize)]
pub struct AgentToolPaths {
    pub bin_dir: String,
    pub rtk: Option<String>,
    pub codegraph: Option<String>,
    pub terrain: Option<String>,
    pub codegraph_runtime: Option<String>,
}

pub fn agent_bin_dir() -> PathBuf {
    user_home()
        .map(|h| h.join(".terrain/bin"))
        .unwrap_or_else(|| PathBuf::from(".terrain/bin"))
}

pub fn agent_tools_runtime_dir() -> PathBuf {
    user_home()
        .map(|h| h.join(".terrain/tools"))
        .unwrap_or_else(|| PathBuf::from(".terrain/tools"))
}

pub fn deploy_agent_toolchain() -> Result<AgentToolPaths> {
    deploy_agent_toolchain_with_options(DeployOptions::default())
}

/// Materialize bundled binaries for Agent shell access.
pub fn deploy_agent_toolchain_with_options(opts: DeployOptions) -> Result<AgentToolPaths> {
    ensure_bundled_tools_initialized();
    let bin_dir = agent_bin_dir();
    fs::create_dir_all(&bin_dir)?;

    let mut paths = AgentToolPaths {
        bin_dir: bin_dir.display().to_string(),
        rtk: None,
        codegraph: None,
        terrain: None,
        codegraph_runtime: None,
    };

    if let Some(src) = bundled_rtk() {
        let dest = bin_dir.join(agent_tool_filename("rtk"));
        symlink_ensure(&dest, &src, opts.force)?;
        paths.rtk = Some(dest.display().to_string());
    }

    if let Some(src) = bundled_terrain_cli() {
        let dest = bin_dir.join(agent_tool_filename("terrain"));
        symlink_ensure(&dest, &src, opts.force)?;
        paths.terrain = Some(dest.display().to_string());
    }

    if let Some(runtime_src) = bundled_codegraph_runtime() {
        let runtime_dest = agent_tools_runtime_dir().join(CODEGRAPH_RUNTIME_NAME);
        fs::create_dir_all(
            runtime_dest
                .parent()
                .unwrap_or(Path::new(".")),
        )?;
        symlink_ensure(&runtime_dest, &runtime_src, opts.force)?;

        let codegraph_bin = find_codegraph_bin(&runtime_dest);
        if let Some(codegraph_src) = codegraph_bin {
            #[cfg(windows)]
            {
                // Copying the bundled .cmd to ~/.terrain/bin/ breaks its relative paths
                // (`%~dp0..\node.exe`). Instead, write a tiny wrapper that delegates to the
                // runtime copy under ~/.terrain/tools/codegraph-runtime/.
                let _ = codegraph_src;
                let wrapper = bin_dir.join("codegraph.cmd");
                let wrapper_content = "@\"%~dp0..\\tools\\codegraph-runtime\\bin\\codegraph.cmd\" %*\n";
                fs::write(&wrapper, wrapper_content).map_err(|e| {
                    CoreError::InvalidDoc(format!(
                        "write codegraph wrapper {}: {e}",
                        wrapper.display()
                    ))
                })?;
                paths.codegraph = Some(wrapper.display().to_string());
            }
            #[cfg(not(windows))]
            {
                let dest_name = codegraph_src
                    .file_name()
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| std::ffi::OsString::from(agent_tool_filename("codegraph")));
                let dest = bin_dir.join(dest_name);
                symlink_ensure(&dest, &codegraph_src, opts.force)?;
                paths.codegraph = Some(dest.display().to_string());
            }
            paths.codegraph_runtime = Some(runtime_dest.display().to_string());
        }
    }

    if paths.rtk.is_none() && paths.codegraph.is_none() && paths.terrain.is_none() {
        return Err(CoreError::InvalidDoc(
            "无可用内置工具可部署到 ~/.terrain/bin/".into(),
        ));
    }

    write_global_manifest(&paths)?;
    Ok(paths)
}

fn paths_for_manifest(paths: &AgentToolPaths) -> AgentToolPaths {
    AgentToolPaths {
        bin_dir: to_tilde_path(Path::new(&paths.bin_dir)),
        rtk: paths.rtk.as_ref().map(|p| to_tilde_path(Path::new(p))),
        codegraph: paths
            .codegraph
            .as_ref()
            .map(|p| to_tilde_path(Path::new(p))),
        terrain: paths.terrain.as_ref().map(|p| to_tilde_path(Path::new(p))),
        codegraph_runtime: paths
            .codegraph_runtime
            .as_ref()
            .map(|p| to_tilde_path(Path::new(p))),
    }
}

/// Write per-repo manifest so Agents read concrete paths from the project.
pub fn write_repo_agent_tools_manifest(repo: &Path, paths: &AgentToolPaths) -> Result<String> {
    let env_dir = repo.join(".terrain/env");
    fs::create_dir_all(&env_dir)?;
    let path = env_dir.join("agent-tools.json");
    let manifest = paths_for_manifest(paths);
    let doc = serde_json::json!({
        "bin_dir": manifest.bin_dir,
        "rtk": manifest.rtk,
        "codegraph": manifest.codegraph,
        "terrain": manifest.terrain,
        "codegraph_runtime": manifest.codegraph_runtime,
        "fallback": {
            "rtk": "bunx @terrain-ai/rtk",
            "codegraph": "bunx codegraph",
            "terrain": "bunx @terrain-ai/cli"
        },
        "usage": {
            "rtk": "Prefer ~/.terrain/bin/rtk if executable; else bunx @terrain-ai/rtk or npx @terrain-ai/rtk",
            "codegraph": "Prefer ~/.terrain/bin/codegraph if executable; else bunx codegraph; index under .codegraph/",
            "terrain": "Prefer ~/.terrain/bin/terrain if executable; else bunx @terrain-ai/cli or npx @terrain-ai/cli"
        }
    });
    fs::write(&path, serde_json::to_string_pretty(&doc)?)?;
    Ok(path.display().to_string())
}

fn write_global_manifest(paths: &AgentToolPaths) -> Result<()> {
    let dir = user_home()
        .map(|h| h.join(".terrain"))
        .ok_or_else(|| CoreError::InvalidDoc("cannot resolve HOME".into()))?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("agent-tools.json");
    let manifest = paths_for_manifest(paths);
    fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(())
}

fn bundled_codegraph_runtime() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    let bin = crate::bundled_tools::bundled_tools().codegraph.as_ref()?;
    // `.../<platform>/bin/codegraph` → runtime root `.../<platform>`
    let runtime = bin.parent()?.parent()?;
    if node_executable_exists(runtime) && runtime.join("lib").is_dir() {
        Some(runtime.to_path_buf())
    } else {
        None
    }
}

fn node_executable_exists(runtime: &Path) -> bool {
    runtime.join("node").is_file() || runtime.join("node.exe").is_file()
}

fn find_codegraph_bin(runtime_dest: &Path) -> Option<PathBuf> {
    for rel in crate::platform::codegraph_wrapper_candidates() {
        let candidate = runtime_dest.join(rel);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn symlink_ensure(link: &Path, target: &Path, force: bool) -> Result<()> {
    if !force && link_is_valid(link, target) {
        return Ok(());
    }
    symlink_replace(link, target)
}

fn link_is_valid(link: &Path, expected_target: &Path) -> bool {
    let Ok(meta) = link.symlink_metadata() else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return link.is_file() && paths_equal(link, expected_target);
    }
    let Ok(actual) = fs::read_link(link) else {
        return false;
    };
    if !actual.is_absolute() {
        let resolved = link.parent().map(|p| p.join(&actual));
        return resolved.is_some_and(|p| paths_equal(&p, expected_target));
    }
    paths_equal(&actual, expected_target)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn symlink_replace(link: &Path, target: &Path) -> Result<()> {
    if link.symlink_metadata().is_ok() {
        fs::remove_file(link).or_else(|_| fs::remove_dir_all(link))?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        CoreError::InvalidDoc(format!(
            "symlink {} -> {}: {e}",
            link.display(),
            target.display()
        ))
    })?;
    #[cfg(not(unix))]
    {
        if target.is_dir() {
            copy_dir_recursive(target, link)?;
        } else {
            fs::copy(target, link).map_err(|e| {
                CoreError::InvalidDoc(format!(
                    "copy {} -> {}: {e}",
                    target.display(),
                    link.display()
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| {
                CoreError::InvalidDoc(format!(
                    "copy {} -> {}: {e}",
                    from.display(),
                    to.display()
                ))
            })?;
        }
    }
    Ok(())
}

fn user_home() -> Option<PathBuf> {
    crate::platform::user_home()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_bin_dir_under_home() {
        if user_home().is_some() {
            assert!(agent_bin_dir().ends_with(".terrain/bin"));
        }
    }

    #[test]
    fn manifest_paths_use_tilde_not_absolute_home() {
        if user_home().is_none() {
            return;
        }
        let paths = AgentToolPaths {
            bin_dir: agent_bin_dir().display().to_string(),
            rtk: Some(agent_bin_dir().join("rtk").display().to_string()),
            codegraph: None,
            terrain: None,
            codegraph_runtime: None,
        };
        let manifest = paths_for_manifest(&paths);
        assert!(manifest.bin_dir.starts_with("~/"));
        assert!(manifest.rtk.as_ref().is_some_and(|p| p.starts_with("~/")));
        let home = user_home().expect("home");
        let home_str = home.to_string_lossy();
        assert!(!manifest.bin_dir.contains(home_str.as_ref()));
    }

    #[test]
    fn codegraph_runtime_detects_node_exe() {
        // The prebuilt codegraph bundle ships node.exe on Windows and `node` on macOS.
        // This test verifies the runtime check recognizes both without breaking macOS.
        let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/codegraph/win32-x64");
        let codegraph_cmd = runtime.join("bin/codegraph.cmd");
        if !codegraph_cmd.is_file() {
            return; // skip if Windows bundle is not staged
        }
        assert!(
            node_executable_exists(&runtime),
            "expected node_executable_exists to find node.exe in {}",
            runtime.display()
        );
    }
}
