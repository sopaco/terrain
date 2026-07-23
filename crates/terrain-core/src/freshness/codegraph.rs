//! Independent, git-based staleness check for the CodeGraph index.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::paths::is_knowledge_output_path;

use super::git::git_output;

/// CodeGraph's own `<cg> status` can report "up to date" while the index is
/// actually behind HEAD. This computes drift purely from `.codegraph/codegraph.db`
/// mtime vs `git log --since`, independent of CodeGraph's own bookkeeping.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodegraphDriftReport {
    pub index_present: bool,
    pub index_synced_at: Option<String>,
    pub commits_after_index: u32,
    pub changed_files: Vec<String>,
    pub likely_stale: bool,
}

pub fn codegraph_drift(repo_path: &str) -> CodegraphDriftReport {
    let repo = Path::new(repo_path);
    let db_path = repo.join(".codegraph/codegraph.db");
    let Ok(meta) = std::fs::metadata(&db_path) else {
        return CodegraphDriftReport {
            index_present: false,
            index_synced_at: None,
            commits_after_index: 0,
            changed_files: Vec::new(),
            likely_stale: false,
        };
    };
    let Ok(modified) = meta.modified() else {
        return CodegraphDriftReport {
            index_present: true,
            index_synced_at: None,
            commits_after_index: 0,
            changed_files: Vec::new(),
            likely_stale: false,
        };
    };
    let synced_at: DateTime<Utc> = modified.into();
    let since = synced_at.to_rfc3339();

    let commit_count = git_output(repo, &["log", "--since", &since, "--pretty=format:%H"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as u32)
        .unwrap_or(0);

    let changed_files: Vec<String> = git_output(
        repo,
        &["log", "--since", &since, "--name-only", "--pretty=format:"],
    )
    .map(|s| {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !is_knowledge_output_path(l))
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .take(20)
            .collect()
    })
    .unwrap_or_default();

    CodegraphDriftReport {
        index_present: true,
        index_synced_at: Some(since),
        commits_after_index: commit_count,
        likely_stale: commit_count > 0 && !changed_files.is_empty(),
        changed_files,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Mutex;

    use super::*;

    static GIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn git_in(repo: &Path, args: &[&str]) {
        let status = crate::process::command("git")
            .args(args)
            .current_dir(repo)
            .status();
        let Ok(status) = status else {
            return;
        };
        assert!(status.success(), "git {:?} failed in {}", args, repo.display());
    }

    fn git_commit_at(repo: &Path, message: &str, when: &str) {
        let status = crate::process::command("git")
            .env("GIT_AUTHOR_DATE", when)
            .env("GIT_COMMITTER_DATE", when)
            .args(["commit", "-m", message])
            .current_dir(repo)
            .status();
        let Ok(status) = status else {
            return;
        };
        assert!(
            status.success(),
            "git commit failed in {}",
            repo.display()
        );
    }

    fn set_file_time_rfc3339(path: &Path, when: &str) {
        let dt = DateTime::parse_from_rfc3339(when)
            .expect("valid RFC3339 test timestamp")
            .with_timezone(&Utc);
        let modified = std::time::UNIX_EPOCH
            + std::time::Duration::from_secs(dt.timestamp().max(0) as u64);
        if let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
            let _ = file.set_modified(modified);
        }
    }

    fn init_test_repo(repo: &Path) {
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/a.rs"), "fn a() {}\n").unwrap();
        git_in(repo, &["init"]);
        git_in(repo, &["config", "user.email", "terrain@test"]);
        git_in(repo, &["config", "user.name", "terrain"]);
        git_in(repo, &["add", "."]);
        git_commit_at(repo, "init", "2020-01-01T00:00:00Z");
    }

    fn require_git() -> bool {
        crate::process::command("git")
            .arg("--version")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }


    #[test]
    fn codegraph_drift_missing_index() {
        let dir = tempfile::tempdir().unwrap();
        let report = codegraph_drift(&dir.path().display().to_string());
        assert!(!report.index_present);
        assert_eq!(report.commits_after_index, 0);
        assert!(!report.likely_stale);
        assert!(report.changed_files.is_empty());
    }

    fn with_git_repo<F>(f: F)
    where
        F: FnOnce(),
    {
        if !require_git() {
            return;
        }
        let _guard = GIT_TEST_LOCK.lock().expect("git test lock");
        f();
    }

    #[test]
    fn codegraph_drift_not_stale_when_index_is_current() {
        with_git_repo(|| {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_test_repo(repo);
        std::fs::create_dir_all(repo.join(".codegraph")).unwrap();
        let db = repo.join(".codegraph/codegraph.db");
        std::fs::write(&db, b"sqlite").unwrap();
        set_file_time_rfc3339(&db, "2020-12-01T00:00:00Z");

        let report = codegraph_drift(&repo.display().to_string());
        assert!(report.index_present);
        assert!(!report.likely_stale);
        assert_eq!(report.commits_after_index, 0);
        });
    }

    #[test]
    fn codegraph_drift_stale_when_source_changes_after_index() {
        with_git_repo(|| {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_test_repo(repo);
        std::fs::create_dir_all(repo.join(".codegraph")).unwrap();
        let db = repo.join(".codegraph/codegraph.db");
        std::fs::write(&db, b"sqlite").unwrap();
        set_file_time_rfc3339(&db, "2020-06-01T00:00:00Z");
        std::fs::write(repo.join("src/b.rs"), "fn b() {}\n").unwrap();
        git_in(repo, &["add", "src/b.rs"]);
        git_commit_at(repo, "add b", "2020-12-01T00:00:00Z");

        let report = codegraph_drift(&repo.display().to_string());
        assert!(report.index_present);
        assert!(report.likely_stale);
        assert!(report.commits_after_index > 0);
        assert!(report.changed_files.iter().any(|p| p == "src/b.rs"));
        });
    }

    #[test]
    fn codegraph_drift_ignores_knowledge_only_commits() {
        with_git_repo(|| {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_test_repo(repo);
        std::fs::create_dir_all(repo.join(".codegraph")).unwrap();
        let db = repo.join(".codegraph/codegraph.db");
        std::fs::write(&db, b"sqlite").unwrap();
        set_file_time_rfc3339(&db, "2020-06-01T00:00:00Z");
        std::fs::create_dir_all(repo.join(".terrain/agent")).unwrap();
        std::fs::write(repo.join(".terrain/agent/context.md"), "# ctx\n").unwrap();
        git_in(repo, &["add", ".terrain/agent/context.md"]);
        git_commit_at(repo, "refresh context", "2020-12-01T00:00:00Z");

        let report = codegraph_drift(&repo.display().to_string());
        assert!(report.index_present);
        assert!(report.commits_after_index > 0);
        assert!(!report.likely_stale);
        assert!(report.changed_files.is_empty());
        });
    }
}
