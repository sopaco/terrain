//! Git panel commands — thin wrappers over terrain-core's gitops module.

use serde::Serialize;
use tauri::State;
use terrain_agent::ChatEngine;

use terrain_core::{
    ChatPhase, ChatTokenUsage, ChatToolCallRecord, GitBranches, GitCommit, GitDiff, GitFileEntry,
    GitMergePreview, GitOpResult, GitStatus, gitops,
};

use super::util::validate_repo;
use crate::AppState;

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
pub async fn git_merge_preview_cmd(
    repo_path: String,
    branch: String,
) -> Result<GitMergePreview, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::merge_preview(&repo, &branch))
        .await
        .map_err(|e| format!("git merge preview failed: {e}"))?
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

// ---------------------------------------------------------------------------
// AI commit message: one native-LLM shot over the included files' diffs.
// The "agent-ctx-" session prefix routes the prompt ungrounded (raw), which
// is what we want — the diffs ARE the context.

#[derive(Debug, Serialize)]
pub struct GeneratedCommitMessage {
    pub summary: String,
    pub description: String,
}

const COMMIT_PROMPT: &str = "\
Generate a git commit message for the following diffs.

Rules:
- FIRST LINE: the summary. Max 72 characters. Imperative mood (\"Fix authentication bug\"). No quotes, no prefix like commit:/feat: unless obviously conventional.
- THEN an empty line.
- THEN 2-4 sentences of description explaining what changed and why.
- Respond with the message ONLY. No code fences, no commentary.
";

fn collect_included_diffs(repo: &std::path::Path, files: &[String], cap: usize) -> String {
    use terrain_core::gitops;
    let mut out = String::new();
    for file in files {
        if out.len() >= cap {
            break;
        }
        if let Ok(diff) = gitops::diff_file(repo, file, false) {
            out.push_str(&diff.text);
            out.push('\n');
        }
    }
    if out.len() > cap {
        let mut end = cap;
        while end > 0 && !out.is_char_boundary(end) {
            end -= 1;
        }
        out.truncate(end);
        out.push_str("\n… (diff truncated)\n");
    }
    out
}

fn parse_commit_message(answer: &str) -> GeneratedCommitMessage {
    let cleaned = answer
        .trim()
        .trim_start_matches("```")
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let mut lines = cleaned.split_inclusive("\n").peekable();
    // skip a leading fence or blank lines before the summary
    let mut summary = String::new();
    let rest_start = {
        let mut idx = 0usize;
        for (i, line) in cleaned.split_inclusive("\n").enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "```" {
                idx = i + 1;
                continue;
            }
            summary = trimmed.to_owned();
            idx = i + 1;
            break;
        }
        idx
    };
    if summary.chars().count() > 72 {
        summary = summary.chars().take(72).collect();
    }
    let description: String = cleaned
        .split_inclusive("\n")
        .skip(rest_start)
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_owned();
    let description = description.trim_matches('`').trim().to_owned();
    GeneratedCommitMessage {
        summary,
        description,
    }
}

#[tauri::command]
pub async fn git_generate_commit_message_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    files: Vec<String>,
) -> Result<GeneratedCommitMessage, String> {
    if files.is_empty() {
        return Err("nothing included to generate a message from".to_owned());
    }
    let repo = validate_repo(&repo_path)?;
    let paths = state.paths().clone();
    let model_config = state.model_config();
    let diffs = tauri::async_runtime::spawn_blocking(move || {
        collect_included_diffs(&repo, &files, 20 * 1024)
    })
    .await
    .map_err(|e| format!("diff collection failed: {e}"))?;
    if diffs.trim().is_empty() {
        return Err("included files have no diffs (are they untracked-empty?)".to_owned());
    }

    let engine = terrain_agent::ChatEngine::new_native(paths, model_config)
        .map_err(|e| format!("LLM not available: {e}"))?;
    let prompt = format!("{COMMIT_PROMPT}\n{diffs}");
    let session_id = format!("agent-ctx-commit-{}", chrono_ts());
    let reply = engine
        .ask(
            &session_id,
            &prompt,
            None,
            None,
            |_: &str| {},
            |_: &str| {},
            |_: &[terrain_core::ChatToolCallRecord]| {},
            |_: terrain_core::ChatPhase| {},
            |_: &terrain_core::ChatTokenUsage| {},
        )
        .await
        .map_err(|e| format!("LLM generation failed: {e}"))?;

    Ok(parse_commit_message(&reply.answer))
}

fn chrono_ts() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[tauri::command]
pub async fn git_discard_file_cmd(repo_path: String, path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::discard_file(&repo, &path))
        .await
        .map_err(|e| format!("git discard failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_undo_last_commit_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::undo_last_commit(&repo))
        .await
        .map_err(|e| format!("git undo failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_stash_push_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::stash_push(&repo))
        .await
        .map_err(|e| format!("git stash failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_stash_pop_cmd(repo_path: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::stash_pop(&repo))
        .await
        .map_err(|e| format!("git stash pop failed: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_create_branch_cmd(repo_path: String, name: String) -> Result<GitOpResult, String> {
    let repo = validate_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || gitops::create_branch(&repo, &name))
        .await
        .map_err(|e| format!("git branch failed: {e}"))?
        .map_err(|e| e.to_string())
}
