//! Environment integration catalog (`env-catalog/catalog.json`).
//!
//! Compile-time `CARGO_MANIFEST_DIR` only works in the Terrain source tree. Packaged
//! apps and downstream crates must resolve the catalog at runtime from app resources,
//! `~/.terrain/env-catalog/`, or `TERRAIN_ENV_CATALOG`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::error::{CoreError, Result};

static ROOT: OnceLock<PathBuf> = OnceLock::new();
static CATALOG: OnceLock<EnvCatalog> = OnceLock::new();

const CATALOG_FILE: &str = "catalog.json";

/// Inject env catalog root (Tauri resource dir or dev discovery). Call once at app startup.
pub fn init_env_catalog_root(path: PathBuf) {
    if path.join(CATALOG_FILE).is_file() {
        let _ = ROOT.set(path);
    }
}

/// Home copy for CLI / external agents (`~/.terrain/env-catalog/`).
pub fn user_env_catalog_dir() -> PathBuf {
    user_home()
        .map(|h| h.join(".terrain/env-catalog"))
        .unwrap_or_else(|| PathBuf::from(".terrain/env-catalog"))
}

/// Ensure a root is selected when the app has not called [`init_env_catalog_root`].
pub fn ensure_env_catalog_initialized() {
    if ROOT.get().is_some() {
        return;
    }
    if let Ok(raw) = std::env::var("TERRAIN_ENV_CATALOG") {
        let path = PathBuf::from(raw);
        if path.join(CATALOG_FILE).is_file() {
            let _ = ROOT.set(path);
            return;
        }
    }
    let home = user_env_catalog_dir();
    if home.join(CATALOG_FILE).is_file() {
        let _ = ROOT.set(home);
        return;
    }
    if let Some(path) = discover_env_catalog_bundled() {
        let _ = ROOT.set(path);
    }
}

fn load_catalog_from_disk() -> Result<EnvCatalog> {
    let path = env_catalog_root().join(CATALOG_FILE);
    let raw = fs::read_to_string(&path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot read env catalog {}: {e}", path.display()))
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        CoreError::InvalidDoc(format!("invalid env catalog {}: {e}", path.display()))
    })
}

pub fn load_catalog() -> Result<EnvCatalog> {
    if let Some(catalog) = CATALOG.get() {
        return Ok(catalog.clone());
    }
    let catalog = load_catalog_from_disk()?;
    let _ = CATALOG.set(catalog.clone());
    Ok(catalog)
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnvCatalog {
    pub version: u32,
    pub integrations: Vec<IntegrationDef>,
    #[serde(default)]
    pub agents_md_blocks: Vec<AgentsMdBlockDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationDef {
    pub id: String,
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub skill_dir: Option<String>,
    #[serde(default)]
    pub preset_skill: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub check: Option<Vec<String>>,
    #[serde(default)]
    pub install_steps: Vec<InstallStep>,
    #[serde(default)]
    pub skip_commands: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Provided by Terrain app bundle; not user-toggleable when available.
    #[serde(default)]
    pub bundled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallStep {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentsMdBlockDef {
    pub id: String,
    pub version: u32,
    pub fragment: String,
}

pub fn env_catalog_root() -> PathBuf {
    ensure_env_catalog_initialized();
    ROOT.get()
        .cloned()
        .or_else(discover_env_catalog_bundled)
        .unwrap_or_else(fallback_dev_env_catalog_root)
}

/// Materialize bundled env catalog into `~/.terrain/env-catalog/` for CLI / external agents.
pub fn deploy_env_catalog_to_home() -> Result<PathBuf> {
    let dest = user_env_catalog_dir();
    let src = resolve_env_catalog_deploy_source(&dest)?;
    if paths_equal(&src, &dest) {
        return Ok(dest);
    }

    if dest.exists() {
        fs::remove_dir_all(&dest).or_else(|_| fs::remove_file(&dest))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&src, &dest).map_err(|e| {
            CoreError::InvalidDoc(format!(
                "symlink {} -> {}: {e}",
                dest.display(),
                src.display()
            ))
        })?;
    }
    #[cfg(not(unix))]
    {
        copy_dir_recursive(&src, &dest)?;
    }

    if !dest.join(CATALOG_FILE).is_file() {
        return Err(CoreError::InvalidDoc(
            "failed to deploy env catalog to ~/.terrain/env-catalog/".into(),
        ));
    }

    Ok(dest)
}

fn resolve_env_catalog_deploy_source(dest: &Path) -> Result<PathBuf> {
    if let Some(root) = ROOT.get() {
        if !paths_equal(root, dest) && root.join(CATALOG_FILE).is_file() {
            return Ok(root.clone());
        }
    }
    discover_env_catalog_bundled().ok_or_else(|| {
        CoreError::InvalidDoc(
            "Terrain env catalog not found (bundle, dev tree, or npm package)".into(),
        )
    })
}

/// Discover env catalog next to the running executable (Tauri `.app` / npm platform package).
pub fn discover_env_catalog_runtime() -> Option<PathBuf> {
    let home = user_env_catalog_dir();
    if home.join(CATALOG_FILE).is_file() {
        return Some(home);
    }
    discover_env_catalog_bundled()
}

fn discover_env_catalog_bundled() -> Option<PathBuf> {
    discover_next_to_exe().or_else(discover_dev_env_catalog)
}

fn discover_next_to_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    let candidates = [
        parent.join("../env-catalog"),
        parent.join("../Resources/env-catalog"),
        parent.join("Resources/env-catalog"),
        parent.join("resources/env-catalog"),
        parent.join("../resources/env-catalog"),
        parent.join("env-catalog"),
    ];
    for candidate in candidates {
        if candidate.join(CATALOG_FILE).is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn discover_dev_env_catalog() -> Option<PathBuf> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "../../env-catalog",
        "../../../env-catalog",
        "../env-catalog",
        "env-catalog",
    ] {
        let candidate = base.join(rel);
        if candidate.join(CATALOG_FILE).is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn fallback_dev_env_catalog_root() -> PathBuf {
    discover_dev_env_catalog().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../env-catalog")
    })
}

pub fn resolve_skill_source(catalog_root: &Path, item: &IntegrationDef) -> PathBuf {
    if let Some(preset) = &item.preset_skill {
        return crate::preset_skills::preset_skill_dir(preset);
    }
    if let Some(dir) = &item.skill_dir {
        let env_path = catalog_root.join("skills").join(dir);
        if env_path.is_dir() {
            return env_path;
        }
    }
    catalog_root.join("skills")
}

pub fn resolve_fragment_path(catalog_root: &Path, fragment: &str) -> PathBuf {
    catalog_root.join(fragment)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    a.canonicalize()
        .ok()
        .zip(b.canonicalize().ok())
        .map(|(left, right)| left == right)
        .unwrap_or(false)
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
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_discovery_finds_env_catalog() {
        let found = discover_dev_env_catalog();
        assert!(
            found.is_some(),
            "expected env-catalog under Terrain workspace"
        );
        let root = found.unwrap();
        assert!(root.join(CATALOG_FILE).is_file());
    }
}
