//! Env integration planning.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::super::catalog::{load_catalog, IntegrationDef};
use super::cache::{env_cache_fingerprint, env_cache_get, env_cache_key, env_cache_put};
use super::probe::{
    bundled_tool_runtime_ready, check_integration, dependency_ready, integration_is_ready,
    tool_check_passes,
};
use super::types::{EnvPlan, EnvPlanStep, EnvStatus};
use crate::error::Result;
use crate::schema::AgentEnvStatus;

use super::super::agents_md::agents_md_ready;

pub fn get_env_status(repo: &Path) -> Result<EnvStatus> {
    let key = env_cache_key(repo)?;
    let fingerprint = env_cache_fingerprint(repo);
    if let Some(status) = env_cache_get(&key, fingerprint) {
        return Ok(status);
    }

    let status = compute_env_status(repo)?;
    env_cache_put(key, fingerprint, status.clone());
    Ok(status)
}

/// Build a concise integration-status phrase for the overview card.
///
/// The big `integrated_count/total_count` number already carries the ratio,
/// so this must NOT repeat it — it only describes state in words.
fn env_integration_summary(ready_count: usize, total: usize) -> String {
    let lang = crate::language::current_language();
    if total == 0 {
        lang.tr("未检测", "Not detected").to_string()
    } else if ready_count == total {
        lang.tr("全部已集成", "Fully integrated").to_string()
    } else if ready_count == 0 {
        lang.tr("尚未集成", "Not integrated yet").to_string()
    } else {
        lang.tr("部分已集成", "Partially integrated").to_string()
    }
}

fn compute_env_status(repo: &Path) -> Result<EnvStatus> {
    let catalog = load_catalog()?;
    let items: Vec<_> = catalog
        .integrations
        .iter()
        .map(|def| check_integration(repo, def))
        .collect::<Result<_>>()?;

    let ready_count = items.iter().filter(|i| i.integrated).count();
    let total = items.len();
    let summary = env_integration_summary(ready_count, total);

    Ok(EnvStatus {
        repo_path: repo.display().to_string(),
        ready_count,
        total_count: total,
        summary,
        items,
    })
}

/// Fast env summary for project overview — uses the same readiness rules as `get_env_status`.
pub fn summarize_agent_env_light(repo: &Path, knowledge_count: usize) -> AgentEnvStatus {
    let lang = crate::language::current_language();
    let catalog = match load_catalog() {
        Ok(c) => c,
        Err(_) => {
            return AgentEnvStatus {
                ready: false,
                integrated_count: 0,
                total_count: 0,
                summary: lang.tr("未检测", "Not detected").into(),
                detail: lang
                    .tr("打开工程环境页以配置", "Open the project environment page to configure")
                    .into(),
            };
        }
    };

    let total = catalog.integrations.len();
    let ready_count = catalog
        .integrations
        .iter()
        .filter(|def| integration_is_ready(repo, def))
        .count();

    let core = repo
        .join(".agents/skills/terrain-knowledge-skill/SKILL.md")
        .is_file()
        && agents_md_ready(repo);

    AgentEnvStatus {
        ready: core,
        integrated_count: ready_count,
        total_count: total,
        summary: env_integration_summary(ready_count, total),
        detail: lang
            .tr(
                &format!("Skills · 工具链 · AGENTS.md · 私域知识 {knowledge_count} 篇"),
                &format!("Skills · Toolchain · AGENTS.md · {knowledge_count} knowledge article(s)"),
            )
            .to_string(),
    }
}

