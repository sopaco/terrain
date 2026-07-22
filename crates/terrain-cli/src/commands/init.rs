use std::path::PathBuf;

use anyhow::Result;
use terrain_agent::{
    resolve_acp_settings, resolve_model_config, run_project_initialization, run_quick_refresh,
};
use terrain_core::KnowledgePaths;

use crate::util::{print_json, require_repo_path, slug_from};

pub async fn run(
    paths: KnowledgePaths,
    cli_repo: Option<PathBuf>,
    repo_path: Option<PathBuf>,
    slug: Option<String>,
) -> Result<()> {
    let repo_path = require_repo_path(cli_repo, repo_path)?;
    let repo = repo_path.display().to_string();
    let slug = slug_from(&repo_path, slug);
    let model_config = resolve_model_config();
    let acp = resolve_acp_settings();

    let result = run_project_initialization(
        &paths,
        &model_config,
        &acp,
        &repo,
        Some(&slug),
        |p| eprintln!("[{}] {}", p.stage, p.message),
        |p| eprintln!("[litho:{}] {}", p.stage, p.message),
    )
    .await?;
    print_json(&result)
}

pub async fn refresh(
    paths: KnowledgePaths,
    cli_repo: Option<PathBuf>,
    repo_path: Option<PathBuf>,
    slug: Option<String>,
) -> Result<()> {
    let repo_path = require_repo_path(cli_repo, repo_path)?;
    let repo = repo_path.display().to_string();
    let slug = slug_from(&repo_path, slug);
    let model_config = resolve_model_config();
    let acp = resolve_acp_settings();

    let result = run_quick_refresh(&paths, &model_config, &acp, &repo, &slug).await?;
    print_json(&result)
}
