//! Deploy Terrain-bundled CLIs where external Coding Agents can invoke them.
//!
//! App bundle / `packages/` paths are not on Agent PATH — we materialize into
//! `~/.terrain/bin/` and `~/.terrain/tools/`:
//!
//! - **Unix**: symlinks (cheap, always track the bundled sidecar)
//! - **Windows**: file copies with fingerprint-based skip, atomic replace, and
//!   graceful fallback when the destination is locked (e.g. Agent running `terrain.exe`)

use std::fs;
#[cfg(not(unix))]
use std::io;
use std::path::{Path, PathBuf};
#[cfg(any(test, not(unix)))]
use std::time::UNIX_EPOCH;

use serde::Serialize;
#[cfg(any(test, not(unix)))]
use serde::Deserialize;

use crate::bundled_tools::{bundled_terrain_cli, bundled_rtk, ensure_bundled_tools_initialized};
use crate::error::{CoreError, Result};
use crate::path_portable::to_tilde_path;
use crate::platform::agent_tool_filename;

const CODEGRAPH_RUNTIME_NAME: &str = "codegraph-runtime";

#[cfg(windows)]
const CODEGRAPH_WRAPPER_CMD: &str =
    "@\"%~dp0..\\tools\\codegraph-runtime\\bin\\codegraph.cmd\" %*\n";

/// When `force` is false, keep existing valid deployments; only fill gaps or stale copies.
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

/// Shared file stats helper (also used in cross-platform tests).
#[cfg(any(test, not(unix)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct FileFingerprint {
    size: u64,
    modified_ms: u128,
}

#[cfg(not(unix))]
type SourceFingerprint = FileFingerprint;

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
        materialize_ensure("rtk", &dest, &src, opts.force)?;
        paths.rtk = Some(dest.display().to_string());
    }

    if let Some(src) = bundled_terrain_cli() {
        let dest = bin_dir.join(agent_tool_filename("terrain"));
        materialize_ensure("terrain", &dest, &src, opts.force)?;
        paths.terrain = Some(dest.display().to_string());
    }

    if let Some(runtime_src) = bundled_codegraph_runtime() {
        let runtime_dest = agent_tools_runtime_dir().join(CODEGRAPH_RUNTIME_NAME);
        fs::create_dir_all(
            runtime_dest
                .parent()
                .unwrap_or(Path::new(".")),
        )?;
        materialize_ensure("codegraph-runtime", &runtime_dest, &runtime_src, opts.force)?;

        let codegraph_bin = find_codegraph_bin(&runtime_dest);
        if let Some(codegraph_src) = codegraph_bin {
            #[cfg(windows)]
            {
                let _ = codegraph_src;
                let wrapper = bin_dir.join("codegraph.cmd");
                write_codegraph_wrapper(&wrapper, opts.force)?;
                paths.codegraph = Some(wrapper.display().to_string());
            }
            #[cfg(not(windows))]
            {
                let dest_name = codegraph_src
                    .file_name()
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| std::ffi::OsString::from(agent_tool_filename("codegraph")));
                let dest = bin_dir.join(dest_name);
                materialize_ensure("codegraph", &dest, &codegraph_src, opts.force)?;
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

#[cfg(windows)]
fn write_codegraph_wrapper(wrapper: &Path, force: bool) -> Result<()> {
    if !force {
        if let Ok(existing) = fs::read_to_string(wrapper) {
            if existing == CODEGRAPH_WRAPPER_CMD {
                return Ok(());
            }
        }
    }
    fs::write(wrapper, CODEGRAPH_WRAPPER_CMD).map_err(|e| {
        CoreError::InvalidDoc(format!(
            "write codegraph wrapper {}: {e}",
            wrapper.display()
        ))
    })
}

/// Deploy or refresh `dest` from bundled `source`.
fn materialize_ensure(key: &str, dest: &Path, source: &Path, force: bool) -> Result<()> {
    if !force && deployment_is_current(key, dest, source)? {
        return Ok(());
    }

    #[cfg(unix)]
    {
        symlink_replace(dest, source)?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        if source.is_dir() {
            materialize_dir(key, dest, source)
        } else {
            materialize_file(key, dest, source)
        }
    }
}

#[cfg(unix)]
fn deployment_is_current(_key: &str, dest: &Path, source: &Path) -> Result<bool> {
    let Ok(meta) = dest.symlink_metadata() else {
        return Ok(false);
    };
    if !meta.file_type().is_symlink() {
        return Ok(false);
    }
    let Ok(actual) = fs::read_link(dest) else {
        return Ok(false);
    };
    let resolved = if actual.is_absolute() {
        actual
    } else {
        dest.parent()
            .map(|p| p.join(&actual))
            .unwrap_or(actual)
    };
    Ok(paths_equal(&resolved, source))
}

#[cfg(not(unix))]
fn deployment_is_current(key: &str, dest: &Path, source: &Path) -> Result<bool> {
    if !dest_exists_for_source(source, dest) {
        return Ok(false);
    }
    let Some(stored) = read_deploy_marker(key, dest)? else {
        return Ok(false);
    };
    Ok(stored == source_fingerprint(source)?)
}

#[cfg(not(unix))]
fn dest_exists_for_source(source: &Path, dest: &Path) -> bool {
    if source.is_dir() {
        dest.is_dir() && node_executable_exists(dest)
    } else {
        dest.is_file()
    }
}

#[cfg(not(unix))]
fn materialize_file(key: &str, dest: &Path, source: &Path) -> Result<()> {
    let temp = temp_sibling_path(dest);
    if let Err(e) = fs::copy(source, &temp) {
        let _ = fs::remove_file(&temp);
        return Err(io_error("copy bundled tool to staging file", &temp, e));
    }

    match replace_path(&temp, dest) {
        Ok(()) => {
            let _ = fs::remove_file(&temp);
            record_deploy_state(key, dest, source)?;
            Ok(())
        }
        Err(e) if dest.is_file() && is_destination_locked(&e) => {
            let _ = fs::remove_file(&temp);
            // Keep the existing deployment; Agent may be running the binary.
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&temp);
            Err(io_error("replace bundled tool", dest, e))
        }
    }
}

#[cfg(not(unix))]
fn materialize_dir(key: &str, dest: &Path, source: &Path) -> Result<()> {
    let parent = dest
        .parent()
        .ok_or_else(|| CoreError::InvalidDoc(format!("invalid deploy dir {}", dest.display())))?;
    let staging = parent.join(format!(
        "{}.staging",
        dest.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("runtime")
    ));

    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    copy_dir_recursive(source, &staging)?;

    match replace_path(&staging, dest) {
        Ok(()) => {
            record_deploy_state(key, dest, source)?;
            Ok(())
        }
        Err(e) if dest.is_dir() && is_destination_locked(&e) => {
            let _ = fs::remove_dir_all(&staging);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            Err(io_error("replace bundled runtime dir", dest, e))
        }
    }
}

#[cfg(unix)]
fn symlink_replace(link: &Path, target: &Path) -> Result<()> {
    if link.symlink_metadata().is_ok() {
        fs::remove_file(link).or_else(|_| fs::remove_dir_all(link))?;
    }
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        CoreError::InvalidDoc(format!(
            "symlink {} -> {}: {e}",
            link.display(),
            target.display()
        ))
    })
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(not(unix))]
fn temp_sibling_path(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tool");
    dest.with_file_name(format!("{name}.deploy-tmp"))
}

