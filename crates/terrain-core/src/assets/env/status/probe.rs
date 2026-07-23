//! Integration readiness probes.

use std::path::Path;

use super::super::agents_md::agents_md_ready;
use super::super::catalog::IntegrationDef;
use super::types::EnvIntegrationStatus;
use crate::agent_tools_deploy::agent_bin_dir;
use crate::bundled_tools::{bundled_codegraph, bundled_rtk, run_bundled_check};
use crate::error::Result;

/// True when the repo has a CodeGraph index on disk (no CLI spawn).
pub fn codegraph_index_ready(repo: &Path) -> bool {
    repo.join(".codegraph/codegraph.db").is_file()
}

pub(crate) fn integration_is_ready(repo: &Path, def: &IntegrationDef) -> bool {
    match def.kind.as_str() {
        "skill" => {
            let name = skill_target_name(def);
            super::super::apply::SKILL_DEPLOY_DIRS
                .iter()
                .all(|base| repo.join(base).join(&name).join("SKILL.md").is_file())
        }
        "tool" => {
            let locked = bundled_tool_runtime_ready(def);
            match def.id.as_str() {
                "tool-rtk" if locked => true,
                _ => tool_check_passes(repo, def),
            }
        }
        "agents_md" => agents_md_ready(repo),
        "gitignore" => gitignore_has_patterns(repo, &def.patterns),
        _ => false,
    }
}

pub(crate) fn dependency_ready(item: &EnvIntegrationStatus) -> bool {
    item.integrated || item.locked
}

pub(crate) fn bundled_tool_runtime_ready(def: &IntegrationDef) -> bool {
    if !def.bundled {
        return false;
    }
    match def.id.as_str() {
        "tool-rtk" => bundled_rtk().is_some(),
        "tool-codegraph" => bundled_codegraph().is_some(),
        _ => false,
    }
}

pub(crate) fn check_integration(repo: &Path, def: &IntegrationDef) -> Result<EnvIntegrationStatus> {
    let integrated = integration_is_ready(repo, def);
    let detail = match def.kind.as_str() {
        "skill" => {
            let name = skill_target_name(def);
            let missing: Vec<&str> = super::super::apply::SKILL_DEPLOY_DIRS
                .iter()
                .copied()
                .filter(|base| !repo.join(base).join(&name).join("SKILL.md").is_file())
                .collect();
            if missing.is_empty() {
                format!(".agents/skills/{name} · .claude/skills/{name}")
            } else if missing.len() == super::super::apply::SKILL_DEPLOY_DIRS.len() {
                "未注入".into()
            } else {
                format!("缺少 {}/{name}（重新集成以补齐）", missing.join(", "))
            }
        }
        "tool" => {
            let locked = bundled_tool_runtime_ready(def);
            tool_status_detail(def, repo, integrated, locked)
        }
        "agents_md" => {
            if integrated {
                "AGENTS.md 含 Terrain 托管片段".into()
            } else {
                "未配置".into()
            }
        }
        "gitignore" => {
            if integrated {
                "repomix.md 已忽略".into()
            } else {
                "未配置".into()
            }
        }
        _ => "未知类型".into(),
    };

    let locked = def.bundled && bundled_tool_runtime_ready(def);

    Ok(EnvIntegrationStatus {
        id: def.id.clone(),
        kind: def.kind.clone(),
        label: def.label.clone(),
        description: def.description.clone(),
        integrated,
        optional: def.optional,
        bundled: def.bundled,
        locked,
        depends_on: def.depends_on.clone(),
        detail,
    })
}

pub(crate) fn tool_check_passes(repo: &Path, def: &IntegrationDef) -> bool {
    match def.id.as_str() {
        "tool-bun" => {
            if def.optional {
                crate::shell_path::command_on_path("bun")
            } else {
                command_succeeds("bun", &["--version"], repo)
            }
        }
        "tool-rtk" => rtk_runtime_ready(),
        "tool-codegraph" => codegraph_index_ready(repo),
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

pub(crate) fn rtk_runtime_ready() -> bool {
    let bin_dir = agent_bin_dir();
    for name in ["rtk", "rtk.exe"] {
        if bin_dir.join(name).is_file() {
            return true;
        }
    }
    false
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

fn skill_target_name(def: &IntegrationDef) -> String {
    def.skill_dir
        .clone()
        .or_else(|| def.preset_skill.clone())
        .unwrap_or_else(|| def.id.clone())
}

fn tool_status_detail(def: &IntegrationDef, repo: &Path, ok: bool, locked: bool) -> String {
    if locked {
        return match def.id.as_str() {
            "tool-rtk" => "约定路径 ~/.terrain/bin/rtk（无 Terrain 时 bunx @terrain-ai/rtk）".into(),
            "tool-codegraph" if ok => {
                "约定路径 ~/.terrain/bin/codegraph（无 Terrain 时 bunx codegraph）· 仓库索引已就绪".into()
            }
            "tool-codegraph" => {
                "约定路径 ~/.terrain/bin/codegraph（无 Terrain 时 bunx codegraph）· 待初始化 .codegraph/".into()
            }
            _ => "Terrain 内置".into(),
        };
    }
    if !ok {
        return if def.id == "tool-bun" {
            "可选：bun 未检测到".into()
        } else {
            "未安装".into()
        };
    }

    match def.id.as_str() {
        "tool-rtk" => {
            if bundled_rtk()
                .as_ref()
                .is_some_and(|p| run_bundled_check(p, &["gain"], repo))
            {
                "已安装（Terrain bundled）".into()
            } else if command_succeeds("rtk", &["gain"], repo) {
                "已安装（PATH）".into()
            } else {
                "已安装（项目本地）".into()
            }
        }
        "tool-codegraph" => {
            if codegraph_index_ready(repo) {
                "已安装（Terrain bundled · 索引就绪）".into()
            } else {
                "已安装".into()
            }
        }
        _ => "已安装".into(),
    }
}

fn command_succeeds(program: &str, args: &[&str], cwd: &Path) -> bool {
    crate::process::command(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codegraph_index_ready_checks_db_file() {
        let dir = std::env::temp_dir().join(format!("mm-cg-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!codegraph_index_ready(&dir));
        std::fs::create_dir_all(dir.join(".codegraph")).unwrap();
        assert!(!codegraph_index_ready(&dir));
        std::fs::write(dir.join(".codegraph/codegraph.db"), b"x").unwrap();
        assert!(codegraph_index_ready(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
