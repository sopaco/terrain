use std::path::PathBuf;

use anyhow::Result;
use terrain_core::{
    read_doc_at, KnowledgePaths, KnowledgeSearch, ProjectScanner, SearchOptions,
};

use crate::util::{print_json, require_repo_path, workspace_project_slug};

pub async fn list(paths: &KnowledgePaths) -> Result<()> {
    let projects = KnowledgeSearch::new(paths).list_projects()?;
    print_json(&projects)
}

pub async fn scan(
    paths: KnowledgePaths,
    cli_repo: Option<PathBuf>,
    repo_path: Option<PathBuf>,
    slug: Option<String>,
) -> Result<()> {
    let repo_path = require_repo_path(cli_repo, repo_path)?;
    let report = ProjectScanner::new(paths)
        .scan_repo(&repo_path.display().to_string(), slug.as_deref())
        .await?;
    print_json(&report)
}

pub fn search(
    paths: &KnowledgePaths,
    query: &str,
    project: Option<String>,
    limit: usize,
) -> Result<()> {
    let project = project.or_else(|| workspace_project_slug(paths));
    let hits = KnowledgeSearch::new(paths).search(
        query,
        SearchOptions {
            project,
            doc_type: None,
            limit,
        },
    )?;
    print_json(&hits)
}

pub fn read(paths: &KnowledgePaths, path: &str) -> Result<()> {
    let doc = read_doc_at(paths, path)?;
    print_json(&doc)
}