#[cfg(not(unix))]
fn replace_path(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        if to.is_dir() {
            fs::remove_dir_all(to)?;
        } else {
            fs::remove_file(to)?;
        }
    }
    fs::rename(from, to)
}

#[cfg(not(unix))]
fn is_destination_locked(err: &io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(err.raw_os_error(), Some(5 | 32))
    }
    #[cfg(not(windows))]
    {
        matches!(
            err.kind(),
            io::ErrorKind::PermissionDenied | io::ErrorKind::ResourceBusy
        )
    }
}

#[cfg(not(unix))]
fn io_error(action: &str, path: &Path, err: io::Error) -> CoreError {
    CoreError::InvalidDoc(format!("{action} {}: {err}", path.display()))
}

#[cfg(not(unix))]
fn source_fingerprint(path: &Path) -> Result<SourceFingerprint> {
    if path.is_dir() {
        runtime_dir_fingerprint(path)
    } else {
        file_fingerprint(path)
    }
}

#[cfg(any(test, not(unix)))]
fn file_fingerprint(path: &Path) -> Result<FileFingerprint> {
    let meta = path
        .metadata()
        .map_err(|e| CoreError::InvalidDoc(format!("read bundled tool metadata {}: {e}", path.display())))?;
    Ok(FileFingerprint {
        size: meta.len(),
        modified_ms: file_modified_ms(&meta)?,
    })
}

#[cfg(not(unix))]
fn runtime_dir_fingerprint(runtime: &Path) -> Result<SourceFingerprint> {
    let node = if runtime.join("node.exe").is_file() {
        runtime.join("node.exe")
    } else {
        runtime.join("node")
    };
    let mut fp = file_fingerprint(&node)?;
    if let Ok(lib_meta) = runtime.join("lib/package.json").metadata() {
        fp.modified_ms = fp.modified_ms.saturating_add(file_modified_ms(&lib_meta)?);
    }
    Ok(fp)
}

