//! Freshness ledger persistence and cache validation.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::assets::agent_context_ready;
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

    let ctx_path = paths.agent_context_main(project_slug);
    if ctx_path.is_file()
        && let Ok(meta) = std::fs::metadata(&ctx_path)
        && let Ok(modified) = meta.modified()
    {
        let modified_at = DateTime::<Utc>::from(modified).to_rfc3339();
        if modified_at.as_str() > ledger_at.as_str() {
            return false;
        }
    }

    // `context-meta.json` can lag behind a restored or hand-edited `context.md`.
    let ctx_ready_now = agent_context_ready(paths, project_slug);
    let ctx_cached_not_ready = ledger
        .assets
        .agent_context
        .stale_reason
        .as_deref()
        == Some("asset_not_ready");
    if ctx_ready_now != !ctx_cached_not_ready {
        return false;
    }

    true
}
