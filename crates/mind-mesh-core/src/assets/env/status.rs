use std::path::{Path, PathBuf};

use serde::Serialize;

use super::agents_md::agents_md_ready;
use super::catalog::{
    env_catalog_root, load_catalog, resolve_skill_source, rtk_package_path, IntegrationDef,
};
use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
pub struct EnvStatus {
    pub repo_path: String,
    pub ready_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub items: Vec<EnvIntegrationStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvIntegrationStatus {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub description: String,
    pub integrated: bool,
    pub optional: bool,
    pub depends_on: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvPlan {
    pub repo_path: String,
    pub selected_ids: Vec<String>,
    pub steps: Vec<EnvPlanStep>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvPlanStep {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub action: String,
}

pub fn get_env_status(repo: &Path) -> Result<EnvStatus> {
    let catalog = load_catalog()?;
    let items: Vec<_> = catalog
        .integrations
        .iter()
        .map(|def| check_integration(repo, def))
        .collect::<Result<_>>()?;

    let ready_count = items.iter().filter(|i| i.integrated).count();
    let total = items.len();
    let summary = format!("{ready_count}/{total} 已集成");

    Ok(EnvStatus {
        repo_path: repo.display().to_string(),
        ready_count,
        total_count: total,
        summary,
        items,
    })
}

pub fn plan_env_integration(repo: &Path, selected_ids: &[String]) -> Result<EnvPlan> {
    let catalog = load_catalog()?;
    let status = get_env_status(repo)?;
    let status_map: std::collections::HashMap<_, _> = status
        .items
        .iter()
        .map(|i| (i.id.clone(), i.integrated))
        .collect();

    let selected: std::collections::HashSet<_> = selected_ids.iter().cloned().collect();
    let mut steps = Vec::new();
    let mut skipped = Vec::new();

    for def in &catalog.integrations {
        if !selected.contains(&def.id) {
            continue;
        }
        if !dependencies_met(def, &selected, &status_map) {
            skipped.push(format!("{}: 依赖未满足", def.id));
            continue;
        }
        if status_map.get(&def.id) == Some(&true) && def.kind != "agents_md" {
            steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: format!("重新集成 {}", def.label),
            });
            continue;
        }
        match def.kind.as_str() {
            "skill" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: format!("复制 skill → .agents/skills/{}", skill_target_name(def)),
            }),
            "tool" => {
                if def.optional && tool_check_passes(repo, def) {
                    skipped.push(format!("{}: 已安装", def.id));
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
                        action: "检测 bun（未安装时需手动安装 https://bun.sh）".into(),
                    });
                }
            }
            "agents_md" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: "更新 AGENTS.md 托管片段".into(),
            }),
            "gitignore" => steps.push(EnvPlanStep {
                id: def.id.clone(),
                label: def.label.clone(),
                kind: def.kind.clone(),
                action: format!("追加 .gitignore: {}", def.patterns.join(", ")),
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

fn dependencies_met(
    def: &IntegrationDef,
    selected: &std::collections::HashSet<String>,
    integrated: &std::collections::HashMap<String, bool>,
) -> bool {
    def.depends_on
        .iter()
        .all(|d| selected.contains(d) || integrated.get(d) == Some(&true))
}

pub(crate) fn dependencies_satisfied(
    def: &IntegrationDef,
    selected: &std::collections::HashSet<String>,
    integrated: &std::collections::HashMap<String, bool>,
) -> bool {
    dependencies_met(def, selected, integrated)
}

fn check_integration(repo: &Path, def: &IntegrationDef) -> Result<EnvIntegrationStatus> {
    let (integrated, detail) = match def.kind.as_str() {
        "skill" => {
            let target = repo.join(".agents/skills").join(skill_target_name(def));
            let skill_file = target.join("SKILL.md");
            (
                skill_file.is_file(),
                if skill_file.is_file() {
                    format!("{}", skill_file.display())
                } else {
                    "未注入".into()
                },
            )
        }
        "tool" => {
            let ok = tool_check_passes(repo, def);
            (
                ok,
                if ok {
                    if def.id == "tool-rtk" {
                        if command_succeeds("rtk", &["gain"], repo) {
                            "已安装（PATH）".into()
                        } else {
                            "已安装（项目本地）".into()
                        }
                    } else {
                        "已安装".into()
                    }
                } else if def.id == "tool-bun" {
                    "可选：bun 未检测到".into()
                } else {
                    "未安装".into()
                },
            )
        }
        "agents_md" => (
            agents_md_ready(repo),
            if agents_md_ready(repo) {
                "AGENTS.md 含 MindMesh 托管片段".into()
            } else {
                "未配置".into()
            },
        ),
        "gitignore" => {
            let ok = gitignore_has_patterns(repo, &def.patterns);
            (
                ok,
                if ok {
                    "repomix.md 已忽略".into()
                } else {
                    "未配置".into()
                },
            )
        }
        _ => (false, "未知类型".into()),
    };

    Ok(EnvIntegrationStatus {
        id: def.id.clone(),
        kind: def.kind.clone(),
        label: def.label.clone(),
        description: def.description.clone(),
        integrated,
        optional: def.optional,
        depends_on: def.depends_on.clone(),
        detail,
    })
}

fn skill_target_name(def: &IntegrationDef) -> String {
    def.skill_dir
        .clone()
        .or_else(|| def.preset_skill.clone())
        .unwrap_or_else(|| def.id.clone())
}

pub(crate) fn tool_check_passes(repo: &Path, def: &IntegrationDef) -> bool {
    match def.id.as_str() {
        "tool-bun" => command_succeeds("bun", &["--version"], repo),
        "tool-rtk" => rtk_available(repo),
        "tool-codegraph" => codegraph_available(repo),
        _ => {
            if let Some(check) = &def.check {
                let args: Vec<&str> = check.iter().skip(1).map(String::as_str).collect();
                command_succeeds(&check[0], &args, repo)
            } else {
                false
            }
        }
    }
}

/// RTK on PATH (global install) or project-local via bunx / node_modules.
fn rtk_available(repo: &Path) -> bool {
    command_succeeds("rtk", &["gain"], repo)
        || command_succeeds("bunx", &["rtk", "gain"], repo)
        || local_bin_succeeds(repo, "rtk", &["gain"])
}

/// CodeGraph CLI on PATH or project-local via bunx / node_modules.
fn codegraph_available(repo: &Path) -> bool {
    command_succeeds("codegraph", &["status"], repo)
        || command_succeeds("bunx", &["codegraph", "status"], repo)
        || local_bin_succeeds(repo, "codegraph", &["status"])
}

fn local_bin_succeeds(repo: &Path, bin: &str, args: &[&str]) -> bool {
    let path = repo.join("node_modules/.bin").join(bin);
    if !path.is_file() {
        return false;
    }
    command_succeeds(&path.to_string_lossy(), args, repo)
}

fn command_succeeds(program: &str, args: &[&str], cwd: &Path) -> bool {
    std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn gitignore_has_patterns(repo: &Path, patterns: &[String]) -> bool {
    let path = repo.join(".gitignore");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    patterns
        .iter()
        .all(|p| content.lines().any(|l| l.trim() == p))
}

pub(crate) fn skill_source_path(def: &IntegrationDef) -> PathBuf {
    resolve_skill_source(&env_catalog_root(), def)
}

pub(crate) fn substitute_install_args(
    _repo: &Path,
    _def: &IntegrationDef,
    args: &[String],
) -> Vec<String> {
    args.iter()
        .map(|a| {
            if a == "@mind-mesh/rtk" {
                let pkg = rtk_package_path();
                if pkg.exists() {
                    return format!("file:{}", pkg.display());
                }
            }
            a.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

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
        integrated.insert("skill-mind-mesh-knowledge".into(), true);

        assert!(dependencies_satisfied(skill_cg, &selected, &integrated));
        assert!(dependencies_satisfied(skill_rtk, &selected, &integrated));
    }
}
