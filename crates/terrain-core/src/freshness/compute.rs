//! Freshness computation and trust-block formatting.

use std::path::Path;

use chrono::Utc;

use crate::assets::{agent_context_ready, agent_pack_ready};
use crate::doc::read_json;
use crate::error::Result;
use crate::paths::KnowledgePaths;
use crate::path_portable::stored_repo_path;
use crate::project::resolve_project_repo_path;
use crate::schema::{
    AgentContextMeta, AgentPackMeta, AssetFreshness, FreshnessLedger, FreshnessSummary, SyncMeta,
};

use super::drift_factors::{build_drift_factors, DriftExplainInput};
use super::git::{git_drift_since, git_snapshot};
use super::ledger::{
    freshness_ledger_still_valid, read_freshness_ledger, write_freshness_ledger,
    LEDGER_VERSION_CONST,
};
use super::scoring::{
    days_since_rfc3339, overall_freshness_score, score_asset, short_git_ref, stale_reason_for,
};
use super::{FRESH_THRESHOLD, MACRO_PRELOAD_THRESHOLD, VERIFY_THRESHOLD};

/// Compute freshness for all knowledge assets and persist `freshness.json`.
pub fn compute_freshness(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<FreshnessSummary> {
    let repo_path = resolve_project_repo_path(paths, project_slug, Some(repo_path))
        .unwrap_or_else(|_| repo_path.to_string());
    let git = git_snapshot(&repo_path);
    let pack_meta = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug)).ok();
    let ctx_meta = read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug)).ok();
    let sync_meta = read_json::<SyncMeta>(paths.sync_meta_path(project_slug)).ok();

    let pack_ready = agent_pack_ready(paths, project_slug);
    let ctx_ready = agent_context_ready(paths, project_slug);

    let pack_baseline = pack_meta
        .as_ref()
        .and_then(|m| m.baseline_git_head.clone());
    let pack_drift = git_drift_since(&repo_path, pack_baseline.as_deref());
    let pack_days = pack_meta
        .as_ref()
        .map(|m| days_since_rfc3339(&m.synced_at))
        .unwrap_or_else(|| {
            sync_meta
                .as_ref()
                .map(|m| days_since_rfc3339(&m.synced_at))
                .unwrap_or(0)
        });
    let pack_total_files = pack_meta.as_ref().map(|m| m.total_files as u32).unwrap_or(0);
    let pack_score = if pack_ready {
        score_asset(
            pack_drift.commits_since_baseline,
            pack_drift.changed_files.len() as u32,
            pack_total_files.max(1),
            pack_days,
            git.dirty,
        )
    } else {
        0
    };

    let ctx_baseline = ctx_meta
        .as_ref()
        .and_then(|m| m.baseline_git_head.clone())
        .or(pack_baseline.clone());
    let ctx_drift = git_drift_since(&repo_path, ctx_baseline.as_deref());
    let ctx_days = ctx_meta
        .as_ref()
        .map(|m| days_since_rfc3339(&m.generated_at))
        .unwrap_or(pack_days);
    let ctx_score_raw = if ctx_ready {
        score_asset(
            ctx_drift.commits_since_baseline,
            ctx_drift.changed_files.len() as u32,
            pack_total_files.max(1),
            ctx_days,
            git.dirty,
        )
    } else {
        0
    };
    let ctx_score = ((ctx_score_raw as f32) * 0.9).min(pack_score as f32) as u8;

    let human_days = sync_meta
        .as_ref()
        .map(|m| days_since_rfc3339(&m.synced_at))
        .unwrap_or(pack_days);
    let human_drift = git_drift_since(
        &repo_path,
        pack_baseline.as_deref().or(git.head.as_deref()),
    );
    let human_score = score_asset(
        human_drift.commits_since_baseline,
        human_drift.changed_files.len() as u32,
        pack_total_files.max(1),
        human_days,
        git.dirty,
    );

    let overall_score = overall_freshness_score(pack_score, ctx_score, human_score);

    let commits_since = pack_drift
        .commits_since_baseline
        .max(ctx_drift.commits_since_baseline);
    let changed_files_count = pack_drift.changed_files.len().max(ctx_drift.changed_files.len()) as u32;

    let overall_stale = overall_score < FRESH_THRESHOLD;
    let stale_reason = if !pack_ready || !ctx_ready {
        Some("asset_not_ready".into())
    } else {
        stale_reason_for(overall_score, commits_since, git.dirty, true)
    };

    let now = Utc::now().to_rfc3339();

    let drift_factors = build_drift_factors(&DriftExplainInput {
        git: &git,
        pack_ready,
        ctx_ready,
        pack_score,
        ctx_score,
        ctx_score_raw,
        human_score,
        overall_score,
        pack_drift: &pack_drift,
        pack_days,
        ctx_days,
        pack_total_files,
        pack_baseline: pack_baseline.as_deref(),
    });

    let sample_changed_files: Vec<String> = pack_drift
        .changed_files
        .iter()
        .take(12)
        .cloned()
        .collect();

    let summary = FreshnessSummary {
        overall_score,
        overall_stale,
        commits_since_baseline: commits_since,
        changed_files_count,
        current_git_head: git.head_short.clone(),
        working_tree_dirty: git.dirty,
        is_git_repo: git.is_git_repo,
        last_computed_at: now.clone(),
        stale_reason: stale_reason.clone(),
        agent_pack_score: pack_score,
        agent_context_score: ctx_score,
        human_docs_score: human_score,
        macro_preload_allowed: ctx_score >= MACRO_PRELOAD_THRESHOLD,
        drift_factors,
        sample_changed_files: sample_changed_files.clone(),
        pack_baseline_short: short_git_ref(pack_baseline.as_deref()),
        context_baseline_short: short_git_ref(
            ctx_meta
                .as_ref()
                .and_then(|m| m.baseline_git_head.as_deref())
                .or(pack_baseline.as_deref()),
        ),
    };

    let ledger = FreshnessLedger {
        version: LEDGER_VERSION_CONST,
        project: project_slug.to_string(),
        repo_path: stored_repo_path(Path::new(&repo_path)),
        baseline: crate::schema::FreshnessBaseline {
            git_head: git.head.clone(),
            git_head_at: now.clone(),
            dirty: git.dirty,
        },
        assets: crate::schema::FreshnessAssets {
            agent_pack: AssetFreshness {
                path: "agent/repomix.md".into(),
                synced_at: pack_meta.as_ref().map(|m| m.synced_at.clone()),
                baseline_git_head: pack_baseline,
                stale: pack_score < FRESH_THRESHOLD,
                stale_reason: stale_reason_for(
                    pack_score,
                    pack_drift.commits_since_baseline,
                    git.dirty,
                    pack_ready,
                ),
                freshness_score: pack_score,
            },
            agent_context: AssetFreshness {
                path: "agent/context.md".into(),
                synced_at: ctx_meta.as_ref().map(|m| m.generated_at.clone()),
                baseline_git_head: ctx_meta
                    .as_ref()
                    .and_then(|m| m.baseline_git_head.clone()),
                stale: ctx_score < FRESH_THRESHOLD,
                stale_reason: stale_reason_for(
                    ctx_score,
                    ctx_drift.commits_since_baseline,
                    git.dirty,
                    ctx_ready,
                ),
                freshness_score: ctx_score,
            },
            human_docs: AssetFreshness {
                path: "human/".into(),
                synced_at: sync_meta.as_ref().map(|m| m.synced_at.clone()),
                baseline_git_head: None,
                stale: human_score < FRESH_THRESHOLD,
                stale_reason: stale_reason_for(
                    human_score,
                    human_drift.commits_since_baseline,
                    git.dirty,
                    true,
                ),
                freshness_score: human_score,
            },
        },
        drift: crate::schema::FreshnessDrift {
            commits_since_baseline: commits_since,
            changed_files_since_baseline: changed_files_count,
            sample_changed_files,
        },
        summary: summary.clone(),
        last_computed_at: now,
    };

    let _ = write_freshness_ledger(paths, project_slug, &ledger);

    Ok(summary)
}

