//! Human-readable drift factor explanations for freshness summaries.

use crate::schema::FreshnessDriftFactor;

use super::git::{GitDrift, GitSnapshot};
use super::scoring::{
    discount_context_score, COMMITS_PENALTY_CAP, COMMITS_PENALTY_PER, CONTEXT_DISCOUNT,
    SYNC_AGE_PENALTY_CAP, SYNC_AGE_PENALTY_PER_DAY,
};
use super::MACRO_PRELOAD_THRESHOLD;

pub(crate) struct DriftExplainInput<'a> {
    pub git: &'a GitSnapshot,
    pub pack_ready: bool,
    pub ctx_ready: bool,
    pub pack_score: u8,
    pub ctx_score: u8,
    pub ctx_score_raw: u8,
    pub human_score: u8,
    pub overall_score: u8,
    pub pack_drift: &'a GitDrift,
    pub ctx_drift: &'a GitDrift,
    pub pack_days: u32,
    pub ctx_days: u32,
    pub pack_total_files: u32,
    pub pack_baseline: Option<&'a str>,
    pub ctx_baseline: Option<&'a str>,
}

/// Points `score_asset` deducts for files changed relative to a baseline.
fn changed_files_penalty(changed: usize, total_files: u32) -> u8 {
    if total_files > 0 {
        ((changed as f64 / total_files as f64).min(1.0) * 30.0).round() as u8
    } else {
        changed.min(30) as u8
    }
}

