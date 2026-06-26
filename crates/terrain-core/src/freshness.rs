//! Knowledge freshness ledger — detect drift between Git repo and `.terrain/` assets.

use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Utc};

use crate::assets::{agent_context_ready, agent_pack_ready};
use crate::doc::read_json;
use crate::error::Result;
use crate::paths::{is_knowledge_output_path, KnowledgePaths};
use crate::path_portable::stored_repo_path;
use crate::project::resolve_project_repo_path;
use crate::schema::{
    AgentContextMeta, AgentPackMeta, AssetFreshness, FreshnessDriftFactor, FreshnessLedger,
    FreshnessSummary, SyncMeta,
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

/// Overall freshness is the minimum across all three asset layers (including 0 = not ready).
fn overall_freshness_score(pack_score: u8, ctx_score: u8, human_score: u8) -> u8 {
    [pack_score, ctx_score, human_score]
        .into_iter()
        .min()
        .unwrap_or(0)
}

fn short_git_ref(value: Option<&str>) -> Option<String> {
    value.map(|h| {
        let h = h.trim();
        if h.len() <= 7 {
            h.to_string()
        } else {
            h.chars().take(7).collect()
        }
    })
}

#[derive(Debug, Clone)]
struct DriftExplainInput<'a> {
    git: &'a GitSnapshot,
    pack_ready: bool,
    ctx_ready: bool,
    pack_score: u8,
    ctx_score: u8,
    ctx_score_raw: u8,
    human_score: u8,
    overall_score: u8,
    pack_drift: &'a GitDrift,
    pack_days: u32,
    ctx_days: u32,
    pack_total_files: u32,
    pack_baseline: Option<&'a str>,
}

fn build_drift_factors(input: &DriftExplainInput<'_>) -> Vec<FreshnessDriftFactor> {
    let mut factors = Vec::new();

    if !input.git.is_git_repo {
        factors.push(FreshnessDriftFactor {
            id: "not_git".into(),
            severity: "info".into(),
            title: "非 Git 仓库".into(),
            detail: "无法对比提交历史，分数主要依据知识资产上次同步至今的天数估算。".into(),
            points_lost: None,
        });
    }

    if !input.pack_ready {
        factors.push(FreshnessDriftFactor {
            id: "pack_missing".into(),
            severity: "high".into(),
            title: "源码索引尚未生成".into(),
            detail: "缺少 agent/repomix.md，Ask 与 Agent 无法按路径检索最新代码。".into(),
            points_lost: None,
        });
    }

    if !input.ctx_ready {
        factors.push(FreshnessDriftFactor {
            id: "context_missing".into(),
            severity: "high".into(),
            title: "Agent 架构上下文尚未生成".into(),
            detail: "缺少 agent/context.md，问答将缺少模块地图与系统边界。".into(),
            points_lost: None,
        });
    }

    if input.git.is_git_repo {
        if input.pack_drift.commits_since_baseline > 0 {
            let lost = (input.pack_drift.commits_since_baseline as i32 * 2).min(40) as u8;
            factors.push(FreshnessDriftFactor {
                id: "commits_behind".into(),
                severity: if input.pack_drift.commits_since_baseline >= 10 {
                    "high".into()
                } else {
                    "medium".into()
                },
                title: format!(
                    "代码已前进 {} 个提交",
                    input.pack_drift.commits_since_baseline
                ),
                detail: format!(
                    "知识资产 baseline 为 {}，当前 HEAD 为 {}。每多 1 个提交约扣 2 分（上限 40 分）。",
                    input.pack_baseline.unwrap_or("（未记录）"),
                    input.git.head_short.as_deref().unwrap_or("—"),
                ),
                points_lost: Some(lost),
            });
        }

        if input.pack_drift.changed_files.is_empty() && input.pack_drift.commits_since_baseline == 0 {
            if let Some(base) = input.pack_baseline {
                factors.push(FreshnessDriftFactor {
                    id: "baseline_match".into(),
                    severity: "info".into(),
                    title: "与 baseline 提交一致".into(),
                    detail: format!("源码索引与 Agent 上下文均基于提交 {base} 生成，相对 HEAD 无文件漂移。"),
                    points_lost: None,
                });
            }
        } else if !input.pack_drift.changed_files.is_empty() {
            let count = input.pack_drift.changed_files.len() as u32;
            let ratio = if input.pack_total_files > 0 {
                count as f64 / input.pack_total_files as f64
            } else {
                0.0
            };
            let lost = if input.pack_total_files > 0 {
                (ratio * 30.0).round() as u8
            } else {
                count.min(30) as u8
            };
            factors.push(FreshnessDriftFactor {
                id: "files_changed".into(),
                severity: if ratio > 0.15 { "high".into() } else { "medium".into() },
                title: format!("{count} 个文件相对 baseline 有变更"),
                detail: "变更文件占索引规模的比例越高，扣分越多（上限 30 分）。下方列出部分路径。".into(),
                points_lost: Some(lost),
            });
        }
    }

    if input.pack_days > 0 {
        let lost = (input.pack_days as i32 * 2).min(20) as u8;
        factors.push(FreshnessDriftFactor {
            id: "pack_age".into(),
            severity: if input.pack_days >= 7 { "medium".into() } else { "low".into() },
            title: format!("源码索引已生成 {} 天", input.pack_days),
            detail: "距上次 Repomix 打包越久，额外扣分越多（每天约 2 分，上限 20 分）。".into(),
            points_lost: if lost > 0 { Some(lost) } else { None },
        });
    }

    if input.ctx_ready && input.ctx_days > input.pack_days {
        factors.push(FreshnessDriftFactor {
            id: "context_older_than_pack".into(),
            severity: "low".into(),
            title: "Agent 上下文早于源码索引".into(),
            detail: format!(
                "context.md 已 {} 天未更新，而源码索引为 {} 天前。建议重新生成 Agent 知识资产。",
                input.ctx_days, input.pack_days
            ),
            points_lost: None,
        });
    }

    if input.git.dirty {
        factors.push(FreshnessDriftFactor {
            id: "dirty_tree".into(),
            severity: "medium".into(),
            title: "工作区有未提交修改".into(),
            detail: "Git 工作区在源码路径上有未提交改动（已排除 `.terrain/` 等知识产出目录）。知识资产基于某次提交快照，与磁盘上的未提交源码改动不一致，扣 5 分。".into(),
            points_lost: Some(5),
        });
    }

    if input.ctx_ready && input.pack_ready && input.ctx_score_raw > input.ctx_score {
        factors.push(FreshnessDriftFactor {
            id: "context_lineage".into(),
            severity: "info".into(),
            title: "Agent 上下文受源码索引牵连".into(),
            detail: format!(
                "架构上下文原始分 {}/100，按规则不超过源码索引分数的 90%，现为 {}/100。",
                input.ctx_score_raw, input.ctx_score
            ),
            points_lost: Some(input.ctx_score_raw.saturating_sub(input.ctx_score)),
        });
    }

    if input.overall_score == input.ctx_score && input.ctx_score <= input.pack_score {
        factors.push(FreshnessDriftFactor {
            id: "overall_driver".into(),
            severity: "info".into(),
            title: "总分由 Agent 架构上下文决定".into(),
            detail: format!(
                "综合分取三层最低值：源码索引 {}、Agent 上下文 {}、人类文档 {}。",
                input.pack_score, input.ctx_score, input.human_score
            ),
            points_lost: None,
        });
    } else if input.overall_score == input.pack_score.min(input.human_score) {
        factors.push(FreshnessDriftFactor {
            id: "overall_driver".into(),
            severity: "info".into(),
            title: "总分由最薄弱的一层决定".into(),
            detail: format!(
                "综合分取三层最低值：源码索引 {}、Agent 上下文 {}、人类文档 {}。",
                input.pack_score, input.ctx_score, input.human_score
            ),
            points_lost: None,
        });
    }

    if !input.ctx_ready || input.ctx_score < MACRO_PRELOAD_THRESHOLD {
        factors.push(FreshnessDriftFactor {
            id: "macro_blocked".into(),
            severity: if input.ctx_score < MACRO_PRELOAD_THRESHOLD {
                "medium".into()
            } else {
                "info".into()
            },
            title: "Ask 宏观层预加载".into(),
            detail: if input.ctx_score >= MACRO_PRELOAD_THRESHOLD {
                "分数 ≥ 50：问答会预加载架构概览。".into()
            } else {
                "分数 < 50：问答不会预加载可能过期的架构概览，需通过源码索引验证。".into()
            },
            points_lost: None,
        });
    }

    factors
}

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
        version: LEDGER_VERSION,
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
            sample_changed_files,
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