#[cfg(any(test, not(unix)))]
fn file_modified_ms(meta: &fs::Metadata) -> Result<u128> {
    meta.modified()
        .or_else(|_| meta.created())
        .map_err(|e| CoreError::InvalidDoc(format!("read file time: {e}")))
        .and_then(|t| {
            t.duration_since(UNIX_EPOCH)
                .map_err(|e| CoreError::InvalidDoc(format!("file time before epoch: {e}")))
                .map(|d| d.as_millis())
        })
}

#[cfg(not(unix))]
fn deploy_marker_path(key: &str, dest: &Path) -> PathBuf {
    dest.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{key}.terrain-deploy.json"))
}

#[cfg(not(unix))]
fn read_deploy_marker(key: &str, dest: &Path) -> Result<Option<SourceFingerprint>> {
    let path = deploy_marker_path(key, dest);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| io_error("read deploy marker", &path, e))?;
    serde_json::from_str(&raw).map_err(|e| {
        CoreError::InvalidDoc(format!("parse deploy marker {}: {e}", path.display()))
    })
}

#[cfg(not(unix))]
fn record_deploy_state(key: &str, dest: &Path, source: &Path) -> Result<()> {
    let path = deploy_marker_path(key, dest);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let fp = source_fingerprint(source)?;
    fs::write(&path, serde_json::to_string_pretty(&fp)?).map_err(|e| {
        io_error("write deploy marker", &path, e)
    })
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
            fs::copy(&from, &to).map_err(|e| io_error("copy bundled runtime file", &to, e))?;
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
    use std::thread;
    use std::time::Duration;

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
        let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/codegraph/win32-x64");
        let codegraph_cmd = runtime.join("bin/codegraph.cmd");
        if !codegraph_cmd.is_file() {
            return;
        }
        assert!(
            node_executable_exists(&runtime),
            "expected node_executable_exists to find node.exe in {}",
            runtime.display()
        );
    }

    #[test]
    fn fingerprint_changes_when_source_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("tool.bin");
        fs::write(&src, b"v1").expect("write");
        let fp1 = file_fingerprint(&src).expect("fp1");
        thread::sleep(Duration::from_millis(20));
        fs::write(&src, b"v2-longer").expect("rewrite");
        let fp2 = file_fingerprint(&src).expect("fp2");
        assert_ne!(fp1, fp2);
    }

    #[cfg(not(unix))]
    #[test]
    fn skips_copy_when_fingerprint_matches() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("terrain.exe");
        let dest = dir.path().join("bin/terrain.exe");
        fs::create_dir_all(dest.parent().expect("parent")).expect("mkdir");
        fs::write(&source, b"terrain-binary").expect("write source");
        materialize_file("terrain", &dest, &source).expect("first deploy");
        let before = fs::metadata(&dest).expect("meta").len();
        materialize_file("terrain", &dest, &source).expect("second deploy");
        let after = fs::metadata(&dest).expect("meta").len();
        assert_eq!(before, after);
        assert!(deployment_is_current("terrain", &dest, &source).expect("current"));
    }

    #[cfg(not(unix))]
    #[test]
    fn recopies_when_source_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("terrain.exe");
        let dest = dir.path().join("bin/terrain.exe");
        fs::create_dir_all(dest.parent().expect("parent")).expect("mkdir");
        fs::write(&source, b"v1").expect("write source");
        materialize_file("terrain", &dest, &source).expect("first deploy");
        fs::write(&source, b"v2-updated").expect("update source");
        materialize_file("terrain", &dest, &source).expect("second deploy");
        let content = fs::read(&dest).expect("read dest");
        assert_eq!(content, b"v2-updated");
    }

    #[cfg(not(unix))]
    #[test]
    fn keeps_existing_binary_when_replace_blocked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("terrain.exe");
        let dest = dir.path().join("bin/terrain.exe");
        fs::create_dir_all(dest.parent().expect("parent")).expect("mkdir");
        fs::write(&source, b"old").expect("write source");
        materialize_file("terrain", &dest, &source).expect("first deploy");

        let locked = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&dest)
            .expect("open dest");
        fs::write(&source, b"new").expect("update source");
        materialize_file("terrain", &dest, &source).expect("deploy while locked");
        drop(locked);

        let content = fs::read(&dest).expect("read dest");
        assert_eq!(content, b"old");
    }

    #[cfg(unix)]
    #[test]
    fn unix_symlink_tracks_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("terrain");
        fs::write(&source, b"terrain").expect("write source");
        let dest = dir.path().join("bin/terrain");
        fs::create_dir_all(dest.parent().expect("parent")).expect("mkdir");
        symlink_replace(&dest, &source).expect("symlink");
        assert!(deployment_is_current("terrain", &dest, &source).expect("check"));
        let linked = fs::read_link(&dest).expect("readlink");
        assert!(paths_equal(&linked, &source));
    }
}
