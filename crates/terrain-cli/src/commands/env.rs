use std::path::PathBuf;

use anyhow::Result;
use terrain_core::{apply_env_integration, get_env_status, plan_env_integration};

use crate::cli::EnvCommands;
use crate::util::{default_env_ids, print_json, require_repo_path};

pub async fn run(
    cli_repo: Option<PathBuf>,
    command: EnvCommands,
) -> Result<()> {
    match command {
        EnvCommands::Status { repo_path } => {
            let repo_path = require_repo_path(cli_repo, repo_path)?;
            let status = get_env_status(&repo_path)?;
            print_json(&status)
        }
        EnvCommands::Plan { repo_path, ids } => {
            let repo_path = require_repo_path(cli_repo, repo_path)?;
            let selected = resolve_selected_ids(&repo_path, ids)?;
            let plan = plan_env_integration(&repo_path, &selected, &[])?;
            print_json(&plan)
        }
        EnvCommands::Apply { repo_path, ids } => {
            let repo_path = require_repo_path(cli_repo, repo_path)?;
            let selected = resolve_selected_ids(&repo_path, ids)?;
            let result = apply_env_integration(
                &repo_path,
                &selected,
                &[],
                |p| eprintln!("[{}] {}", p.stage, p.message),
            )
            .await?;
            print_json(&result)
        }
    }
}

fn resolve_selected_ids(
    repo_path: &std::path::Path,
    ids: Option<Vec<String>>,
) -> Result<Vec<String>> {
    match ids {
        Some(v) if !v.is_empty() => Ok(v),
        _ => default_env_ids(repo_path),
    }
}
