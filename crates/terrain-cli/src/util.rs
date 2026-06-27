use std::path::PathBuf;

use anyhow::{Context, Result};
use terrain_core::{get_env_status, KnowledgePaths};

use crate::cli::Cli;

pub fn paths(cli: &Cli) -> KnowledgePaths {
    if let Some(repo) = cli.repo_path.clone() {
        return KnowledgePaths::with_workspace_repo(repo);
    }
    KnowledgePaths::from_workspace()
}

pub fn workspace_project_slug(paths: &KnowledgePaths) -> Option<String> {
    let repo = paths.workspace_repo()?;
    Some(slug_from(&repo.to_path_buf(), None))
}

pub fn require_repo_path(global: Option<PathBuf>, explicit: Option<PathBuf>) -> Result<PathBuf> {
    explicit
        .or(global)
        .or_else(KnowledgePaths::resolve_workspace_repo)
        .context("repository path is required; pass a path, --repo-path, or run inside a Git workspace")
}

pub fn slug_from(repo_path: &PathBuf, slug: Option<String>) -> String {
    slug.unwrap_or_else(|| {
        slug::slugify(
            repo_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project"),
        )
    })
}

pub fn default_env_ids(repo_path: &std::path::Path) -> Result<Vec<String>> {
    let status = get_env_status(repo_path)?;
    Ok(status
        .items
        .iter()
        .filter(|i| i.locked || !i.integrated)
        .map(|i| i.id.clone())
        .collect())
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
