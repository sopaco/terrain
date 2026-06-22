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
        let dest = bin_dir.join("rtk");
        symlink_ensure(&dest, &src, opts.force)?;
        paths.rtk = Some(dest.display().to_string());
    }

    if let Some(src) = bundled_terrain_cli() {
        let dest = bin_dir.join("terrain");
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

        let codegraph_bin = runtime_dest.join("bin/codegraph");
        if codegraph_bin.is_file() {
            let dest = bin_dir.join("codegraph");
            symlink_ensure(&dest, &codegraph_bin, opts.force)?;
            paths.codegraph = Some(dest.display().to_string());
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
        "usage": {
            "rtk": "Use the `rtk` field (`~/.terrain/bin/rtk`); expand `~` in shell",
            "codegraph": "Use the `codegraph` field; index is per-repo under .codegraph/",
            "terrain": "ACP / CLI knowledge tools: `terrain tools …`"
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
    // `.../darwin-arm64/bin/codegraph` → runtime root `.../darwin-arm64`
    let runtime = bin.parent()?.parent()?;
    if runtime.join("node").is_file() && runtime.join("lib").is_dir() {
        Some(runtime.to_path_buf())
    } else {
        None
    }
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
        fs::copy(target, link)?;
    }
    Ok(())
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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
        assert!(!manifest.bin_dir.contains("/Users/"));
    }
}
