//! Run AI engineering environment integration for a repository.

use std::path::Path;

use mind_mesh_core::{apply_env_integration, get_env_status, plan_env_integration, EnvApplyProgress, EnvApplyResult, EnvPlan, EnvStatus};

pub fn env_status_for_repo(repo_path: &str) -> anyhow::Result<EnvStatus> {
    get_env_status(Path::new(repo_path)).map_err(Into::into)
}

pub fn env_plan_for_repo(repo_path: &str, selected_ids: &[String]) -> anyhow::Result<EnvPlan> {
    plan_env_integration(Path::new(repo_path), selected_ids).map_err(Into::into)
}

pub async fn run_env_integration(
    repo_path: &str,
    selected_ids: &[String],
    on_progress: impl Fn(EnvApplyProgress),
) -> anyhow::Result<EnvApplyResult> {
    apply_env_integration(Path::new(repo_path), selected_ids, on_progress)
        .await
        .map_err(Into::into)
}
