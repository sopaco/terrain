//! Knowledge freshness ledger — detect drift between Git repo and `.terrain/` assets.

mod codegraph;
mod compute;
mod drift_factors;
mod git;
mod ledger;
mod scoring;

pub use codegraph::{codegraph_drift, CodegraphDriftReport};
pub use compute::{compute_freshness, format_freshness_trust_block, resolve_freshness_summary};
pub use git::{
    baseline_matches_head, git_change_set, git_commit_exists, git_drift_since, git_snapshot,
    DriftBasis, GitChangeSet, GitChangedFile, GitDrift, GitSnapshot,
};
pub use ledger::{freshness_meta_path, read_freshness_ledger, write_freshness_ledger};
pub use scoring::score_asset;

/// Below this score, Ask mode will not preload macro architecture context.
pub const MACRO_PRELOAD_THRESHOLD: u8 = 50;

/// Score at or above this is considered fresh for UI green state.
pub const FRESH_THRESHOLD: u8 = 80;

/// Warn band — verify with repomix before architecture claims.
pub const VERIFY_THRESHOLD: u8 = 70;

#[cfg(test)]
mod tests {
    use super::git::{count_source_commits_in_log, working_tree_dirty_excluding_knowledge};
    use super::scoring::{context_score_from_raw, discount_context_score};
    use crate::paths::is_knowledge_output_path;

    #[test]
    fn knowledge_only_commits_do_not_count_as_source_drift() {
        // Committing regenerated `.terrain/` assets advances HEAD; it must not cost points.
        let knowledge_only =
            "\0ff69643\n\n.terrain/.meta/freshness.json\n.terrain/agent/context.md\n";
        assert_eq!(count_source_commits_in_log(knowledge_only), 0);

        let mixed = "\0aaa\n\n.terrain/agent/context.md\nsrc/main.rs\n\0bbb\n\n.terrain/human/1.md\n";
        assert_eq!(count_source_commits_in_log(mixed), 1);

        let source_only = "\0aaa\n\nsrc/a.rs\nsrc/b.rs\n\0bbb\n\nCargo.toml\n";
        assert_eq!(count_source_commits_in_log(source_only), 2);

        assert_eq!(count_source_commits_in_log(""), 0);
    }

    #[test]
    fn git_drift_ignores_knowledge_only_commits() {
        use std::fs;
        use std::process::Command;

        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .expect("git");
        };
        git(&["init"]);
        git(&["config", "user.email", "t@test.com"]);
        git(&["config", "user.name", "t"]);
        fs::write(repo.join("main.rs"), "fn main() {}\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "init"]);
        let baseline = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(repo)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        // Committing regenerated knowledge assets advances HEAD but must not register as drift.
        fs::create_dir_all(repo.join(".terrain/agent")).unwrap();
        fs::write(repo.join(".terrain/agent/context.md"), "generated\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-m", "update terrain assets"]);

        let repo_str = repo.display().to_string();
        let drift = super::git_drift_since(&repo_str, Some(&baseline));
        assert_eq!(drift.commits_since_baseline, 0);
        assert!(drift.changed_files.is_empty());
        assert_eq!(super::score_asset(0, 0, 24, 0, false), 100);

