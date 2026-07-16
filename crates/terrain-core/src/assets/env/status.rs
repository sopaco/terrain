use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use super::agents_md::agents_md_ready;
use super::catalog::{
    env_catalog_root, load_catalog, resolve_skill_source, IntegrationDef,
};
use crate::agent_tools_deploy::agent_bin_dir;
use crate::bundled_tools::{bundled_codegraph, bundled_rtk, run_bundled_check};
use crate::error::Result;
use crate::schema::AgentEnvStatus;

struct EnvStatusCacheEntry {
    fingerprint: u64,
    status: EnvStatus,
}

static ENV_STATUS_CACHE: Mutex<Option<HashMap<String, EnvStatusCacheEntry>>> = Mutex::new(None);

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvStatus {
    pub repo_path: String,
    pub ready_count: usize,
    pub total_count: usize,
    pub summary: String,
    pub items: Vec<EnvIntegrationStatus>,
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvIntegrationStatus {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub description: String,
  pub integrated: bool,
  pub optional: bool,
  pub bundled: bool,
  pub locked: bool,
  pub depends_on: Vec<String>,
    pub detail: String,
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvPlan {
    pub repo_path: String,
    pub selected_ids: Vec<String>,
    pub steps: Vec<EnvPlanStep>,
    pub skipped: Vec<String>,
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvPlanStep {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub action: String,
}

/// Drop cached env status (all repos). Call after env apply or global tool deploy.
pub fn invalidate_env_status_cache() {
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock() {
        *guard = None;
    }
}

/// Drop cached env status for one repository.
pub fn invalidate_env_status_cache_for_repo(repo: &Path) {
    let Ok(key) = env_cache_key(repo) else {
        return;
    };
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(&key);
        }
    }
}

/// Fast env summary for project overview — uses the same readiness rules as `get_env_status`.
pub fn summarize_agent_env_light(repo: &Path, knowledge_count: usize) -> AgentEnvStatus {
    let catalog = match load_catalog() {
        Ok(c) => c,
        Err(_) => {
            return AgentEnvStatus {
                ready: false,
                integrated_count: 0,
                total_count: 0,
                summary: "未检测".into(),
                detail: "打开工程环境页以配置".into(),
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
        summary: format!("{ready_count}/{total} 已集成"),
        detail: format!("Skills · 工具链 · AGENTS.md · 私域知识 {knowledge_count} 篇"),
    }
}

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

fn compute_env_status(repo: &Path) -> Result<EnvStatus> {
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

fn env_cache_key(repo: &Path) -> Result<String> {
    Ok(repo
        .canonicalize()
        .unwrap_or_else(|_| repo.to_path_buf())
        .to_string_lossy()
        .into_owned())
}

fn env_cache_fingerprint(repo: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in env_cache_watch_paths(repo) {
        path.to_string_lossy().hash(&mut hasher);
        if let Ok(meta) = std::fs::metadata(&path) {
            meta.len().hash(&mut hasher);
            meta.modified().ok().hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn env_cache_watch_paths(repo: &Path) -> Vec<PathBuf> {
    let bin = agent_bin_dir();
    vec![
        repo.join("AGENTS.md"),
        repo.join(".gitignore"),
        repo.join(".codegraph/codegraph.db"),
        repo.join(".terrain/env/manifest.json"),
        repo.join(".terrain/env/agent-tools.json"),
        repo.join(".agents/skills/terrain-knowledge-skill/SKILL.md"),
        repo.join(".agents/skills/codegraph-skill/SKILL.md"),
        repo.join(".agents/skills/rtk-skill/SKILL.md"),
        repo.join(".agents/skills/repomix-context-skill/SKILL.md"),
        repo.join(".claude/skills/terrain-knowledge-skill/SKILL.md"),
        repo.join(".claude/skills/codegraph-skill/SKILL.md"),
        repo.join(".claude/skills/rtk-skill/SKILL.md"),
        repo.join(".claude/skills/repomix-context-skill/SKILL.md"),
        bin.join("rtk"),
        bin.join("codegraph"),
    ]
}

fn env_cache_get(key: &str, fingerprint: u64) -> Option<EnvStatus> {
    let guard = ENV_STATUS_CACHE.lock().ok()?;
    let map = guard.as_ref()?;
    let entry = map.get(key)?;
    if entry.fingerprint == fingerprint {
        Some(entry.status.clone())
    } else {
        None
    }
}

fn env_cache_put(key: String, fingerprint: u64, status: EnvStatus) {
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(
            key,
            EnvStatusCacheEntry {
                fingerprint,
                status,
            },
        );
    }
}

fn integration_is_ready(repo: &Path, def: &IntegrationDef) -> bool {
    match def.kind.as_str() {
        "skill" => {
            let name = skill_target_name(def);
            super::apply::SKILL_DEPLOY_DIRS
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

/// True when the repo has a CodeGraph index on disk (no CLI spawn).
pub fn codegraph_index_ready(repo: &Path) -> bool {
    repo.join(".codegraph/codegraph.db").is_file()
}

pub fn plan_env_integration(
    repo: &Path,
    selected_ids: &[String],
    reinstall_ids: &[String],
) -> Result<EnvPlan> {
    let catalog = load_catalog()?;
    let status = get_env_status(repo)?;
    let status_map: std::collections::HashMap<_, _> = status
        .items
        .iter()
        .map(|i| (i.id.clone(), dependency_ready(i)))
        .collect();

    let selected: std::collections::HashSet<_> = selected_ids.iter().cloned().collect();
    let reinstall: std::collections::HashSet<_> = reinstall_ids.iter().cloned().collect();
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
        if status_map.get(&def.id) == Some(&true) && def.kind != "agents_md" && !def.bundled {
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
                action: format!(
                    "复制 skill → .agents/skills/{0} 与 .claude/skills/{0}",
                    skill_target_name(def)
                ),
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

pub(crate) fn dependency_ready(item: &EnvIntegrationStatus) -> bool {
    item.integrated || item.locked
}

fn bundled_tool_runtime_ready(def: &IntegrationDef) -> bool {
    if !def.bundled {
        return false;
    }
    match def.id.as_str() {
        "tool-rtk" => bundled_rtk().is_some(),
        "tool-codegraph" => bundled_codegraph().is_some(),
        _ => false,
    }
}

fn plan_bundled_tool(
    repo: &Path,
    def: &IntegrationDef,
    steps: &mut Vec<EnvPlanStep>,
    skipped: &mut Vec<String>,
    reinstall: bool,
) {
    if !bundled_tool_runtime_ready(def) {
        skipped.push(format!("{}: Terrain 内置工具不可用", def.id));
        return;
    }
    if reinstall {
        match def.id.as_str() {
            "tool-rtk" => {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: "重新部署 ~/.terrain/bin/rtk".into(),
                });
            }
            "tool-codegraph" => {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: "重新部署 ~/.terrain/bin/codegraph".into(),
                });
                if !tool_check_passes(repo, def) {
                    steps.push(EnvPlanStep {
                        id: def.id.clone(),
                        label: def.label.clone(),
                        kind: def.kind.clone(),
                        action: "内置 CodeGraph：init -i（写入 .codegraph/）".into(),
                    });
                }
            }
            _ => skipped.push(format!("{}: 未知内置工具", def.id)),
        }
        return;
    }
    match def.id.as_str() {
        "tool-rtk" => {
            skipped.push(format!("{}: RTK 由 Terrain 内置提供", def.id));
        }
        "tool-codegraph" => {
            if tool_check_passes(repo, def) {
                skipped.push(format!("{}: 仓库索引已就绪", def.id));
            } else {
                steps.push(EnvPlanStep {
                    id: def.id.clone(),
                    label: def.label.clone(),
                    kind: def.kind.clone(),
                    action: "内置 CodeGraph：init -i（写入 .codegraph/）".into(),
                });
            }
        }
        _ => skipped.push(format!("{}: 未知内置工具", def.id)),
    }
}

pub(crate) fn dependencies_satisfied(
    def: &IntegrationDef,
    selected: &std::collections::HashSet<String>,
    integrated: &std::collections::HashMap<String, bool>,
) -> bool {
    dependencies_met(def, selected, integrated)
}

fn check_integration(repo: &Path, def: &IntegrationDef) -> Result<EnvIntegrationStatus> {
    let integrated = integration_is_ready(repo, def);
    let detail = match def.kind.as_str() {
        "skill" => {
            let name = skill_target_name(def);
            let missing: Vec<&str> = super::apply::SKILL_DEPLOY_DIRS
                .iter()
                .copied()
                .filter(|base| !repo.join(base).join(&name).join("SKILL.md").is_file())
                .collect();
            if missing.is_empty() {
                format!(".agents/skills/{name} · .claude/skills/{name}")
            } else if missing.len() == super::apply::SKILL_DEPLOY_DIRS.len() {
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
    args.to_vec()
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
