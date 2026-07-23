//! Human-readable drift factor explanations for freshness summaries.

use crate::schema::FreshnessDriftFactor;

use super::git::{GitDrift, GitSnapshot};
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
    pub pack_days: u32,
    pub ctx_days: u32,
    pub pack_total_files: u32,
    pub pack_baseline: Option<&'a str>,
}

pub(crate) fn build_drift_factors(input: &DriftExplainInput<'_>) -> Vec<FreshnessDriftFactor> {
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
                severity: if ratio > 0.15 {
                    "high".into()
                } else {
                    "medium".into()
                },
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
            severity: if input.pack_days >= 7 {
                "medium".into()
            } else {
                "low".into()
            },
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
            pack_days: 0,
            ctx_days: 0,
            pack_total_files: 0,
            pack_baseline: None,
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
            pack_days: 1,
            ctx_days: 1,
            pack_total_files: 100,
            pack_baseline: Some("deadbeef"),
        });

        let commits = factors
            .iter()
            .find(|f| f.id == "commits_behind")
            .expect("commits_behind factor");
        assert_eq!(commits.points_lost, Some(10));
        assert!(commits.title.contains('5'));
    }
}
