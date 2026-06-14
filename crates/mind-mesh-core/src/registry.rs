//! Local registry mapping project slugs to repository paths.
//!
//! Knowledge files live under `{repo}/.mind-mesh/` and are meant to be versioned
//! with the repository. The registry at `~/.mind-mesh/registry.json` only stores
//! pointers so the desktop app can discover indexed repos.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

const REGISTRY_FILE: &str = "registry.json";

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

fn registry_path() -> PathBuf {
    dirs_home().join(".mind-mesh").join(REGISTRY_FILE)
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn knowledge_root_for_repo(repo_path: &Path) -> PathBuf {
    repo_path.join(".mind-mesh")
}

pub fn load_registry() -> Result<Vec<RegistryEntry>> {
    let path = registry_path();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    let file: RegistryFile = serde_json::from_str(&raw)?;
    Ok(file.projects)
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

pub fn knowledge_root_for_slug(slug: &str) -> Option<PathBuf> {
    load_registry()
        .ok()?
        .into_iter()
        .find(|e| e.slug == slug)
        .map(|e| knowledge_root_for_repo(Path::new(&e.repo_path)))
}

pub fn repo_path_for_slug(slug: &str) -> Option<String> {
    load_registry()
        .ok()?
        .into_iter()
        .find(|e| e.slug == slug)
        .map(|e| e.repo_path)
}

/// Registered projects whose repo `.mind-mesh/index.md` is missing (e.g. data was deleted).
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
