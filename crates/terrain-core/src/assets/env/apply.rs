use std::fs;
use std::path::Path;

use serde::Serialize;

use super::agents_md::patch_agents_md;
use super::catalog::load_catalog;
use super::status::{
    dependencies_satisfied, get_env_status, gitignore_has_patterns,
    invalidate_env_status_cache_for_repo, plan_env_integration, skill_source_path,
    substitute_install_args, tool_check_passes,
};
use crate::bundled_tools::{bundled_codegraph, bundled_rtk, run_bundled_check};
use crate::error::{CoreError, Result};
use crate::path_portable::REPO_AGENT_TOOLS_MANIFEST;

#[derive(Debug, Clone, Serialize)]
pub struct EnvApplyProgress {
    pub stage: String,
    pub message: String,
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct EnvApplyResult {
    pub repo_path: String,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    pub manifest_path: String,
}

pub async fn apply_env_integration(
    repo: &Path,
    selected_ids: &[String],
    reinstall_ids: &[String],
    on_progress: impl Fn(EnvApplyProgress),
) -> Result<EnvApplyResult> {
    let catalog = load_catalog()?;
    let status = get_env_status(repo)?;
    let integrated: std::collections::HashMap<_, _> = status
        .items
        .iter()
        .map(|i| (i.id.clone(), super::status::dependency_ready(i)))
        .collect();
    let plan = plan_env_integration(repo, selected_ids)?;
    let mut applied = Vec::new();
    let mut skipped = plan.skipped.clone();
    let mut errors = Vec::new();

    let selected: std::collections::HashSet<_> = selected_ids.iter().cloned().collect();

    let needs_agent_deploy = catalog.integrations.iter().any(|def| {
        def.bundled && selected.contains(&def.id)
    });
    if needs_agent_deploy {
        let force_bundled = reinstall_ids.iter().any(|id| {
            matches!(id.as_str(), "tool-rtk" | "tool-codegraph")
        });
        let opts = crate::agent_tools_deploy::DeployOptions {
            force: force_bundled,
        };
        if let Ok(paths) = crate::agent_tools_deploy::deploy_agent_toolchain_with_options(opts) {
            let _ = crate::agent_tools_deploy::write_repo_agent_tools_manifest(repo, &paths);
        }
    }

    for def in &catalog.integrations {
        if !selected.contains(&def.id) {
            continue;
        }
        if !dependencies_satisfied(def, &selected, &integrated) {
            continue;
        }

        on_progress(EnvApplyProgress {
            stage: def.id.clone(),
            message: format!("正在集成 {}…", def.label),
        });

        let result = match def.kind.as_str() {
            "skill" => apply_skill(repo, def),
            "tool" => apply_tool(repo, def).await,
            "agents_md" => patch_agents_md(repo).map(|p| format!("已更新 {p}")),
            "gitignore" => apply_gitignore(repo, &def.patterns),
            _ => Err(CoreError::InvalidDoc(format!("unknown kind {}", def.kind))),
        };

        match result {
            Ok(msg) => {
                applied.push(format!("{}: {msg}", def.id));
            }
            Err(e) => {
                if def.optional {
                    skipped.push(format!("{}: {e}", def.id));
                } else {
                    errors.push(format!("{}: {e}", def.id));
                }
            }
        }
    }

    ensure_knowledge_dir(repo)?;

    let manifest_path = write_manifest(repo, &applied, &catalog.version)?;
    invalidate_env_status_cache_for_repo(repo);

    Ok(EnvApplyResult {
        repo_path: repo.display().to_string(),
        applied,
        skipped,
        errors,
        manifest_path,
    })
}

fn apply_skill(repo: &Path, def: &super::catalog::IntegrationDef) -> Result<String> {
    let source = skill_source_path(def);
    if !source.is_dir() {
        return Err(CoreError::InvalidDoc(format!(
            "skill source not found: {}",
            source.display()
        )));
    }
    let target_name = def
        .skill_dir
        .as_deref()
        .or(def.preset_skill.as_deref())
        .unwrap_or(&def.id);
    let dest = repo.join(".agents/skills").join(target_name);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_dir_recursive(&source, &dest)?;
    Ok(format!("→ .agents/skills/{target_name}"))
}

async fn apply_tool(repo: &Path, def: &super::catalog::IntegrationDef) -> Result<String> {
    if def.id == "tool-bun" {
        if tool_check_passes(repo, def) {
            return Ok("bun 已存在".into());
        }
        return Err(CoreError::InvalidDoc(
            "bun 未安装，请从 https://bun.sh 安装后重试".into(),
        ));
    }

    if def.bundled {
        return apply_bundled_tool(repo, def).await;
    }

    if tool_check_passes(repo, def) {
        return Ok(format!("{} 已存在，跳过安装", def.label));
    }

    if def.install_steps.is_empty() {
        return Err(CoreError::InvalidDoc("无安装步骤".into()));
    }

    let mut messages = Vec::new();
    for step in &def.install_steps {
        let args = substitute_install_args(repo, def, &step.args);
        run_command(repo, &step.cmd, &args).await?;
        messages.push(format!("{} {}", step.cmd, args.join(" ")));
    }
    Ok(messages.join("; "))
}

async fn apply_bundled_tool(
    repo: &Path,
    def: &super::catalog::IntegrationDef,
) -> Result<String> {
    let _ = repo;
    let manifest = REPO_AGENT_TOOLS_MANIFEST;

    match def.id.as_str() {
        "tool-rtk" => {
            let rtk = bundled_rtk().ok_or_else(|| {
                CoreError::InvalidDoc("Terrain 内置 RTK 不可用".into())
            })?;
            if !run_bundled_check(&rtk, &["gain"], repo) {
                return Err(CoreError::InvalidDoc(format!(
                    "内置 RTK 无法执行: {}",
                    rtk.display()
                )));
            }
            Ok(format!(
                "RTK 已部署至 ~/.terrain/bin/rtk（详见 {manifest}）"
            ))
        }
        "tool-codegraph" => {
            let codegraph = bundled_codegraph().ok_or_else(|| {
                CoreError::InvalidDoc("Terrain 内置 CodeGraph 不可用".into())
            })?;
            if tool_check_passes(repo, def) {
                Ok(format!("CodeGraph 索引已就绪（Agent 路径见 {manifest}）"))
            } else {
                run_binary(repo, &codegraph, &["init", "-i"]).await?;
                Ok(format!(
                    "CodeGraph 已初始化 .codegraph/（Agent 使用 ~/.terrain/bin/codegraph，见 {manifest}）"
                ))
            }
        }
        _ => Err(CoreError::InvalidDoc(format!("未知内置工具 {}", def.id))),
    }
}

async fn run_binary(cwd: &Path, program: &Path, args: &[&str]) -> Result<()> {
    let output = tokio::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| {
            CoreError::InvalidDoc(format!(
                "failed to run {}: {e}",
                program.display()
            ))
        })?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(CoreError::InvalidDoc(format!(
        "{} {} failed ({}): {stderr}{stdout}",
        program.display(),
        args.join(" "),
        output.status
    )))
}