/// Return cached freshness when Git HEAD/dirty and asset timestamps still match the ledger.
pub fn resolve_freshness_summary(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<FreshnessSummary> {
    let repo_path = resolve_project_repo_path(paths, project_slug, Some(repo_path))
        .unwrap_or_else(|_| repo_path.to_string());

    if let Some(ledger) = read_freshness_ledger(paths, project_slug)
        && freshness_ledger_still_valid(paths, project_slug, &ledger, &repo_path) {
            return Ok(ledger.summary);
        }

    compute_freshness(paths, project_slug, &repo_path)
}

/// Format trust rules block for Ask / Agent prompts.
pub fn format_freshness_trust_block(summary: &FreshnessSummary) -> String {
    format!(
        "## Knowledge freshness (MANDATORY — read before answering)\n\
         overall_score: {}/100 · stale: {}\n\
         agent_pack: {}/100 · agent_context: {}/100 · human_docs: {}/100\n\
         commits_since_baseline: {} · changed_files: {}\n\
         current_git_head: {} · working_tree_dirty: {} · is_git_repo: {}\n\
         {stale_line}\
         \n\
         TRUST RULES:\n\
         - freshness_score ≥ {fresh}: treat preloaded architecture context as reliable\n\
         - {verify}–{fresh_minus}: verify architecture claims with grep_agent_pack before answering\n\
         - < {macro_threshold}: DO NOT rely on preloaded macro context; use grep_agent_pack + read_agent_pack_file\n\
         - If stale=true, prefix answers about architecture/modules with a brief ⚠️ outdated-knowledge notice\n\
         - Priority on conflict: repomix source slices > codegraph > agent/context.md > human/\n",
        summary.overall_score,
        summary.overall_stale,
        summary.agent_pack_score,
        summary.agent_context_score,
        summary.human_docs_score,
        summary.commits_since_baseline,
        summary.changed_files_count,
        summary
            .current_git_head
            .as_deref()
            .unwrap_or("(unknown)"),
        summary.working_tree_dirty,
        summary.is_git_repo,
        stale_line = summary
            .stale_reason
            .as_ref()
            .map(|r| format!("stale_reason: {r}\n"))
            .unwrap_or_default(),
        fresh = FRESH_THRESHOLD,
        verify = VERIFY_THRESHOLD,
        fresh_minus = FRESH_THRESHOLD - 1,
        macro_threshold = MACRO_PRELOAD_THRESHOLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FreshnessSummary;

    #[test]
    fn trust_block_contains_thresholds() {
        let summary = FreshnessSummary {
            overall_score: 34,
            overall_stale: true,
            commits_since_baseline: 47,
            changed_files_count: 128,
            current_git_head: Some("a1b2c3d".into()),
            working_tree_dirty: false,
            is_git_repo: true,
            last_computed_at: "now".into(),
            stale_reason: Some("repo_advanced_47_commits".into()),
            agent_pack_score: 40,
            agent_context_score: 34,
            human_docs_score: 50,
            macro_preload_allowed: false,
            drift_factors: vec![],
            sample_changed_files: vec![],
            pack_baseline_short: None,
            context_baseline_short: None,
        };
        let block = format_freshness_trust_block(&summary);
        assert!(block.contains("overall_score: 34"));
        assert!(block.contains("DO NOT rely on preloaded macro context"));
    }
}
