//! Git plumbing for the app's Git panel — thin, safe wrappers over the git CLI.
//!
//! No libgit2 dependency: Terrain already requires git (freshness, HEAD checks),
//! and the CLI inherits the user's credential helpers for push/pull auth.

use crate::error::{CoreError, Result};
use crate::process;

use serde::Serialize;
use std::path::Path;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitFileEntry {
    pub path: String,
    /// Porcelain staged state ("A", "M", "D", "R"), when present.
    pub staged: Option<String>,
    /// Porcelain unstaged state ("M", "D"), when present.
    pub unstaged: Option<String>,
    pub untracked: bool,
    /// Merge/index conflict detected (any DU/AA/UD-style state).
    pub conflict: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<GitFileEntry>,
    /// True while a merge is in progress (conflict resolution expected).
    pub merging: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitBranches {
    pub current: String,
    pub branches: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitOpResult {
    pub ok: bool,
    pub output: String,
}

/// Resolve the git binary via the login-shell-augmented PATH. GUI apps get a
/// minimal PATH where a Homebrew-only git is invisible.
fn git_binary() -> String {
    crate::shell_path::resolve_executable("git")
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "git".to_owned())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<(bool, String)> {
    let output = process::command(git_binary())
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| CoreError::InvalidDoc(format!("failed to launch git: {e}")))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
    .trim()
    .to_owned();
    Ok((output.status.success(), text))
}

fn require(repo: &Path, args: &[&str], what: &str) -> Result<String> {
    let (ok, text) = run_git(repo, args)?;
    if ok {
        Ok(text)
    } else {
        Err(CoreError::InvalidDoc(format!("{what} failed: {text}")))
    }
}

/// Parse `git status --porcelain=v2 --branch` into panel data.
pub fn status(repo: &Path) -> Result<GitStatus> {
    let text = require(
        repo,
        &["status", "--porcelain=v2", "--branch"],
        "git status",
    )?;
    let mut parsed = GitStatus::default();
    let mut merging = false;
    if let Ok(head_file) = std::fs::read_to_string(repo.join(".git/MERGE_HEAD")) {
        merging = !head_file.trim().is_empty();
    }
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            parsed.branch = rest.trim().to_owned();
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            parsed.upstream = Some(rest.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let mut parts = rest.trim().splitn(2, ' ');
            let ahead = parts.next().unwrap_or("+0").trim_start_matches('+');
            let behind = parts.next().unwrap_or("-0").trim_start_matches('-');
            parsed.ahead = ahead.parse().unwrap_or(0);
            parsed.behind = behind.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            if let Some(state) = parse_ordinary(rest) {
                parsed.entries.push(state);
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // 2 <XY> ... <path><sep><origPath> — rename entries (tab separator)
            if let Some(state) = parse_ordinary(rest) {
                parsed.entries.push(state);
            }
        } else if let Some(rest) = line.strip_prefix("? ") {
            parsed.entries.push(GitFileEntry {
                path: rest.trim().to_owned(),
                staged: None,
                unstaged: None,
                untracked: true,
                conflict: false,
            });
        }
    }
    parsed.merging = merging || parsed.entries.iter().any(|e| e.conflict);
    if parsed.branch == "(detached)" {
        parsed.branch = "HEAD (detached)".to_owned();
    }
    Ok(parsed)
}

/// Parse the XY state of an ordinary/renamed record (8 fields, path last).
fn parse_ordinary(rest: &str) -> Option<GitFileEntry> {
    let fields: Vec<&str> = rest.splitn(8, ' ').collect();
    if fields.len() < 8 {
        return None;
    }
    let xy = fields[0];
    let bytes = xy.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let staged_char = bytes[0] as char;
    let unstaged_char = bytes[1] as char;
    let raw_path = fields[7];
    // Rename records carry "newPath<TAB>origPath" — keep the new path.
    let path = raw_path
        .rsplit('\t')
        .next()
        .unwrap_or(raw_path)
        .trim()
        .to_owned();
    let conflict = matches!(
        (staged_char, unstaged_char),
        ('D', 'U') | ('U', 'U') | ('U', 'D') | ('U', 'A') | ('A', 'U') | ('D', 'D') | ('A', 'A')
    );
    Some(GitFileEntry {
        path,
        staged: if staged_char != '.' && staged_char != '?' {
            Some(staged_char.to_string())
        } else {
            None
        },
        unstaged: if unstaged_char != '.' && unstaged_char != '?' {
            Some(unstaged_char.to_string())
        } else {
            None
        },
        untracked: false,
        conflict,
    })
}

/// Stage paths (or everything with `all`).
pub fn stage(repo: &Path, paths: &[String], all: bool) -> Result<GitOpResult> {
    let mut args: Vec<&str> = vec!["add", "--"];
    if all {
        args.push(".");
    } else if paths.is_empty() {
        return Err(CoreError::InvalidDoc("no paths to stage".to_owned()));
    } else {
        for path in paths {
            args.push(path);
        }
    }
    op(repo, &args, "git add")
}

/// Unstage paths (or everything with `all`).
pub fn unstage(repo: &Path, paths: &[String], all: bool) -> Result<GitOpResult> {
    let mut args: Vec<&str> = vec!["restore", "--staged", "--"];
    if all {
        args.push(".");
    } else if paths.is_empty() {
        return Err(CoreError::InvalidDoc("no paths to unstage".to_owned()));
    } else {
        for path in paths {
            args.push(path);
        }
    }
    op(repo, &args, "git restore")
}

/// Commit staged changes with the given message.
pub fn commit(repo: &Path, message: &str) -> Result<GitOpResult> {
    if message.trim().is_empty() {
        return Err(CoreError::InvalidDoc("empty commit message".to_owned()));
    }
    op(repo, &["commit", "-m", message.trim()], "git commit")
}

/// Local branches (short names) + current branch.
pub fn branches(repo: &Path) -> Result<GitBranches> {
    let out = require(
        repo,
        &["for-each-ref", "refs/heads", "--format=%(refname:short)"],
        "git for-each-ref",
    )?;
    let status_now = status(repo)?;
    Ok(GitBranches {
        current: status_now.branch,
        branches: out
            .lines()
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect(),
    })
}

/// Merge a branch into the current one. On conflict, status reports conflicted entries.
pub fn merge(repo: &Path, branch: &str) -> Result<GitOpResult> {
    if branch.trim().is_empty() {
        return Err(CoreError::InvalidDoc("empty branch name".to_owned()));
    }
    let (_ok, text) = run_git(repo, &["merge", "--no-edit", branch.trim()])?;
    let ok = !(text.contains("CONFLICT") || text.contains("Automatic merge failed"));
    Ok(GitOpResult { ok, output: text })
}

/// What a merge would bring in: the commits `HEAD..<branch>` and the combined
/// shortstat of `HEAD...<branch>` (files changed / insertions / deletions).
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitMergePreview {
    /// The branch being merged in.
    pub branch: String,
    /// Incoming commits (newest first; `body` is empty here).
    pub commits: Vec<GitCommit>,
    /// `git diff --shortstat HEAD...<branch>` text; empty when up to date.
    pub shortstat: String,
}

/// Preview of the commits a merge would apply, mirroring desktop/desktop's
/// merge-branch-dialog (commit list + change-size summary) before confirming.
pub fn merge_preview(repo: &Path, branch: &str) -> Result<GitMergePreview> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(CoreError::InvalidDoc("empty branch name".to_owned()));
    }
    let range = format!("HEAD..{branch}");
    let (ok, text) = run_git(
        repo,
        &[
            "log",
            "-n",
            "100",
            &range,
            "--pretty=format:\u{1e}%H\u{1f}%h\u{1f}%an\u{1f}%ct\u{1f}%s\u{1f}",
        ],
    )?;
    if !ok {
        return Err(CoreError::InvalidDoc(format!(
            "git merge preview failed: {text}"
        )));
    }
    let commits = text
        .split('\u{1e}')
        .filter(|record| !record.trim().is_empty())
        .filter_map(|record| {
            let record = record.trim_end_matches('\n');
            let mut parts = record.splitn(6, '\u{1f}');
            Some(GitCommit {
                hash: parts.next()?.to_owned(),
                short: parts.next()?.to_owned(),
                author: parts.next()?.to_owned(),
                time_secs: parts.next()?.trim().parse().unwrap_or(0),
                summary: parts.next().unwrap_or("").to_owned(),
                body: String::new(),
            })
        })
        .collect();
    let (_ok2, shortstat) = run_git(
        repo,
        &["diff", "--shortstat", &format!("HEAD...{branch}")],
    )?;
    Ok(GitMergePreview {
        branch: branch.to_owned(),
        commits,
        shortstat: shortstat.trim().to_owned(),
    })
}

