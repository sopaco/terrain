//! Knowledge freshness ledger — detect drift between Git repo and `.terrain/` assets.

mod codegraph;
mod compute;
mod drift_factors;
mod git;
mod ledger;
mod scoring;

pub use codegraph::{codegraph_drift, CodegraphDriftReport};
pub use compute::{compute_freshness, format_freshness_trust_block, resolve_freshness_summary};
pub use git::{git_drift_since, git_snapshot, GitDrift, GitSnapshot};
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
    use super::git::working_tree_dirty_excluding_knowledge;
    use crate::paths::is_knowledge_output_path;

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