        // A real source change still counts.
        fs::write(repo.join("main.rs"), "fn main() { /* changed */ }\n").unwrap();
        git(&["commit", "-am", "touch source"]);
        let drift = super::git_drift_since(&repo_str, Some(&baseline));
        assert_eq!(drift.commits_since_baseline, 1);
        assert_eq!(drift.changed_files, vec!["main.rs".to_string()]);
    }

    #[test]
    fn context_discount_caps_layer_at_ninety() {
        assert_eq!(discount_context_score(100), 90);
        assert_eq!(context_score_from_raw(100, 100), 90);
        assert_eq!(context_score_from_raw(98, 100), 88);
        // Never above the source index it derives from.
        assert_eq!(context_score_from_raw(100, 60), 60);
    }

    #[test]
    fn knowledge_output_paths_excluded_from_dirty() {
        assert!(is_knowledge_output_path(".terrain/agent/context.md"));
        assert!(is_knowledge_output_path(".terrain/.meta/freshness.json"));
        assert!(!is_knowledge_output_path("crates/terrain-core/src/lib.rs"));
        assert!(!is_knowledge_output_path("AGENTS.md"));

        let porcelain = " M .terrain/agent/context.md\n M crates/foo.rs\n";
        assert!(working_tree_dirty_excluding_knowledge(porcelain));
        let only_knowledge = " M .terrain/human/1.md\n?? .terrain/.meta/sync.json\n";
        assert!(!working_tree_dirty_excluding_knowledge(only_knowledge));

        // Regression: git_output must not trim() porcelain — first line loses leading status space.
        let corrupted_first_line = "M .terrain/agent/context.md\n";
        assert!(!working_tree_dirty_excluding_knowledge(corrupted_first_line));
    }

    fn init_local_repo(repo: &std::path::Path) {
        use std::fs;
        use std::process::Command;

        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .expect("git")
        };
        git(&["init"]);
        git(&["config", "user.email", "t@test.com"]);
        git(&["config", "user.name", "t"]);
        fs::write(repo.join("main.rs"), "fn main() {}\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "init"]);
    }

    /// A baseline that never existed must read as unmeasurable, not as "nothing changed".
    #[test]
    fn nonexistent_baseline_is_unmeasured_not_in_sync() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_local_repo(repo);

        let repo_str = repo.display().to_string();
        let drift = super::git_drift_since(&repo_str, Some(&"0".repeat(40)));
        assert_eq!(drift.basis, super::git::DriftBasis::BaselineUnreachable);
        assert!(!drift.is_measured());
        assert_eq!(super::scoring::score_layer(&drift, Some(0), 24, false), 25);
    }

    /// A baseline whose commit was rewritten away used to score ~90 while staying `stale: false`
    /// — the assets read as fresh at exactly the moment drift cannot be measured.
    #[test]
    fn rewritten_baseline_is_not_scored_as_fresh() {
        use std::fs;
        use std::process::Command;

        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .expect("git");
            assert!(out.status.success(), "git {args:?}");
            out
        };
        init_local_repo(repo);
        let baseline = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();

        // Rewrite the baseline itself away, then move the code on as a real change would.
        git(&["commit", "--amend", "-m", "init (rewritten)"]);
        fs::write(repo.join("main.rs"), "fn main() { /* changed */ }\n").unwrap();
        git(&["commit", "-am", "touch source"]);
        // A rewritten-away commit stays resolvable until the reflog is expired and it is pruned
        // (a `--depth 1` clone drops it too), so without this the diff below would still succeed
        // and the test would prove nothing.
        git(&["reflog", "expire", "--expire=now", "--all"]);
        git(&["gc", "--prune=now", "-q"]);

        let repo_str = repo.display().to_string();
        assert!(
            !super::git_commit_exists(&repo_str, &baseline),
            "precondition: the recorded baseline must be unreachable"
        );

        let drift = super::git_drift_since(&repo_str, Some(&baseline));
        assert_eq!(drift.basis, super::git::DriftBasis::BaselineUnreachable);
        assert!(!drift.is_measured());
        // Counts are still zero — the point is that zero no longer means "in sync".
        assert_eq!(drift.commits_since_baseline, 0);
        assert!(drift.changed_files.is_empty());

        // Rebuild the pipeline: pack layer, context discount, then the overall minimum.
        let pack_score = super::scoring::score_layer(&drift, Some(0), 24, false);
        let ctx_score = context_score_from_raw(pack_score, pack_score);
        let overall = pack_score.min(ctx_score);
        assert!(overall < super::FRESH_THRESHOLD, "{overall}");
        assert!(overall < super::MACRO_PRELOAD_THRESHOLD, "{overall}");
        assert_eq!(
            super::scoring::stale_reason_for(pack_score, 0, false, true, drift.basis).as_deref(),
            Some("baseline_unreachable")
        );

        // The incremental update path already agreed nothing could be diffed.
        assert!(super::git_change_set(&repo_str, &baseline).is_none());
    }
}