pub(crate) fn build_drift_factors(input: &DriftExplainInput<'_>) -> Vec<FreshnessDriftFactor> {
    let lang = crate::language::current_language();
    let mut factors = Vec::new();

    if !input.git.is_git_repo {
        factors.push(FreshnessDriftFactor {
            id: "not_git".into(),
            severity: "info".into(),
            title: lang.tr("非 Git 仓库", "Not a Git repository").into(),
            detail: lang
                .tr(
                    "无法对比提交历史，分数主要依据知识资产上次同步至今的天数估算。",
                    "Commit history is unavailable, so the score is mainly estimated from the days since the knowledge assets were last synced.",
                )
                .into(),
            points_lost: None,
        });
    }

    if !input.pack_ready {
        factors.push(FreshnessDriftFactor {
            id: "pack_missing".into(),
            severity: "high".into(),
            title: lang.tr("源码索引尚未生成", "Source index not generated yet").into(),
            detail: lang
                .tr(
                    "缺少 agent/repomix.md，Ask 与 Agent 无法按路径检索最新代码。",
                    "agent/repomix.md is missing; Ask and agents cannot look up the latest code by path.",
                )
                .into(),
            points_lost: None,
        });
    }

    if !input.ctx_ready {
        factors.push(FreshnessDriftFactor {
            id: "context_missing".into(),
            severity: "high".into(),
            title: lang
                .tr("Agent 架构上下文尚未生成", "Agent architecture context not generated yet")
                .into(),
            detail: lang
                .tr(
                    "缺少 agent/context.md，问答将缺少模块地图与系统边界。",
                    "agent/context.md is missing; answers will lack the module map and system boundaries.",
                )
                .into(),
            points_lost: None,
        });
    }

    // The context layer scores against its own baseline, so when that differs from the pack's,
    // its deduction is invisible in the pack-based factors below — it needs its own entry.
    let ctx_baseline_differs = match (input.ctx_baseline, input.pack_baseline) {
        (Some(ctx), Some(pack)) => ctx != pack,
        (Some(_), None) => true,
        _ => false,
    };
    let ctx_commits = input.ctx_drift.commits_since_baseline;
    let ctx_changed = input.ctx_drift.changed_files.len();
    let ctx_drifted = input.ctx_ready && ctx_baseline_differs && (ctx_commits > 0 || ctx_changed > 0);

    if input.git.is_git_repo {
        if input.pack_drift.commits_since_baseline > 0 {
            let lost = (input.pack_drift.commits_since_baseline as i32 * COMMITS_PENALTY_PER)
                .min(COMMITS_PENALTY_CAP) as u8;
            factors.push(FreshnessDriftFactor {
                id: "commits_behind".into(),
                severity: if input.pack_drift.commits_since_baseline >= 10 {
                    "high".into()
                } else {
                    "medium".into()
                },
                title: lang
                    .tr(
                        &format!("代码已前进 {} 个提交", input.pack_drift.commits_since_baseline),
                        &format!(
                            "Code has advanced by {} commits",
                            input.pack_drift.commits_since_baseline
                        ),
                    )
                    .to_string(),
                detail: lang
                    .tr(
                        &format!(
                            "知识资产 baseline 为 {}，当前 HEAD 为 {}。每多 1 个提交约扣 {} 分（上限 {} 分）。",
                            input
                                .pack_baseline
                                .unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                            input.git.head_short.as_deref().unwrap_or("—"),
                            COMMITS_PENALTY_PER,
                            COMMITS_PENALTY_CAP,
                        ),
                        &format!(
                            "Knowledge asset baseline is {}, current HEAD is {}. Each additional commit costs about {} point(s) (capped at {}).",
                            input
                                .pack_baseline
                                .unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                            input.git.head_short.as_deref().unwrap_or("—"),
                            COMMITS_PENALTY_PER,
                            COMMITS_PENALTY_CAP,
                        ),
                    )
                    .to_string(),
                points_lost: Some(lost),
            });
        }

        if input.pack_drift.changed_files.is_empty() && input.pack_drift.commits_since_baseline == 0 {
            if let Some(base) = input.pack_baseline {
                factors.push(FreshnessDriftFactor {
                    id: "baseline_match".into(),
                    severity: "info".into(),
                    title: lang.tr("与 baseline 提交一致", "In sync with the baseline commit").into(),
                    detail: if !ctx_baseline_differs {
                        lang.tr(
                            &format!("源码索引与 Agent 上下文均基于提交 {base} 生成，相对 HEAD 无文件漂移。"),
                            &format!("Both the source index and the Agent context were generated from commit {base}, with no file drift relative to HEAD."),
                        )
                        .to_string()
                    } else if ctx_drifted {
                        lang.tr(
                            &format!("源码索引基于提交 {base} 生成，相对 HEAD 无文件漂移；Agent 上下文基于另一个提交，见下方条目。"),
                            &format!("The source index was generated from commit {base}, with no file drift relative to HEAD; the Agent context is based on another commit — see the entry below."),
                        )
                        .to_string()
                    } else {
                        lang.tr(
                            &format!("源码索引基于提交 {base} 生成，相对 HEAD 无文件漂移；Agent 上下文的 baseline 虽然不同，但两者之间没有改动源码的提交，因此不扣分。"),
                            &format!("The source index was generated from commit {base}, with no file drift relative to HEAD; although the Agent context baseline differs, no source-changing commits sit between them, so no points are deducted."),
                        )
                        .to_string()
                    },
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
            let lost =
                changed_files_penalty(input.pack_drift.changed_files.len(), input.pack_total_files);
            factors.push(FreshnessDriftFactor {
                id: "files_changed".into(),
                severity: if ratio > 0.15 {
                    "high".into()
                } else {
                    "medium".into()
                },
                title: lang
                    .tr(
                        &format!("{count} 个文件相对 baseline 有变更"),
                        &format!("{count} files changed relative to the baseline"),
                    )
                    .to_string(),
                detail: lang
                    .tr(
                        "变更文件占索引规模的比例越高，扣分越多（上限 30 分）。下方列出部分路径。",
                        "The higher the share of changed files in the index, the more points are lost (capped at 30). Some paths are listed below.",
                    )
                    .into(),
                points_lost: Some(lost),
            });
        }

        if ctx_drifted {
            let commit_lost =
                (ctx_commits as i32 * COMMITS_PENALTY_PER).min(COMMITS_PENALTY_CAP) as u8;
            let file_lost = changed_files_penalty(ctx_changed, input.pack_total_files);
            factors.push(FreshnessDriftFactor {
                id: "context_baseline_behind".into(),
                severity: if ctx_commits >= 10 {
                    "high".into()
                } else {
                    "medium".into()
                },
                title: lang
                    .tr(
                        &format!("Agent 上下文的 baseline 落后 {ctx_commits} 个提交"),
                        &format!("The Agent context baseline is {ctx_commits} commits behind"),
                    )
                    .to_string(),
                detail: lang
                    .tr(
                        &format!(
                            "context.md 基于提交 {}，源码索引基于 {}。这一层单独按自己的 baseline 计分：{ctx_commits} 个提交扣 {commit_lost} 分、{ctx_changed} 个变更文件扣 {file_lost} 分。重新生成 Agent 知识资产可消除该差距。",
                            input.ctx_baseline.unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                            input.pack_baseline.unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                        ),
                        &format!(
                            "context.md is based on commit {}, while the source index is based on {}. This layer is scored against its own baseline: {ctx_commits} commits cost {commit_lost} points and {ctx_changed} changed files cost {file_lost} points. Regenerating the Agent knowledge assets closes the gap.",
                            input.ctx_baseline.unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                            input.pack_baseline.unwrap_or(lang.tr("（未记录）", "(not recorded)")),
                        ),
                    )
                    .to_string(),
                points_lost: Some(commit_lost.saturating_add(file_lost)),
            });
        }
    }

    if input.pack_days > 0 {
        let lost = (input.pack_days as i32 * SYNC_AGE_PENALTY_PER_DAY)
            .min(SYNC_AGE_PENALTY_CAP) as u8;
        factors.push(FreshnessDriftFactor {
            id: "pack_age".into(),
            severity: if input.pack_days >= 7 {
                "medium".into()
            } else {
                "low".into()
            },
            title: lang
                .tr(
                    &format!("源码索引已生成 {} 天", input.pack_days),
                    &format!("Source index generated {} days ago", input.pack_days),
                )
                .to_string(),
            detail: lang
                .tr(
                    &format!(
                        "距上次 Repomix 打包越久，额外扣分越多（每天约 {} 分，上限 {} 分）。",
                        SYNC_AGE_PENALTY_PER_DAY, SYNC_AGE_PENALTY_CAP,
                    ),
                    &format!(
                        "The longer since the last Repomix pack, the more extra points are lost (about {} per day, capped at {}).",
                        SYNC_AGE_PENALTY_PER_DAY, SYNC_AGE_PENALTY_CAP,
                    ),
                )
                .to_string(),
            points_lost: if lost > 0 { Some(lost) } else { None },
        });
    }

    if input.ctx_ready && input.ctx_days > input.pack_days {
        factors.push(FreshnessDriftFactor {
            id: "context_older_than_pack".into(),
            severity: "low".into(),
            title: lang
                .tr("Agent 上下文早于源码索引", "Agent context is older than the source index")
                .into(),
            detail: lang
                .tr(
                    &format!(
                        "context.md 已 {} 天未更新，而源码索引为 {} 天前。建议重新生成 Agent 知识资产。",
                        input.ctx_days, input.pack_days
                    ),
                    &format!(
                        "context.md has not been updated for {} days, while the source index is {} days old. Regenerating the Agent knowledge assets is recommended.",
                        input.ctx_days, input.pack_days
                    ),
                )
                .to_string(),
            points_lost: None,
        });
    }

    if input.git.dirty {
        factors.push(FreshnessDriftFactor {
            id: "dirty_tree".into(),
            severity: "medium".into(),
            title: lang.tr("工作区有未提交修改", "Uncommitted changes in the working tree").into(),
            detail: lang
                .tr(
                    "Git 工作区在源码路径上有未提交改动（已排除 `.terrain/` 等知识产出目录）。知识资产基于某次提交快照，与磁盘上的未提交源码改动不一致，扣 5 分。",
                    "The Git working tree has uncommitted changes in source paths (knowledge output directories such as `.terrain/` are excluded). Knowledge assets are snapshots of a commit and do not match the uncommitted source changes on disk, so 5 points are deducted.",
                )
                .into(),
            points_lost: Some(5),
        });
    }

    if input.ctx_ready && input.pack_ready && input.ctx_score_raw > input.ctx_score {
        let discounted = discount_context_score(input.ctx_score_raw);
        let ceiling = discount_context_score(100);
        factors.push(FreshnessDriftFactor {
            id: "context_lineage".into(),
            severity: "info".into(),
            title: lang
                .tr("Agent 上下文按派生资产折算", "Agent context discounted as a derived asset")
                .into(),
            detail: if discounted <= input.pack_score {
                lang.tr(
                    &format!(
                        "context.md 由 LLM 从源码索引推导而来，可信度按 {:.0}% 折算：原始分 {}/100 → {}/100。因此该层上限为 {ceiling} 分，综合分也不会更高。",
                        CONTEXT_DISCOUNT * 100.0,
                        input.ctx_score_raw,
                        input.ctx_score,
                    ),
                    &format!(
                        "context.md is derived from the source index by an LLM, so its confidence is discounted to {:.0}%: raw score {}/100 → {}/100. This layer is therefore capped at {ceiling}, and the overall score cannot be higher either.",
                        CONTEXT_DISCOUNT * 100.0,
                        input.ctx_score_raw,
                        input.ctx_score,
                    ),
                )
                .to_string()
            } else {
                lang.tr(
                    &format!(
                        "context.md 原始分 {}/100 按 {:.0}% 折算为 {discounted}/100，但派生资产不会高于源码索引分数 {}，最终 {}/100。",
                        input.ctx_score_raw,
                        CONTEXT_DISCOUNT * 100.0,
                        input.pack_score,
                        input.ctx_score,
                    ),
                    &format!(
                        "The raw context.md score {}/100 is discounted at {:.0}% to {discounted}/100, but a derived asset cannot outscore the source index ({}), giving {}/100.",
                        input.ctx_score_raw,
                        CONTEXT_DISCOUNT * 100.0,
                        input.pack_score,
                        input.ctx_score,
                    ),
                )
                .to_string()
            },
            points_lost: Some(input.ctx_score_raw.saturating_sub(input.ctx_score)),
        });
    }

    if input.overall_score == input.ctx_score && input.ctx_score <= input.pack_score {
        factors.push(FreshnessDriftFactor {
            id: "overall_driver".into(),
            severity: "info".into(),
            title: lang
                .tr(
                    "总分由 Agent 架构上下文决定",
                    "Overall score is driven by the Agent architecture context",
                )
                .into(),
            detail: lang
                .tr(
                    &format!(
                        "综合分取三层最低值：源码索引 {}、Agent 上下文 {}、人类文档 {}。",
                        input.pack_score, input.ctx_score, input.human_score
                    ),
                    &format!(
                        "The overall score is the lowest of the three layers: source index {}, Agent context {}, human docs {}.",
                        input.pack_score, input.ctx_score, input.human_score
                    ),
                )
                .to_string(),
            points_lost: None,
        });
    } else if input.overall_score == input.pack_score.min(input.human_score) {
        factors.push(FreshnessDriftFactor {
            id: "overall_driver".into(),
            severity: "info".into(),
            title: lang
                .tr("总分由最薄弱的一层决定", "Overall score is driven by the weakest layer")
                .into(),
            detail: lang
                .tr(
                    &format!(
                        "综合分取三层最低值：源码索引 {}、Agent 上下文 {}、人类文档 {}。",
                        input.pack_score, input.ctx_score, input.human_score
                    ),
                    &format!(
                        "The overall score is the lowest of the three layers: source index {}, Agent context {}, human docs {}.",
                        input.pack_score, input.ctx_score, input.human_score
                    ),
                )
                .to_string(),
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
            title: lang.tr("Ask 宏观层预加载", "Ask macro-layer preload").into(),
            detail: if input.ctx_score >= MACRO_PRELOAD_THRESHOLD {
                lang.tr(
                    "分数 ≥ 50：问答会预加载架构概览。",
                    "Score ≥ 50: Q&A preloads the architecture overview.",
                )
                .into()
            } else {
                lang.tr(
                    "分数 < 50：问答不会预加载可能过期的架构概览，需通过源码索引验证。",
                    "Score < 50: Q&A does not preload the potentially stale architecture overview; it must be verified against the source index.",
                )
                .into()
            },
            points_lost: None,
        });
    }

    factors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freshness::git::{GitDrift, GitSnapshot};

    fn sample_git() -> GitSnapshot {
        GitSnapshot {
            head: Some("abc123def456".into()),
            head_short: Some("abc123d".into()),
            dirty: false,
            is_git_repo: true,
        }
    }

    #[test]
    fn drift_factors_include_missing_pack_and_context() {
        let git = sample_git();
        let drift = GitDrift::default();
        let factors = build_drift_factors(&DriftExplainInput {
            git: &git,
            pack_ready: false,
            ctx_ready: false,
            pack_score: 0,
            ctx_score: 0,
            ctx_score_raw: 0,
            human_score: 0,
            overall_score: 0,
            pack_drift: &drift,
            ctx_drift: &drift,
            pack_days: 0,
            ctx_days: 0,
            pack_total_files: 0,
            pack_baseline: None,
            ctx_baseline: None,
        });

        let ids: Vec<_> = factors.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"pack_missing"));
        assert!(ids.contains(&"context_missing"));
        assert!(ids.contains(&"macro_blocked"));
    }

    #[test]
    fn drift_factors_report_commits_behind() {
        let git = sample_git();
        let drift = GitDrift {
            commits_since_baseline: 5,
            changed_files: vec!["src/lib.rs".into()],
        };
        let factors = build_drift_factors(&DriftExplainInput {
            git: &git,
            pack_ready: true,
            ctx_ready: true,
            pack_score: 70,
            ctx_score: 63,
            ctx_score_raw: 70,
            human_score: 80,
            overall_score: 63,
            pack_drift: &drift,
            ctx_drift: &drift,
            pack_days: 1,
            ctx_days: 1,
            pack_total_files: 100,
            pack_baseline: Some("deadbeef"),
            ctx_baseline: Some("deadbeef"),
        });

        let commits = factors
            .iter()
            .find(|f| f.id == "commits_behind")
            .expect("commits_behind factor");
        assert_eq!(commits.points_lost, Some(10));
        assert!(commits.title.contains('5'));
    }

    #[test]
    fn context_baseline_behind_is_explained_when_pack_is_current() {
        let git = sample_git();
        let pack_drift = GitDrift::default();
        let ctx_drift = GitDrift {
            commits_since_baseline: 1,
            changed_files: vec![],
        };
        let factors = build_drift_factors(&DriftExplainInput {
            git: &git,
            pack_ready: true,
            ctx_ready: true,
            pack_score: 100,
            ctx_score: 88,
            ctx_score_raw: 98,
            human_score: 100,
            overall_score: 88,
            pack_drift: &pack_drift,
            ctx_drift: &ctx_drift,
            pack_days: 0,
            ctx_days: 0,
            pack_total_files: 24,
            pack_baseline: Some("abc123def456"),
            ctx_baseline: Some("de04626709e7"),
        });

        // Every point between raw 100 and the reported 88 must be attributable to a factor.
        let behind = factors
            .iter()
            .find(|f| f.id == "context_baseline_behind")
            .expect("context_baseline_behind factor");
        assert_eq!(behind.points_lost, Some(2));
        let lineage = factors
            .iter()
            .find(|f| f.id == "context_lineage")
            .expect("context_lineage factor");
        assert_eq!(lineage.points_lost, Some(10));
        assert!(lineage.detail.contains("90%"), "{}", lineage.detail);

        // The "baselines match" claim must not cover a context layer on a different baseline.
        let matched = factors
            .iter()
            .find(|f| f.id == "baseline_match")
            .expect("baseline_match factor");
        assert!(!matched.detail.contains("均基于"), "{}", matched.detail);
        assert!(matched.detail.contains("见下方条目"), "{}", matched.detail);
    }

    #[test]
    fn different_context_baseline_without_source_drift_is_not_blamed() {
        let git = sample_git();
        // Baselines differ (a knowledge-asset commit moved HEAD) but no source commit sits between.
        let drift = GitDrift::default();
        let factors = build_drift_factors(&DriftExplainInput {
            git: &git,
            pack_ready: true,
            ctx_ready: true,
            pack_score: 100,
            ctx_score: 90,
            ctx_score_raw: 100,
            human_score: 100,
            overall_score: 90,
            pack_drift: &drift,
            ctx_drift: &drift,
            pack_days: 0,
            ctx_days: 0,
            pack_total_files: 24,
            pack_baseline: Some("abc123def456"),
            ctx_baseline: Some("de04626709e7"),
        });

        assert!(!factors.iter().any(|f| f.id == "context_baseline_behind"));
        let matched = factors
            .iter()
            .find(|f| f.id == "baseline_match")
            .expect("baseline_match factor");
        // Must not promise an entry that was never emitted.
        assert!(!matched.detail.contains("见下方条目"), "{}", matched.detail);
        assert!(matched.detail.contains("不扣分"), "{}", matched.detail);
    }
}