pub fn plan_env_integration(
    repo: &Path,
    selected_ids: &[String],
    reinstall_ids: &[String],
) -> Result<EnvPlan> {
    let lang = crate::language::current_language();
    let catalog = load_catalog()?;
    let status = get_env_status(repo)?;
    let status_map: HashMap<_, _> = status
        .items
        .iter()
        .map(|i| (i.id.clone(), dependency_ready(i)))
        .collect();

    let selected: HashSet<_> = selected_ids.iter().cloned().collect();
    let reinstall: HashSet<_> = reinstall_ids.iter().cloned().collect();
    let mut steps = Vec::new();
    let mut skipped = Vec::new();

    for def in &catalog.integrations {
        if !selected.contains(&def.id) {
            continue;
        }
        if !dependencies_met(def, &selected, &status_map) {
            skipped.push(
                lang.tr(
                    &format!("{}: 依赖未满足", def.id),
                    &format!("{}: dependencies not met", def.id),
                )
                .to_string(),
            );
            continue;
        }
        if status_map.get(&def.id) == Some(&true) && def.kind != "agents_md" && !def.bundled {
            steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: lang
                    .tr(
                        &format!("重新集成 {}", def.label),
                        &format!("Re-integrate {}", def.label),
                    )
                    .to_string(),
            });
            continue;
        }
        match def.kind.as_str() {
            "skill" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: lang
                    .tr(
                        &format!(
                            "复制 skill → .agents/skills/{0} 与 .claude/skills/{0}",
                            skill_target_name(def)
                        ),
                        &format!(
                            "Copy skill → .agents/skills/{0} and .claude/skills/{0}",
                            skill_target_name(def)
                        ),
                    )
                    .to_string(),
            }),
            "tool" => {
                if def.bundled {
                    plan_bundled_tool(
                        repo,
                        def,
                        &mut steps,
                        &mut skipped,
                        reinstall.contains(&def.id),
                    );
                    continue;
                }
                if def.optional && tool_check_passes(repo, def) {
                    skipped.push(
                        lang.tr(
                            &format!("{}: 已安装", def.id),
                            &format!("{}: installed", def.id),
                        )
                        .to_string(),
                    );
                    continue;
                }
                for step in &def.install_steps {
                    let cmd = format!("{} {}", step.cmd, step.args.join(" "));
                    steps.push(EnvPlanStep {
                        id: def.id.clone(),
                        label: def.label.clone(),
                        kind: def.kind.clone(),
                        action: cmd,
                    });
                }
                if def.install_steps.is_empty() && def.id == "tool-bun" {
                    steps.push(EnvPlanStep {
                        id: def.id.clone(),
                        label: def.label.clone(),
                        kind: def.kind.clone(),
                        action: lang
                            .tr(
                                "检测 bun（未安装时需手动安装 https://bun.sh）",
                                "Detect bun (install manually from https://bun.sh if missing)",
                            )
                            .into(),
                    });
                }
            }
            "agents_md" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: lang
                    .tr(
                        "更新 AGENTS.md 托管片段",
                        "Update the AGENTS.md managed section",
                    )
                    .into(),
            }),
            "terrain_ignore" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: lang
                    .tr(
                        "写入 .terrain/.gitignore 与 .terrain/.gitattributes",
                        "Write .terrain/.gitignore and .terrain/.gitattributes",
                    )
                    .into(),
            }),
            "gitignore" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: lang
                    .tr(
                        &format!("追加 .gitignore: {}", def.patterns.join(", ")),
                        &format!("Append to .gitignore: {}", def.patterns.join(", ")),
                    )
                    .to_string(),
            }),
            _ => {}
        }
    }

    Ok(EnvPlan {
        repo_path: repo.display().to_string(),
        selected_ids: selected_ids.to_vec(),
        steps,
        skipped,
    })
}

pub(crate) fn dependencies_satisfied(
    def: &IntegrationDef,
    selected: &HashSet<String>,
    integrated: &HashMap<String, bool>,
) -> bool {
    dependencies_met(def, selected, integrated)
}

fn dependencies_met(
    def: &IntegrationDef,
    selected: &HashSet<String>,
    integrated: &HashMap<String, bool>,
) -> bool {
    def.depends_on
        .iter()
        .all(|d| selected.contains(d) || integrated.get(d) == Some(&true))
}

fn plan_bundled_tool(
    repo: &Path,
    def: &IntegrationDef,
    steps: &mut Vec<EnvPlanStep>,
    skipped: &mut Vec<String>,
    reinstall: bool,
) {
    let lang = crate::language::current_language();
    if !bundled_tool_runtime_ready(def) {
        skipped.push(
            lang.tr(
                &format!("{}: Terrain 内置工具不可用", def.id),
                &format!("{}: Terrain-bundled tool unavailable", def.id),
            )
            .to_string(),
        );
        return;
    }
    if reinstall {
        match def.id.as_str() {
            "tool-rtk" => {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: lang
                        .tr(
                            "重新部署 ~/.terrain/bin/rtk",
                            "Redeploy ~/.terrain/bin/rtk",
                        )
                        .into(),
                });
            }
            "tool-codegraph" => {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: lang
                        .tr(
                            "重新部署 ~/.terrain/bin/codegraph",
                            "Redeploy ~/.terrain/bin/codegraph",
                        )
                        .into(),
                });
                if !tool_check_passes(repo, def) {
                    steps.push(EnvPlanStep {
                        id: def.id.clone(),
                        label: def.label.clone(),
                        kind: def.kind.clone(),
                        action: lang
                            .tr(
                                "内置 CodeGraph：init -i（写入 .codegraph/）",
                                "Bundled CodeGraph: init -i (writes .codegraph/)",
                            )
                            .into(),
                    });
                }
            }
            _ => skipped.push(
                lang.tr(
                    &format!("{}: 未知内置工具", def.id),
                    &format!("{}: unknown bundled tool", def.id),
                )
                .to_string(),
            ),
        }
        return;
    }
    match def.id.as_str() {
        "tool-rtk" => {
            skipped.push(
                lang.tr(
                    &format!("{}: RTK 由 Terrain 内置提供", def.id),
                    &format!("{}: RTK is provided bundled with Terrain", def.id),
                )
                .to_string(),
            );
        }
        "tool-codegraph" => {
            if tool_check_passes(repo, def) {
                skipped.push(
                    lang.tr(
                        &format!("{}: 仓库索引已就绪", def.id),
                        &format!("{}: repo index is ready", def.id),
                    )
                    .to_string(),
                );
            } else {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: lang
                        .tr(
                            "内置 CodeGraph：init -i（写入 .codegraph/）",
                            "Bundled CodeGraph: init -i (writes .codegraph/)",
                        )
                        .into(),
                });
            }
        }
        _ => skipped.push(
            lang.tr(
                &format!("{}: 未知内置工具", def.id),
                &format!("{}: unknown bundled tool", def.id),
            )
            .to_string(),
        ),
    }
}

