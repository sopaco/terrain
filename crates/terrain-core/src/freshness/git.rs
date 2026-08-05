//! Git snapshot and drift helpers for freshness scoring.

use std::path::Path;

use crate::paths::is_knowledge_output_path;

/// Git snapshot for a repository at computation time.
#[derive(Debug, Clone)]
pub struct GitSnapshot {
    pub head: Option<String>,
    pub head_short: Option<String>,
    pub dirty: bool,
    pub is_git_repo: bool,
}

/// Drift between a stored baseline commit and current HEAD.
#[derive(Debug, Clone, Default)]
pub struct GitDrift {
    pub commits_since_baseline: u32,
    pub changed_files: Vec<String>,
}

/// One changed path between a baseline commit and the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChangedFile {
    /// `git diff --name-status` letter (`A`/`M`/`D`/`R`…), or `W` for worktree-only changes.
    pub status: String,
    pub path: String,
}

/// Everything an incremental update needs to know about `baseline..HEAD` plus the worktree.
#[derive(Debug, Clone, Default)]
pub struct GitChangeSet {
    pub changed: Vec<GitChangedFile>,
    /// `<short-sha> <subject>` lines, newest first (capped).
    pub commit_log: Vec<String>,
    /// `git diff --stat` output for `baseline..HEAD`.
    pub diff_stat: String,
    /// Uncommitted source paths (working tree + index), excluding `.terrain/`.
    pub dirty_paths: Vec<String>,
}

/// How many commit subjects to carry into an incremental update prompt.
const MAX_COMMIT_LOG_LINES: usize = 40;

/// True when `baseline` matches current HEAD (or repo is not Git).
pub fn baseline_matches_head(repo_path: &str, baseline: Option<&str>) -> bool {
    let git = git_snapshot(repo_path);
    if !git.is_git_repo {
        return true;
    }
    match (baseline, git.head.as_deref()) {
        (Some(baseline), Some(head)) => baseline == head,
        _ => false,
    }
}

/// Capture current Git HEAD for a repository (best-effort).
pub fn git_snapshot(repo_path: &str) -> GitSnapshot {
    let repo = Path::new(repo_path);
    if !repo.join(".git").exists() {
        return GitSnapshot {
            head: None,
            head_short: None,
            dirty: false,
            is_git_repo: false,
        };
    }

    let head = git_output(repo, &["rev-parse", "HEAD"]);
    let head_short = git_output(repo, &["rev-parse", "--short", "HEAD"]);
    let dirty = git_output(repo, &["status", "--porcelain"])
        .map(|s| working_tree_dirty_excluding_knowledge(&s))
        .unwrap_or(false);

    GitSnapshot {
        head,
        head_short,
        dirty,
        is_git_repo: true,
    }
}

/// Drift from `baseline` commit to current HEAD (empty when not a git repo or baseline missing).
pub fn git_drift_since(repo_path: &str, baseline: Option<&str>) -> GitDrift {
    let Some(baseline) = baseline.filter(|b| !b.is_empty()) else {
        return GitDrift::default();
    };
    let repo = Path::new(repo_path);
    if !repo.join(".git").exists() {
        return GitDrift::default();
    }

    let count = source_commits_since(repo, baseline);

    let changed_files = git_output(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-only",
            &format!("{baseline}..HEAD"),
        ],
    )
    .map(|s| {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !is_knowledge_output_path(l))
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default();

    GitDrift {
        commits_since_baseline: count,
        changed_files,
    }
}

/// True when `commitish` resolves to a commit object in `repo`.
///
/// A baseline recorded before a rebase, squash or shallow re-clone is unreachable; diffing
/// against it silently returns nothing, which would otherwise read as "no changes".
pub fn git_commit_exists(repo_path: &str, commitish: &str) -> bool {
    let repo = Path::new(repo_path);
    if commitish.is_empty() || !repo.join(".git").exists() {
        return false;
    }
    git_output(repo, &["rev-parse", "--verify", &format!("{commitish}^{{commit}}")]).is_some()
}

