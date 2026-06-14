//! Knowledge freshness ledger — detect drift between Git repo and `.mind-mesh/` assets.

use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Utc};

use crate::assets::{agent_context_ready, agent_pack_ready};
use crate::doc::read_json;
use crate::error::Result;
use crate::paths::KnowledgePaths;
use crate::schema::{
    AgentContextMeta, AgentPackMeta, AssetFreshness, FreshnessLedger, FreshnessSummary, SyncMeta,
};

/// Below this score, Ask mode will not preload macro architecture context.
pub const MACRO_PRELOAD_THRESHOLD: u8 = 50;

/// Score at or above this is considered fresh for UI green state.
pub const FRESH_THRESHOLD: u8 = 80;

/// Warn band — verify with repomix before architecture claims.
pub const VERIFY_THRESHOLD: u8 = 70;

const LEDGER_VERSION: u32 = 1;

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
        .map(|s| !s.trim().is_empty())
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

    let count = git_output(
        repo,
        &["rev-list", "--count", &format!("{baseline}..HEAD")],
    )
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0);

    let changed_files = git_output(
        repo,
        &["diff", "--name-only", &format!("{baseline}..HEAD")],
    )
    .map(|s| {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default();

    GitDrift {
        commits_since_baseline: count,
        changed_files,
    }
}

/// Compute asset freshness score (0–100).
pub fn score_asset(
    commits_since: u32,
    changed_files_count: u32,
    total_tracked_estimate: u32,
    days_since_sync: u32,
    working_tree_dirty: bool,
) -> u8 {
    let mut score: i32 = 100;
    score -= (commits_since as i32 * 2).min(40);
    if total_tracked_estimate > 0 {
        let ratio = (changed_files_count as f64 / total_tracked_estimate as f64).min(1.0);
        score -= (ratio * 30.0).round() as i32;
    } else if changed_files_count > 0 {
        score -= (changed_files_count as i32).min(30);
    }
    score -= (days_since_sync as i32 * 2).min(20);
    if working_tree_dirty {
        score -= 5;
    }
    score.clamp(0, 100) as u8
}

fn days_since_rfc3339(value: &str) -> u32 {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| {
            let now = Utc::now();
            let synced = dt.with_timezone(&Utc);
            now.signed_duration_since(synced).num_days().max(0) as u32
        })
        .unwrap_or(0)
}

fn stale_reason_for(score: u8, commits: u32, dirty: bool, ready: bool) -> Option<String> {
    if !ready {
        return Some("asset_not_ready".into());
    }
    if score >= FRESH_THRESHOLD {
        return None;
    }
    if commits > 0 {
        return Some(format!("repo_advanced_{commits}_commits"));
    }
    if dirty {
        return Some("working_tree_dirty".into());
    }
    Some("sync_age".into())
}

/// Compute freshness for all knowledge assets and persist `freshness.json`.
pub fn compute_freshness(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<FreshnessSummary> {
    let git = git_snapshot(repo_path);
    let pack_meta = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug)).ok();
    let ctx_meta = read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug)).ok();
    let sync_meta = read_json::<SyncMeta>(paths.sync_meta_path(project_slug)).ok();

    let pack_ready = agent_pack_ready(paths, project_slug);
    let ctx_ready = agent_context_ready(paths, project_slug);

    let pack_baseline = pack_meta
        .as_ref()
        .and_then(|m| m.baseline_git_head.clone());
    let pack_drift = git_drift_since(repo_path, pack_baseline.as_deref());
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
    let ctx_drift = git_drift_since(repo_path, ctx_baseline.as_deref());
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
        repo_path,
        pack_baseline.as_deref().or(git.head.as_deref()),
    );
    let human_score = score_asset(
        human_drift.commits_since_baseline,
        human_drift.changed_files.len() as u32,
        pack_total_files.max(1),
        human_days,
        git.dirty,
    );

    let overall_score = [pack_score, ctx_score, human_score]
        .into_iter()
        .filter(|&s| s > 0)
        .min()
        .unwrap_or(0);

    let commits_since = pack_drift
        .commits_since_baseline
        .max(ctx_drift.commits_since_baseline);
    let changed_files_count = pack_drift.changed_files.len().max(ctx_drift.changed_files.len()) as u32;

    let overall_stale = overall_score < FRESH_THRESHOLD;
    let stale_reason = stale_reason_for(overall_score, commits_since, git.dirty, pack_ready || ctx_ready);

    let now = Utc::now().to_rfc3339();

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
    };

    let ledger = FreshnessLedger {
        version: LEDGER_VERSION,
        project: project_slug.to_string(),
        repo_path: repo_path.to_string(),
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
                stale_reason: stale_reason_for(pack_score, pack_drift.commits_since_baseline, git.dirty, pack_ready),
                freshness_score: pack_score,
            },
            agent_context: AssetFreshness {
                path: "agent/context.md".into(),
                synced_at: ctx_meta.as_ref().map(|m| m.generated_at.clone()),
                baseline_git_head: ctx_meta
                    .as_ref()
                    .and_then(|m| m.baseline_git_head.clone()),
                stale: ctx_score < FRESH_THRESHOLD,
                stale_reason: stale_reason_for(ctx_score, ctx_drift.commits_since_baseline, git.dirty, ctx_ready),
                freshness_score: ctx_score,
            },
            human_docs: AssetFreshness {
                path: "human/".into(),
                synced_at: sync_meta.as_ref().map(|m| m.synced_at.clone()),
                baseline_git_head: None,
                stale: human_score < FRESH_THRESHOLD,
                stale_reason: stale_reason_for(human_score, human_drift.commits_since_baseline, git.dirty, true),
                freshness_score: human_score,
            },
        },
        drift: crate::schema::FreshnessDrift {
            commits_since_baseline: commits_since,
            changed_files_since_baseline: changed_files_count,
            sample_changed_files: pack_drift
                .changed_files
                .iter()
                .take(12)
                .cloned()
                .collect(),
        },
        summary: summary.clone(),
        last_computed_at: now,
    };

    let _ = write_freshness_ledger(paths, project_slug, &ledger);

    Ok(summary)
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

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_decreases_with_commits_and_age() {
        let fresh = score_asset(0, 0, 100, 0, false);
        assert!(fresh >= 95);

        let stale = score_asset(25, 50, 100, 10, true);
        assert!(stale < FRESH_THRESHOLD);
        assert!(stale < MACRO_PRELOAD_THRESHOLD);
    }

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
        };
        let block = format_freshness_trust_block(&summary);
        assert!(block.contains("overall_score: 34"));
        assert!(block.contains("DO NOT rely on preloaded macro context"));
    }
}
