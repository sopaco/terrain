//! Knowledge freshness ledger — detect drift between Git repo and `.terrain/` assets.

mod codegraph;
mod compute;
mod drift_factors;
mod git;
mod ledger;
mod scoring;

pub use codegraph::{codegraph_drift, CodegraphDriftReport};
pub use compute::{compute_freshness, format_freshness_trust_block, resolve_freshness_summary};
pub use git::{baseline_matches_head, git_drift_since, git_snapshot, GitDrift, GitSnapshot};
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
}
