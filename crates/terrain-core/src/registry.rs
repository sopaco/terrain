//! Local registry mapping project slugs to repository paths.
//!
//! Knowledge files live under `{repo}/.terrain/` and are meant to be versioned
//! with the repository. The registry at `~/.terrain/registry.json` only stores
//! pointers so the desktop app can discover indexed repos.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

struct RegistryCache {
    mtime: Option<SystemTime>,
    entries: Vec<RegistryEntry>,
}

static REGISTRY_CACHE: Mutex<Option<RegistryCache>> = Mutex::new(None);

fn invalidate_registry_cache() {
    if let Ok(mut guard) = REGISTRY_CACHE.lock() {
        *guard = None;
    }
}

const REGISTRY_FILE: &str = "registry.json";

/// Serialize registry env mutation in unit/integration tests.
pub fn registry_test_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub slug: String,
    pub repo_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    projects: Vec<RegistryEntry>,
}

pub fn registry_dir() -> PathBuf {
    dirs_home().join(".terrain")
}

fn registry_path() -> PathBuf {
    if let Ok(path) = std::env::var("TERRAIN_REGISTRY_FILE") {
        return PathBuf::from(path);
    }
    registry_dir().join(REGISTRY_FILE)
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn knowledge_root_for_repo(repo_path: &Path) -> PathBuf {
    repo_path.join(".terrain")
}

pub fn load_registry() -> Result<Vec<RegistryEntry>> {
    let path = registry_path();
    if !path.is_file() {
        if let Ok(mut guard) = REGISTRY_CACHE.lock() {
            *guard = Some(RegistryCache {
                mtime: None,
                entries: Vec::new(),
            });
        }
        return Ok(Vec::new());
    }

    let mtime = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    if let Ok(guard) = REGISTRY_CACHE.lock() {
        if let Some(cache) = guard.as_ref() {
            if cache.mtime == mtime {
                return Ok(cache.entries.clone());
            }
        }
    }

    let raw = std::fs::read_to_string(&path)?;
    let file: RegistryFile = serde_json::from_str(&raw)?;
    let entries = file.projects;
    if let Ok(mut guard) = REGISTRY_CACHE.lock() {
        *guard = Some(RegistryCache {
            mtime,
            entries: entries.clone(),
        });
    }
    Ok(entries)
}

/// Slug → repository path map (single registry read via cache).
pub fn registry_repo_map() -> Result<HashMap<String, String>> {
    Ok(load_registry()?
        .into_iter()
        .map(|e| (e.slug, e.repo_path))
        .collect())
}

pub fn save_registry(entries: &[RegistryEntry]) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = RegistryFile {
        projects: entries.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file)?;
    std::fs::write(path, json)?;
    invalidate_registry_cache();
    Ok(())
}

pub fn register_project(slug: &str, repo_path: &str) -> Result<()> {
    let mut entries = load_registry()?;
    let repo = PathBuf::from(repo_path);
    if !repo.is_dir() {
        return Err(CoreError::InvalidDoc(format!(
            "repository path is not a directory: {repo_path}"
        )));
    }

    if let Some(existing) = entries.iter_mut().find(|e| e.slug == slug) {
        existing.repo_path = repo_path.to_string();
    } else {
        entries.push(RegistryEntry {
            slug: slug.to_string(),
            repo_path: repo_path.to_string(),
        });
    }
    entries.sort_by(|a, b| a.slug.cmp(&b.slug));
    save_registry(&entries)?;
    Ok(())
}

/// Remove a project from the local registry only (does not delete the repository or `.terrain/`).
pub fn unregister_project(slug: &str) -> Result<()> {
    let mut entries = load_registry()?;
    let before = entries.len();
    entries.retain(|e| e.slug != slug);
    if entries.len() == before {
        return Err(CoreError::InvalidDoc(format!(
            "project not in registry: {slug}"
        )));
    }
    save_registry(&entries)?;
    Ok(())
}

pub fn knowledge_root_for_slug(slug: &str) -> Option<PathBuf> {
    load_registry()
        .ok()?
        .into_iter()
        .find(|e| e.slug == slug)
        .map(|e| knowledge_root_for_repo(Path::new(&e.repo_path)))
}

pub fn repo_path_for_slug(slug: &str) -> Option<String> {
    registry_repo_map().ok()?.get(slug).cloned()
}

/// Registered projects whose repo `.terrain/index.md` is missing (e.g. data was deleted).
    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleProjectSummary {
    pub slug: String,
    pub repo_path: String,
}

pub fn list_stale_registry_projects() -> Result<Vec<StaleProjectSummary>> {
    let mut stale = Vec::new();
    for entry in load_registry()? {
        let root = knowledge_root_for_repo(Path::new(&entry.repo_path));
        if root.join("index.md").is_file() {
            continue;
        }
        if !Path::new(&entry.repo_path).is_dir() {
            continue;
        }
        stale.push(StaleProjectSummary {
            slug: entry.slug,
            repo_path: entry.repo_path,
        });
    }
    stale.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(stale)
}

/// Slug → knowledge root for all registered projects with an index file.
pub fn indexed_project_roots() -> Result<HashMap<String, PathBuf>> {
    let mut map = HashMap::new();
    for entry in load_registry()? {
        let root = knowledge_root_for_repo(Path::new(&entry.repo_path));
        if root.join("index.md").is_file() {
            map.insert(entry.slug, root);
        }
    }
    Ok(map)
}