fn porcelain_entry_path(line: &str) -> &str {
    let line = line.trim_end();
    if line.len() < 3 {
        return line.trim();
    }
    // Porcelain v1: XY<space>PATH — do not trim the line start (status columns matter).
    let path_start = if line.as_bytes().get(2) == Some(&b' ') { 3 } else { 2 };
    let rest = line.get(path_start..).unwrap_or(line).trim();
    rest.split_once(" -> ")
        .map(|(_, new_path)| new_path.trim())
        .unwrap_or(rest)
}

/// True when porcelain output contains changes outside generated knowledge paths.
fn working_tree_dirty_excluding_knowledge(porcelain: &str) -> bool {
    porcelain.lines().any(|line| {
        let path = porcelain_entry_path(line);
        !path.is_empty() && !is_knowledge_output_path(path)
    })
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
    let text = String::from_utf8_lossy(&output.stdout);
    // trim_end only — trim() would strip the first porcelain status column (leading space).
    let text = text.trim_end().to_string();
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
    fn overall_score_includes_not_ready_layers() {
        assert_eq!(overall_freshness_score(100, 0, 100), 0);
        assert_eq!(overall_freshness_score(0, 100, 100), 0);
        assert_eq!(overall_freshness_score(100, 80, 100), 80);
        assert_eq!(overall_freshness_score(50, 60, 70), 50);
    }

    #[test]
    fn score_decreases_with_commits_and_age() {
        let fresh = score_asset(0, 0, 100, 0, false);
        assert!(fresh >= 95);

        let stale = score_asset(25, 50, 100, 10, true);
        assert!(stale < FRESH_THRESHOLD);
        assert!(stale < MACRO_PRELOAD_THRESHOLD);
    }

    #[test]
    fn knowledge_output_paths_excluded_from_dirty() {
        use crate::paths::is_knowledge_output_path;

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