/// Push current branch (sets upstream on first push).
pub fn push(repo: &Path) -> Result<GitOpResult> {
    let st = status(repo)?;
    let has_upstream = st.upstream.is_some();
    let args: Vec<&str> = if has_upstream {
        vec!["push"]
    } else {
        vec!["push", "--set-upstream", "origin", &st.branch]
    };
    op(repo, &args, "git push")
}

/// Pull with fast-forward only (safe sync); conflicts must be resolved by merge.
pub fn pull(repo: &Path) -> Result<GitOpResult> {
    op(repo, &["pull", "--ff-only"], "git pull")
}

/// Fetch not exposed v1.
fn op(repo: &Path, args: &[&str], what: &str) -> Result<GitOpResult> {
    let (ok, text) = run_git(repo, args)?;
    if ok {
        Ok(GitOpResult {
            ok: true,
            output: text,
        })
    } else {
        Err(CoreError::InvalidDoc(format!("{what} failed: {text}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_repo(name: &str) -> PathBuf {
        let repo = std::env::temp_dir().join(format!("gitops-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            ["init", "-q", "."],
            ["config", "user.email", "t@t"],
            ["config", "user.name", "t"],
            ["config", "commit.gpgsign", "false"],
        ] {
            let _ = crate::process::command("git")
                .args(args)
                .current_dir(&repo)
                .output();
        }
        std::fs::write(repo.join("a.txt"), "hello\n").unwrap();
        let _ = crate::process::command("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output();
        repo
    }

    #[test]
    fn status_stage_commit_roundtrip() {
        let repo = temp_repo("roundtrip");
        std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
        std::fs::write(repo.join("b.txt"), "new\n").unwrap();

        let st = status(&repo).unwrap();
        assert_eq!(st.branch, "main");
        assert!(
            st.entries
                .iter()
                .any(|e| e.path == "a.txt" && e.unstaged.is_some())
        );
        assert!(st.entries.iter().any(|e| e.path == "b.txt" && e.untracked));

        stage(&repo, &["a.txt".into()], false).unwrap();
        let st = status(&repo).unwrap();
        assert!(
            st.entries
                .iter()
                .any(|e| e.path == "a.txt" && e.staged.is_some())
        );

        let res = commit(&repo, "second commit").unwrap();
        assert!(res.ok);
        let st = status(&repo).unwrap();
        assert!(st.entries.iter().all(|e| e.path != "a.txt"));
        assert!(st.entries.iter().any(|e| e.path == "b.txt" && e.untracked));
    }

    #[test]
    fn branches_and_merge_flow() {
        let repo = temp_repo("merge");
        let _ = crate::process::command("git")
            .args(["checkout", "-q", "-b", "feature"])
            .current_dir(&repo)
            .output();
        std::fs::write(repo.join("c.txt"), "f\n").unwrap();
        let _ = crate::process::command("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["commit", "-m", "feature work", "-m", "adds c.txt"])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["checkout", "-q", "main"])
            .current_dir(&repo)
            .output();

        let br = branches(&repo).unwrap();
        assert_eq!(br.current, "main");
        assert!(br.branches.contains(&"feature".to_owned()));

        let res = merge(&repo, "feature").unwrap();
        assert!(res.ok, "merge output: {}", res.output);
        assert!(repo.join("c.txt").exists());
    }

    #[test]
    fn merge_preview_lists_incoming() {
        let repo = temp_repo("preview");
        let _ = crate::process::command("git")
            .args(["checkout", "-q", "-b", "feature"])
            .current_dir(&repo)
            .output();
        std::fs::write(repo.join("c.txt"), "f\n").unwrap();
        let _ = crate::process::command("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["commit", "-q", "-m", "feature work", "-m", "adds c.txt"])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["checkout", "-q", "main"])
            .current_dir(&repo)
            .output();

        let preview = merge_preview(&repo, "feature").unwrap();
        assert_eq!(preview.commits.len(), 1, "{:?}", preview.commits);
        assert!(preview.commits[0].summary.contains("feature work"));
        assert!(preview.shortstat.contains("1 file changed"), "stat: {}", preview.shortstat);

        // Up to date: nothing incoming, empty stat.
        merge(&repo, "feature").unwrap();
        let none = merge_preview(&repo, "feature").unwrap();
        assert!(none.commits.is_empty());
        assert!(none.shortstat.is_empty(), "stat: {}", none.shortstat);
    }

    #[test]
    fn history_keeps_multiline_body() {
        let repo = temp_repo("body");
        std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
        let _ = crate::process::command("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["commit", "-q", "-m", "subject line", "-m", "body para one", "-m", "body para two"])
            .current_dir(&repo)
            .output();

        let log = history(&repo, 5).unwrap();
        let top = log.first().unwrap();
        assert_eq!(top.summary, "subject line");
        assert!(top.body.contains("body para one"), "body: {:?}", top.body);
        assert!(top.body.contains("body para two"), "body: {:?}", top.body);
    }
}

// ---------------------------------------------------------------------------
// GitHub-Desktop-mirrored flows (behavior extracted from desktop/desktop,
// MIT): the panel owns the include set; commits reset the index, stage exactly
// the included files, and commit — never a shared staged area.

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub short: String,
    pub author: String,
    pub time_secs: u64,
    pub summary: String,
    /// Full commit-message body (everything after the subject), trimmed.
    pub body: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GitDiff {
    pub text: String,
    pub truncated: bool,
}

const MAX_DIFF_BYTES: usize = 512 * 1024;

/// Unified patch for one file: `staged` chooses index-vs-HEAD over worktree.
pub fn diff_file(repo: &Path, path: &str, staged: bool) -> Result<GitDiff> {
    let is_untracked = {
        let (ok, listing) = run_git(repo, &["ls-files", "--", path])?;
        ok && listing.trim().is_empty()
    };
    let mut args: Vec<&str> = if staged {
        vec!["diff", "--no-color", "--cached", "--"]
    } else if is_untracked {
        vec!["diff", "--no-color", EMPTY_TREE, "--"]
    } else {
        vec!["diff", "--no-color", "--"]
    };
    args.push(path);
    let (ok, text) = run_git(repo, &args)?;
    if !ok {
        return Err(CoreError::InvalidDoc(format!("git diff failed: {text}")));
    }
    let mut limit = MAX_DIFF_BYTES;
    let mut truncated = false;
    if text.len() > MAX_DIFF_BYTES {
        while !text.is_char_boundary(limit) {
            limit -= 1;
        }
        truncated = true;
        return Ok(GitDiff {
            text: text[..limit].to_owned(),
            truncated,
        });
    }
    Ok(GitDiff { text, truncated })
}

/// Recent commits on the current branch (newest first).
pub fn history(repo: &Path, limit: usize) -> Result<Vec<GitCommit>> {
    let lim = limit.clamp(1, 200).to_string();
    let (ok, text) = run_git(
        repo,
        &[
            "log",
            "-n",
            &lim,
            // The record separator (\u{1e}) keeps multi-line bodies (%b)
            // inside one record; fields are split on \u{1f}.
            "--pretty=format:\u{1e}%H\u{1f}%h\u{1f}%an\u{1f}%ct\u{1f}%s\u{1f}%b",
        ],
    )?;
    if !ok {
        return Err(CoreError::InvalidDoc(format!("git log failed: {text}")));
    }
    let commits = text
        .split('\u{1e}')
        .filter(|record| !record.trim().is_empty())
        .filter_map(|record| {
            let record = record.trim_end_matches('\n');
            let mut parts = record.splitn(6, '\u{1f}');
            Some(GitCommit {
                hash: parts.next()?.to_owned(),
                short: parts.next()?.to_owned(),
                author: parts.next()?.to_owned(),
                time_secs: parts.next()?.trim().parse().unwrap_or(0),
                summary: parts.next().unwrap_or("").to_owned(),
                body: parts.next().unwrap_or("").trim().to_owned(),
            })
        })
        .collect();
    Ok(commits)
}

/// Name-status entries of one commit.
pub fn commit_file_list(repo: &Path, hash: &str) -> Result<Vec<GitFileEntry>> {
    let sep_arg = "--pretty=format:\u{1b}commit\u{1b}%H";
    let (_ok, text) = run_git(
        repo,
        &[
            "show",
            "--name-status",
            "--no-color",
            sep_arg,
            "--no-renames",
            hash,
        ],
    )?;
    let mut entries = Vec::new();
    for line in text.lines().skip(1) {
        let Some((code, path)) = line.split_once('\t') else {
            continue;
        };
        entries.push(GitFileEntry {
            path: path.trim().to_owned(),
            staged: Some(code.trim().to_owned()),
            unstaged: None,
            untracked: false,
            conflict: false,
        });
    }
    Ok(entries)
}

/// Patch of one commit (optionally one file inside it).
pub fn commit_diff(repo: &Path, hash: &str, path: Option<&str>) -> Result<GitDiff> {
    let mut args: Vec<&str> = vec!["show", "--no-color", "--no-renames", "--format="];
    if let Some(path) = path {
        args.push("--");
        args.push(path);
    }
    args.push(hash);
    let (ok, text) = run_git(repo, &args)?;
    if !ok {
        return Err(CoreError::InvalidDoc(format!("git show failed: {text}")));
    }
    let mut end = text.len().min(MAX_DIFF_BYTES);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    Ok(GitDiff {
        text: text[..end].to_owned(),
        truncated: text.len() > MAX_DIFF_BYTES,
    })
}

/// Fetch remotes (refreshes ahead/behind shown by the sync state).
pub fn fetch(repo: &Path) -> Result<GitOpResult> {
    op(repo, &["fetch", "--all", "--quiet"], "git fetch")
}

/// Commit exactly the included files: reset the index, stage the selection,
/// then commit. Mirrors desktop/desktop createCommit (unstageAll → stageFiles).
pub fn commit_files(
    repo: &Path,
    summary: &str,
    description: &str,
    files: &[String],
) -> Result<GitOpResult> {
    op(repo, &["reset", "--mixed", "--quiet"], "git reset")?;
    stage(repo, files, false)?;
    let message = if description.trim().is_empty() {
        summary.trim().to_owned()
    } else {
        format!("{}\n\n{}", summary.trim(), description.trim())
    };
    commit(repo, &message)
}

/// Switch the working tree to another local branch.
pub fn checkout(repo: &Path, branch: &str) -> Result<GitOpResult> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(CoreError::InvalidDoc("empty branch name".to_owned()));
    }
    op(repo, &["checkout", branch, "--quiet"], "git checkout")
}

#[cfg(test)]
mod desktop_mirror_tests {
    use super::*;

    fn temp_repo(name: &str) -> std::path::PathBuf {
        let repo =
            std::env::temp_dir().join(format!("gitops-mirror-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            ["init", "-q", "."],
            ["config", "user.email", "t@t"],
            ["config", "user.name", "t"],
            ["config", "commit.gpgsign", "false"],
        ] {
            let _ = crate::process::command("git")
                .args(args)
                .current_dir(&repo)
                .output();
        }
        std::fs::write(repo.join("a.txt"), "hello\n").unwrap();
        let _ = crate::process::command("git")
            .args(["add", "."])
            .current_dir(&repo)
            .output();
        let _ = crate::process::command("git")
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output();
        repo
    }

    #[test]
    fn commit_files_commits_only_included() {
        let repo = temp_repo("incl");
        std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
        std::fs::write(repo.join("b.txt"), "new\n").unwrap();
        std::fs::write(repo.join("c.txt"), "not included\n").unwrap();

        let res = commit_files(
            &repo,
            "include two",
            "body text",
            &["a.txt".to_owned(), "b.txt".to_owned()],
        )
        .unwrap();
        assert!(res.ok, "{}", res.output);

        // only a.txt + b.txt in the commit; c.txt still pending
        let files = commit_file_list(&repo, "HEAD").unwrap();
        let committed: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(committed.contains(&"a.txt") && committed.contains(&"b.txt"));
        assert!(!committed.contains(&"c.txt"));

        let st = status(&repo).unwrap();
        assert!(st.entries.iter().any(|e| e.path == "c.txt" && e.untracked));

        // history sees the new commit on top
        let log = history(&repo, 5).unwrap();
        assert_eq!(log.first().map(|c| c.summary.as_str()), Some("include two"));

        // diff of a new unstaged change
        std::fs::write(repo.join("a.txt"), "again\n").unwrap();
        let diff = diff_file(&repo, "a.txt", false).unwrap();
        assert!(diff.text.contains("+again"), "diff: {}", diff.text);
        assert!(!diff.text.contains("+changed"));
    }
}

// ---------------------------------------------------------------------------
// v3 operations: stash, discard, undo-commit, create-branch; untracked diffs
// via the constant empty-tree hash (platform-independent).

/// Git's well-known empty tree object id — same in every repository.
pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// Stash all changes (tracked + untracked) with a marker message.
pub fn stash_push(repo: &Path) -> Result<GitOpResult> {
    op(
        repo,
        &[
            "stash",
            "push",
            "--include-untracked",
            "--quiet",
            "-m",
            "terrain stash",
        ],
        "git stash",
    )
}

/// Pop the most recent stash back onto the working tree.
pub fn stash_pop(repo: &Path) -> Result<GitOpResult> {
    op(repo, &["stash", "pop", "--quiet"], "git stash pop")
}

/// How many stashes exist (0 = none).
pub fn stash_count(repo: &Path) -> usize {
    let (_ok, text) = run_git(repo, &["stash", "list", "--quiet"]).unwrap_or((true, String::new()));
    text.lines().filter(|l| !l.trim().is_empty()).count()
}

/// Discard one file's working-tree changes. Tracked files are restored from
/// HEAD; untracked files are deleted (they live nowhere else).
pub fn discard_file(repo: &Path, path: &str) -> Result<GitOpResult> {
    let path = path.trim().to_owned();
    if path.is_empty() {
        return Err(CoreError::InvalidDoc("empty path".to_owned()));
    }
    // Untracked = not in the index/HEAD: deleting is the discard.
    let (ok, _text) = run_git(repo, &["ls-files", "--", &path])?;
    if !ok || _text.trim().is_empty() {
        let target = repo.join(&path);
        if target.is_file() {
            std::fs::remove_file(&target)?;
            return Ok(GitOpResult {
                ok: true,
                output: format!("removed untracked {path}"),
            });
        }
        return Err(CoreError::InvalidDoc(format!(
            "file not tracked and not present: {path}"
        )));
    }
    op(repo, &["restore", "--", &path], "git restore")
}

/// Undo the last commit, keeping its changes in the working tree.
pub fn undo_last_commit(repo: &Path) -> Result<GitOpResult> {
    op(
        repo,
        &["reset", "--mixed", "--quiet", "HEAD~1"],
        "git reset HEAD~1",
    )
}

/// Create a new branch from the current HEAD and switch to it.
pub fn create_branch(repo: &Path, name: &str) -> Result<GitOpResult> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::InvalidDoc("empty branch name".to_owned()));
    }
    op(
        repo,
        &["checkout", "-b", name, "--quiet"],
        "git checkout -b",
    )
}

#[cfg(test)]
mod v3_tests {
    use super::*;

    fn temp_repo(name: &str) -> std::path::PathBuf {
        let repo = std::env::temp_dir().join(format!("gitops-v3-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        for args in [
            ["init", "-q", "."],
            ["config", "user.email", "t@t"],
            ["config", "user.name", "t"],
            ["config", "commit.gpgsign", "false"],
        ] {
            let _ = crate::process::command("git").args(args).current_dir(&repo).output();
        }
        std::fs::write(repo.join("a.txt"), "hello\n").unwrap();
        let _ = crate::process::command("git").args(["add", "."]).current_dir(&repo).output();
        let _ = crate::process::command("git").args(["commit", "-m", "init"]).current_dir(&repo).output();
        std::fs::write(repo.join("b.txt"), "second\n").unwrap();
        let _ = crate::process::command("git").args(["add", "."]).current_dir(&repo).output();
        let _ = crate::process::command("git").args(["commit", "-m", "second"]).current_dir(&repo).output();
        repo
    }

    #[test]
    fn stash_and_undo_roundtrip() {
        let repo = temp_repo("stash");
        std::fs::write(repo.join("a.txt"), "dirty\n").unwrap();
        stash_push(&repo).unwrap();
        assert_eq!(stash_count(&repo), 1);
        let st = status(&repo).unwrap();
        assert!(st.entries.is_empty(), "worktree clean after stash");
        stash_pop(&repo).unwrap();
        assert_eq!(stash_count(&repo), 0);
        let st = status(&repo).unwrap();
        assert!(st.entries.iter().any(|e| e.unstaged == Some("M".to_owned())));

        undo_last_commit(&repo).unwrap();
        let st = status(&repo).unwrap();
        // reset --mixed: the undone commit's file returns as untracked
        assert!(st.entries.iter().any(|e| e.path == "b.txt" && e.untracked));
    }

    #[test]
    fn discard_and_branch_and_untracked_diff() {
        let repo = temp_repo("discard");
        std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
        std::fs::write(repo.join("untracked.txt"), "new\n").unwrap();

        discard_file(&repo, "a.txt").unwrap();
        let st = status(&repo).unwrap();
        assert!(!st.entries.iter().any(|e| e.path == "a.txt"));

        discard_file(&repo, "untracked.txt").unwrap();
        assert!(!repo.join("untracked.txt").exists());

        let diff = diff_file(&repo, "a.txt", false).unwrap();
        assert!(diff.text.is_empty());

        create_branch(&repo, "terrain-v3-test").unwrap();
        let st = status(&repo).unwrap();
        assert_eq!(st.branch, "terrain-v3-test");
    }
}