async fn run_command(cwd: &Path, cmd: &str, args: &[String]) -> Result<()> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| CoreError::InvalidDoc(format!("failed to run {cmd}: {e}")))?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(CoreError::InvalidDoc(format!(
        "{cmd} {} failed ({}): {stderr}{stdout}",
        args.join(" "),
        output.status
    )))
}

fn apply_gitignore(repo: &Path, patterns: &[String]) -> Result<String> {
    if gitignore_has_patterns(repo, patterns) {
        return Ok("已存在".into());
    }
    let path = repo.join(".gitignore");
    let mut content = if path.is_file() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("\n# Terrain — repomix index (regenerated locally, not versioned)\n");
    for p in patterns {
        content.push_str(p);
        content.push('\n');
    }
    fs::write(&path, content)?;
    Ok(format!("追加 {} 条规则", patterns.len()))
}

fn ensure_knowledge_dir(repo: &Path) -> Result<()> {
    let dir = repo.join(".terrain/knowledge");
    fs::create_dir_all(&dir)?;
    let readme = dir.join(".gitkeep");
    if !readme.exists() {
        fs::write(readme, "")?;
    }
    Ok(())
}

fn write_manifest(repo: &Path, applied: &[String], catalog_version: &u32) -> Result<String> {
    let env_dir = repo.join(".terrain/env");
    fs::create_dir_all(&env_dir)?;
    let manifest = serde_json::json!({
        "catalog_version": catalog_version,
        "integrated_at": chrono::Utc::now().to_rfc3339(),
        "applied": applied,
    });
    let path = env_dir.join("manifest.json");
    fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(path.display().to_string())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
