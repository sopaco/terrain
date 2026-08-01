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