/// Collect committed and uncommitted changes since `baseline` (knowledge outputs filtered out).
///
/// Returns `None` when `repo_path` is not a Git repo or `baseline` is unreachable — callers
/// must fall back to a full regeneration rather than assume an empty change set.
pub fn git_change_set(repo_path: &str, baseline: &str) -> Option<GitChangeSet> {
    if !git_commit_exists(repo_path, baseline) {
        return None;
    }
    let repo = Path::new(repo_path);
    let range = format!("{baseline}..HEAD");

    let changed = git_output(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-status",
            "-M",
            &range,
        ],
    )
    .map(|out| parse_name_status(&out))
    .unwrap_or_default();

    let commit_log = git_output(
        repo,
        &[
            "log",
            "--no-merges",
            "--format=%h %s",
            &format!("--max-count={MAX_COMMIT_LOG_LINES}"),
            &range,
        ],
    )
    .map(|out| {
        out.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default();

    let diff_stat = git_output(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--stat",
            "--stat-width=100",
            &range,
        ],
    )
    .unwrap_or_default();

    let dirty_paths = git_output(repo, &["-c", "core.quotepath=false", "status", "--porcelain"])
        .map(|out| {
            out.lines()
                .map(porcelain_entry_path)
                .filter(|p| !p.is_empty() && !is_knowledge_output_path(p))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Some(GitChangeSet {
        changed,
        commit_log,
        diff_stat,
        dirty_paths,
    })
}

/// Parse `git diff --name-status -M` output, dropping generated knowledge paths.
///
/// Rename entries carry two paths (`R100\told\tnew`); the destination is what matters for docs.
fn parse_name_status(out: &str) -> Vec<GitChangedFile> {
    out.lines()
        .filter_map(|line| {
            let mut cols = line.split('\t');
            let status = cols.next()?.trim();
            let paths: Vec<&str> = cols.map(str::trim).filter(|p| !p.is_empty()).collect();
            let path = paths.last()?;
            if status.is_empty() || path.is_empty() || is_knowledge_output_path(path) {
                return None;
            }
            Some(GitChangedFile {
                status: status.to_string(),
                path: path.to_string(),
            })
        })
        .collect()
}

/// Count commits in `baseline..HEAD` that touch at least one non-knowledge path.
///
/// A plain `rev-list --count` also counts commits that only rewrite `.terrain/` — so
/// committing regenerated knowledge assets advances HEAD and immediately penalizes the
/// very assets that commit refreshed (`changed_files` filters those paths out, leaving a
/// deduction with no visible cause). Merge commits show no paths under `--name-only` and
/// are skipped; the commits they bring in are counted individually when in range.
fn source_commits_since(repo: &Path, baseline: &str) -> u32 {
    let Some(log) = git_output(
        repo,
        &[
            "-c",
            "core.quotepath=false",
            "log",
            // NUL prefix marks commit boundaries so a path can never be read as a hash.
            "--format=%x00%H",
            "--name-only",
            &format!("{baseline}..HEAD"),
        ],
    ) else {
        return 0;
    };
    count_source_commits_in_log(&log)
}

/// Count commits in `git log --format=%x00%H --name-only` output that touch a non-knowledge path.
pub(crate) fn count_source_commits_in_log(log: &str) -> u32 {
    let mut count = 0u32;
    let mut current_counted = false;
    for line in log.lines() {
        if line.starts_with('\0') {
            current_counted = false;
            continue;
        }
        let path = line.trim();
        if current_counted || path.is_empty() || is_knowledge_output_path(path) {
            continue;
        }
        count += 1;
        current_counted = true;
    }
    count
}

pub(crate) fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = crate::process::command("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // trim_end only — trim() would strip the first porcelain status column (leading space).
    let text = text.trim_end().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn porcelain_entry_path(line: &str) -> &str {
    let line = line.trim_end();
    if line.len() < 3 {
        return line.trim();
    }
    // Porcelain v1: XY<space>PATH — do not trim the line start (status columns matter).
    let path_start = if line.as_bytes().get(2) == Some(&b' ') {
        3
    } else {
        2
    };
    let rest = line.get(path_start..).unwrap_or(line).trim();
    rest.split_once(" -> ")
        .map(|(_, new_path)| new_path.trim())
        .unwrap_or(rest)
}

/// True when porcelain output contains changes outside generated knowledge paths.
pub(crate) fn working_tree_dirty_excluding_knowledge(porcelain: &str) -> bool {
    porcelain.lines().any(|line| {
        let path = porcelain_entry_path(line);
        !path.is_empty() && !is_knowledge_output_path(path)
    })
}
