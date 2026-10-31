//! Git panel commands — thin wrappers over terrain-core's gitops module.

use terrain_core::{GitBranches, GitCommit, GitDiff, GitFileEntry, GitOpResult, GitStatus, gitops};

use super::util::validate_repo;

#[tauri::command]
pub async fn git_status_cmd(repo_path: String) -> Result<GitStatus, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::status(&repo))
        .await
        .map_err(|e| format!("git status failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_branches_cmd(repo_path: String) -> Result<GitBranches, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::branches(&repo))
        .await
        .map_err(|e| format!("git branches failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_stage_cmd(
    repo_path: String,
    paths: Option<Vec<String>>,
    all: Option<bool>,
) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    let paths = paths.unwrap_or_default();
    let all = all.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || gitops::stage(&repo, &paths, all))
        .await
        .map_err(|e| format!("git add failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_unstage_cmd(
    repo_path: String,
    paths: Option<Vec<String>>,
    all: Option<bool>,
) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    let paths = paths.unwrap_or_default();
    let all = all.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || gitops::unstage(&repo, &paths, all))
        .await
        .map_err(|e| format!("git restore failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit_cmd(repo_path: String, message: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::commit(&repo, &message))
        .await
        .map_err(|e| format!("git commit failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_merge_cmd(repo_path: String, branch: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::merge(&repo, &branch))
        .await
        .map_err(|e| format!("git merge failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_push_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::push(&repo))
        .await
        .map_err(|e| format!("git push failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_pull_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::pull(&repo))
        .await
        .map_err(|e| format!("git pull failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit_files_cmd(
    repo_path: String,
    summary: String,
    description: Option<String>,
    files: Vec<String>,
) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    let description = description.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        gitops::commit_files(&repo, &summary, &description, &files)
    })
    .await
    .map_err(|e| format!("git commit failed: {e}"))?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_diff_file_cmd(
    repo_path: String,
    path: String,
    staged: Option<bool>,
) -> Result<GitDiff, String> {
    let repo = validate_repo(&repo_path)?;
    let staged = staged.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || gitops::diff_file(&repo, &path, staged))
        .await
        .map_err(|e| format!("git diff failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_history_cmd(
    repo_path: String,
    limit: Option<usize>,
) -> Result<Vec<GitCommit>, String> {
    let repo = validate_repo(&repo_path)?;
    let limit = limit.unwrap_or(50);
    tauri::async_runtime::spawn_blocking(move || gitops::history(&repo, limit))
        .await
        .map_err(|e| format!("git log failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit_files_list_cmd(
    repo_path: String,
    hash: String,
) -> Result<Vec<GitFileEntry>, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::commit_file_list(&repo, &hash))
        .await
        .map_err(|e| format!("git show failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_commit_diff_cmd(
    repo_path: String,
    hash: String,
    path: Option<String>,
) -> Result<GitDiff, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::commit_diff(&repo, &hash, path.as_deref()))
        .await
        .map_err(|e| format!("git show failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_checkout_cmd(repo_path: String, branch: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::checkout(&repo, &branch))
        .await
        .map_err(|e| format!("git checkout failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_fetch_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::fetch(&repo))
        .await
        .map_err(|e| format!("git fetch failed: {e}"))?
        .map_err(|e| e.to_string())
}
