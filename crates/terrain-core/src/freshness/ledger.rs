//! Freshness ledger persistence and cache validation.

use std::path::Path;

use crate::doc::read_json;
use crate::error::Result;
use crate::paths::KnowledgePaths;
use crate::schema::{AgentContextMeta, AgentPackMeta, FreshnessLedger};

use super::git::{git_output, working_tree_dirty_excluding_knowledge};

const LEDGER_VERSION: u32 = 1;

pub(crate) const LEDGER_VERSION_CONST: u32 = LEDGER_VERSION;

pub fn freshness_meta_path(paths: &KnowledgePaths, project_slug: &str) -> std::path::PathBuf {
    paths.freshness_meta_path(project_slug)
}

pub fn read_freshness_ledger(
    paths: &KnowledgePaths,
    project_slug: &str,
) -> Option<FreshnessLedger> {
    read_json(freshness_meta_path(paths, project_slug)).ok()
}

pub fn write_freshness_ledger(
    paths: &KnowledgePaths,
    project_slug: &str,
    ledger: &FreshnessLedger,
) -> Result<()> {
    let path = freshness_meta_path(paths, project_slug);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::doc::write_json(path, ledger)?;
    Ok(())
}

/// Fast validity: HEAD/dirty unchanged and no asset meta newer than the ledger.
pub(crate) fn freshness_ledger_still_valid(
    paths: &KnowledgePaths,
    project_slug: &str,
    ledger: &FreshnessLedger,
    repo_path: &str,
) -> bool {
    let repo = Path::new(repo_path);
    if !repo.join(".git").exists() {
        return false;
    }
    let Some(head) = git_output(repo, &["rev-parse", "HEAD"]) else {
        return false;
    };
    if ledger.baseline.git_head.as_deref() != Some(head.as_str()) {
        return false;
    }
    let dirty = git_output(repo, &["status", "--porcelain"])
        .map(|s| working_tree_dirty_excluding_knowledge(&s))
        .unwrap_or(false);
    if ledger.baseline.dirty != dirty {
        return false;
    }

    let ledger_at = &ledger.last_computed_at;
    if let Ok(pack_meta) = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug))
        && pack_meta.synced_at.as_str() > ledger_at.as_str() {
            return false;
        }
    if let Ok(ctx_meta) = read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug))
        && ctx_meta.generated_at.as_str() > ledger_at.as_str() {
            return false;
        }

    true
}