fn skill_target_name(def: &IntegrationDef) -> String {
    def.skill_dir
        .clone()
        .or_else(|| def.preset_skill.clone())
        .unwrap_or_else(|| def.id.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

    use super::super::cache::invalidate_env_status_cache;
    use super::*;

    #[test]
    fn dependency_met_when_prerequisite_already_integrated() {
        let catalog = load_catalog().expect("catalog");
        let skill_cg = catalog
            .integrations
            .iter()
            .find(|i| i.id == "skill-codegraph")
            .expect("skill-codegraph");
        let skill_rtk = catalog
            .integrations
            .iter()
            .find(|i| i.id == "skill-rtk")
            .expect("skill-rtk");

        let mut selected = HashSet::new();
        selected.insert("skill-codegraph".into());
        selected.insert("skill-rtk".into());

        let mut integrated = HashMap::new();
        integrated.insert("tool-codegraph".into(), true);
        integrated.insert("tool-rtk".into(), true);
        integrated.insert("skill-terrain-knowledge".into(), true);

        assert!(dependencies_satisfied(skill_cg, &selected, &integrated));
        assert!(dependencies_satisfied(skill_rtk, &selected, &integrated));
    }

    #[test]
    fn overview_summary_matches_full_env_status_count() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
        invalidate_env_status_cache();
        let full = get_env_status(&repo).expect("env status");
        let light = summarize_agent_env_light(&repo, 0);
        assert_eq!(
            light.integrated_count,
            full.ready_count,
            "overview ({}) and env page ({}) should match",
            light.summary,
            full.summary,
        );
    }

    #[test]
    fn bundled_tool_reinstall_produces_plan_steps() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
        invalidate_env_status_cache();
        let selected = vec!["tool-rtk".into(), "tool-codegraph".into()];
        let reinstall = vec!["tool-rtk".into(), "tool-codegraph".into()];
        let plan = plan_env_integration(&repo, &selected, &reinstall).expect("plan");
        let rtk_steps: Vec<_> = plan
            .steps
            .iter()
            .filter(|s| s.id == "tool-rtk")
            .collect();
        let cg_steps: Vec<_> = plan
            .steps
            .iter()
            .filter(|s| s.id == "tool-codegraph")
            .collect();
        assert!(
            !rtk_steps.is_empty(),
            "reinstall RTK should produce at least one plan step, got skipped={:?}",
            plan.skipped
        );
        assert!(
            !cg_steps.is_empty(),
            "reinstall CodeGraph should produce at least one plan step, got skipped={:?}",
            plan.skipped
        );
    }

    #[test]
    fn bundled_tool_without_reinstall_skips_rtk() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
        invalidate_env_status_cache();
        let selected = vec!["tool-rtk".into()];
        let plan = plan_env_integration(&repo, &selected, &[]).expect("plan");
        assert!(plan.steps.is_empty());
        assert!(plan.skipped.iter().any(|s| s.contains("tool-rtk")));
    }

    #[test]
    fn env_status_cache_reuses_when_fingerprint_unchanged() {
        let dir = std::env::temp_dir().join(format!("mm-env-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        invalidate_env_status_cache();

        let first = get_env_status(&dir).expect("first status");
        let second = get_env_status(&dir).expect("cached status");
        assert_eq!(first.summary, second.summary);

        let _ = std::fs::remove_dir_all(&dir);
        invalidate_env_status_cache();
    }
}
