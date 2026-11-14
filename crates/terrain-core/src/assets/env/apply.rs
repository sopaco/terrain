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
use crate::fingerprint::fingerprint;
use crate::path_portable::REPO_AGENT_TOOLS_MANIFEST;

/// Skill deployment targets: ACP/OpenCode convention + Claude Code discovery dir.
pub const SKILL_DEPLOY_DIRS: [&str; 2] = [".agents/skills", ".claude/skills"];

use crate::progress::{EnvApplyProgress, ProgressEvent};

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
    let lang = crate::language::current_language();
    let catalog = load_catalog()?;
    let status = get_env_status(repo)?;
    let integrated: std::collections::HashMap<_, _> = status
        .items
        .iter()
        .map(|i| (i.id.clone(), super::status::dependency_ready(i)))
        .collect();
    let plan = plan_env_integration(repo, selected_ids, reinstall_ids)?;
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
        let toolchain_ready = super::status::rtk_runtime_ready()
            && bundled_codegraph().is_some_and(|p| p.is_file());
        if force_bundled || !toolchain_ready {
            let opts = crate::agent_tools_deploy::DeployOptions {
                force: force_bundled,
            };
            if let Ok(paths) = crate::agent_tools_deploy::deploy_agent_toolchain_with_options(opts)
            {
                let _ = crate::agent_tools_deploy::write_repo_agent_tools_manifest(repo, &paths);
            }
        }
    }

    for def in &catalog.integrations {
        if !selected.contains(&def.id) {
            continue;
        }
        if !dependencies_satisfied(def, &selected, &integrated) {
            continue;
        }

        on_progress(ProgressEvent::env(
            def.id.clone(),
            lang.tr(
                &format!("正在集成 {}…", def.label),
                &format!("Integrating {}…", def.label),
            )
            .to_string(),
        ));

        let result = match def.kind.as_str() {
            "skill" => apply_skill(repo, def, reinstall_ids.contains(&def.id)),
            "tool" => apply_tool(repo, def).await,
            "agents_md" => patch_agents_md(repo).map(|p| {
                lang.tr(&format!("已更新 {p}"), &format!("Updated {p}"))
                    .to_string()
            }),
            "terrain_ignore" => apply_terrain_git_policy(repo),
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

fn apply_skill(
    repo: &Path,
    def: &super::catalog::IntegrationDef,
    force: bool,
) -> Result<String> {
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
    // `.agents/skills` is the ACP/OpenCode convention; `.claude/skills` is what
    // Claude Code actually scans for Skill discovery. Deploy to both.
    let mut preserved = 0usize;
    for base in SKILL_DEPLOY_DIRS {
        let dest = repo.join(base).join(target_name);
        preserved += sync_skill_dir(&source, &dest, force)?.preserved;
    }

    let lang = crate::language::current_language();
    let note = if preserved > 0 {
        format!(
            " {}",
            lang.tr(
                &format!("（保留 {preserved} 个自定义文件，未覆盖）"),
                &format!("({preserved} customized file(s) preserved)"),
            )
        )
    } else {
        String::new()
    };
    Ok(format!(
        "→ .agents/skills/{target_name} + .claude/skills/{target_name}{note}"
    ))
}

/// Manifest stored inside each deployed skill directory. It records the fingerprint of the
/// content Terrain last wrote, which is what lets us tell an outdated managed file from a
/// user's own edit.
const SKILL_MANIFEST_NAME: &str = ".terrain-skill.json";
const SKILL_MANIFEST_SCHEMA: u32 = 1;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct SkillDeployManifest {
    schema_version: u32,
    files: std::collections::BTreeMap<String, String>,
}

#[derive(Default)]
struct SkillSyncReport {
    preserved: usize,
}

/// Upsert one skill directory without deleting it.
///
/// Rules, in order:
/// - a file Terrain wrote last time and the user has not touched → update to the new version;
/// - a file whose on-disk content differs from what Terrain last wrote → user edit, keep it;
/// - a file that exists only in the target (user added it) → keep it;
/// - a managed file removed from the source → delete it, but only if still untouched.
///
/// `force` (the UI's "reinstall") overrides the user-edit protection for managed files, but
/// still never removes user-added files or the directory itself.
fn sync_skill_dir(source: &Path, dest: &Path, force: bool) -> Result<SkillSyncReport> {
    let mut source_files = std::collections::BTreeMap::new();
    collect_skill_files(source, Path::new(""), &mut source_files)?;

    let manifest_path = dest.join(SKILL_MANIFEST_NAME);
    let previous: SkillDeployManifest = crate::doc::read_json::<SkillDeployManifest>(&manifest_path)
        .ok()
        .filter(|m| m.schema_version == SKILL_MANIFEST_SCHEMA)
        .unwrap_or_default();

    fs::create_dir_all(dest)?;
    let mut next = SkillDeployManifest {
        schema_version: SKILL_MANIFEST_SCHEMA,
        files: std::collections::BTreeMap::new(),
    };
    let mut report = SkillSyncReport::default();

    for (relative, src_bytes) in &source_files {
        let src_fp = fingerprint(src_bytes);
        let target = dest.join(relative);
        let current = fs::read(&target).ok();

        match current {
            None => {
                write_skill_file(&target, src_bytes)?;
                next.files.insert(relative.clone(), src_fp);
            }
            Some(existing) => {
                let existing_fp = fingerprint(&existing);
                let managed_before = previous.files.get(relative) == Some(&existing_fp);
                let user_edited = existing_fp != src_fp && !managed_before && !force;

                if user_edited {
                    // Keep both the user's content and the old marker, so later upgrades keep
                    // protecting this file instead of silently claiming ownership again.
                    if let Some(old) = previous.files.get(relative) {
                        next.files.insert(relative.clone(), old.clone());
                    }
                    report.preserved += 1;
                } else {
                    if existing_fp != src_fp {
                        write_skill_file(&target, src_bytes)?;
                    }
                    next.files.insert(relative.clone(), src_fp);
                }
            }
        }
    }

    // Managed files that disappeared from the source: remove only untouched copies.
    for (relative, last_fp) in &previous.files {
        if next.files.contains_key(relative) || relative == SKILL_MANIFEST_NAME {
            continue;
        }
        if let Ok(existing) = fs::read(dest.join(relative))
            && fingerprint(&existing).as_str() == last_fp.as_str()
        {
            let _ = fs::remove_file(dest.join(relative));
        }
    }

    crate::doc::write_json(&manifest_path, &next)?;
    Ok(report)
}

fn write_skill_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn collect_skill_files(
    dir: &Path,
    prefix: &Path,
    out: &mut std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let relative = prefix.join(&name);
        if entry.file_type()?.is_dir() {
            collect_skill_files(&entry.path(), &relative, out)?;
            continue;
        }
        let key = relative.to_string_lossy().replace('\\', "/");
        if key == SKILL_MANIFEST_NAME {
            continue;
        }
        out.insert(key, fs::read(entry.path())?);
    }
    Ok(())
}

async fn apply_tool(repo: &Path, def: &super::catalog::IntegrationDef) -> Result<String> {
    let lang = crate::language::current_language();
    if def.id == "tool-bun" {
        if tool_check_passes(repo, def) {
            return Ok(lang.tr("bun 已存在", "bun is already installed").into());
        }
        return Err(CoreError::InvalidDoc(
            lang.tr(
                "bun 未安装，请从 https://bun.sh 安装后重试",
                "bun is not installed; install it from https://bun.sh and retry",
            )
            .into(),
        ));
    }

    if def.bundled {
        return apply_bundled_tool(repo, def).await;
    }

    if tool_check_passes(repo, def) {
        return Ok(lang
            .tr(
                &format!("{} 已存在，跳过安装", def.label),
                &format!("{} is already installed, skipping", def.label),
            )
            .to_string());
    }

    if def.install_steps.is_empty() {
        return Err(CoreError::InvalidDoc(
            lang.tr("无安装步骤", "No install steps").into(),
        ));
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
    let lang = crate::language::current_language();
    let manifest = REPO_AGENT_TOOLS_MANIFEST;

    match def.id.as_str() {
        "tool-rtk" => {
            let rtk = bundled_rtk().ok_or_else(|| {
                CoreError::InvalidDoc(
                    lang.tr("Terrain 内置 RTK 不可用", "Terrain-bundled RTK is unavailable")
                        .into(),
                )
            })?;
            if !run_bundled_check(&rtk, &["gain"], repo) {
                return Err(CoreError::InvalidDoc(
                    lang.tr(
                        &format!("内置 RTK 无法执行: {}", rtk.display()),
                        &format!("Bundled RTK cannot be executed: {}", rtk.display()),
                    )
                    .to_string(),
                ));
            }
            Ok(lang
                .tr(
                    &format!("RTK 已部署至 ~/.terrain/bin/rtk（详见 {manifest}）"),
                    &format!("RTK deployed to ~/.terrain/bin/rtk (see {manifest})"),
                )
                .to_string())
        }
        "tool-codegraph" => {
            let codegraph = bundled_codegraph().ok_or_else(|| {
                CoreError::InvalidDoc(
                    lang.tr(
                        "Terrain 内置 CodeGraph 不可用",
                        "Terrain-bundled CodeGraph is unavailable",
                    )
                    .into(),
                )
            })?;
            if tool_check_passes(repo, def) {
                Ok(lang
                    .tr(
                        &format!("CodeGraph 索引已就绪（Agent 路径见 {manifest}）"),
                        &format!("CodeGraph index is ready (agent path: see {manifest})"),
                    )
                    .to_string())
            } else {
                run_binary(repo, &codegraph, &["init", "-i"]).await?;
                Ok(lang
                    .tr(
                        &format!(
                            "CodeGraph 已初始化 .codegraph/（Agent 使用 ~/.terrain/bin/codegraph，见 {manifest}）"
                        ),
                        &format!(
                            "CodeGraph initialized .codegraph/ (agents use ~/.terrain/bin/codegraph, see {manifest})"
                        ),
                    )
                    .to_string())
            }
        }
        _ => Err(CoreError::InvalidDoc(
            lang.tr(
                &format!("未知内置工具 {}", def.id),
                &format!("Unknown bundled tool {}", def.id),
            )
            .to_string(),
        )),
    }
}

async fn run_binary(cwd: &Path, program: &Path, args: &[&str]) -> Result<()> {
    let output = crate::process::async_command(program)
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
    let output = crate::process::async_command(cmd)
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

/// Write `.terrain/.gitignore` + `.terrain/.gitattributes`. Normally already done
/// when `.terrain/` was scaffolded; this path exists to repair or upgrade them.
fn apply_terrain_git_policy(repo: &Path) -> Result<String> {
    use crate::git_policy::{ensure_git_policy, PolicyOutcome};

    let lang = crate::language::current_language();
    let root = crate::registry::knowledge_root_for_repo(repo);
    let report = ensure_git_policy(&root)?;

    let written: Vec<&str> = report
        .iter()
        .filter(|r| matches!(r.outcome, PolicyOutcome::Created | PolicyOutcome::Upgraded))
        .map(|r| r.name)
        .collect();
    let user_owned: Vec<&str> = report
        .iter()
        .filter(|r| r.outcome == PolicyOutcome::UserOwned)
        .map(|r| r.name)
        .collect();

    let mut parts = Vec::new();
    if !written.is_empty() {
        parts.push(
            lang.tr(
                &format!("已写入 .terrain/{{{}}}", written.join(", ")),
                &format!("Wrote .terrain/{{{}}}", written.join(", ")),
            )
            .to_string(),
        );
    }
    if !user_owned.is_empty() {
        parts.push(
            lang.tr(
                &format!(
                    "跳过自定义文件 .terrain/{{{}}}（无 Terrain 标记）",
                    user_owned.join(", ")
                ),
                &format!(
                    "Skipped custom file .terrain/{{{}}} (no Terrain marker)",
                    user_owned.join(", ")
                ),
            )
            .to_string(),
        );
    }
    if parts.is_empty() {
        parts.push(lang.tr("已是最新", "Already up to date").into());
    }
    Ok(parts.join("；"))
}

fn apply_gitignore(repo: &Path, patterns: &[String]) -> Result<String> {
    let lang = crate::language::current_language();
    if gitignore_has_patterns(repo, patterns) {
        return Ok(lang.tr("已存在", "Already present").into());
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
    Ok(lang
        .tr(
            &format!("追加 {} 条规则", patterns.len()),
            &format!("Appended {} rule(s)", patterns.len()),
        )
        .to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn keeps_user_edits_and_user_files_while_upgrading_managed_ones() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let dest = tmp.path().join("dest");
        write(&source.join("SKILL.md"), "v1");
        write(&source.join("references/phase1.md"), "ref-v1");

        sync_skill_dir(&source, &dest, false).unwrap();
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "v1");

        // User edits one managed file and adds a file of their own.
        write(&dest.join("SKILL.md"), "my custom skill");
        write(&dest.join("notes.md"), "my notes");
        // Terrain ships a new version of both managed files.
        write(&source.join("SKILL.md"), "v2");
        write(&source.join("references/phase1.md"), "ref-v2");

        let report = sync_skill_dir(&source, &dest, false).unwrap();
        assert_eq!(report.preserved, 1);
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "my custom skill");
        assert_eq!(
            fs::read_to_string(dest.join("references/phase1.md")).unwrap(),
            "ref-v2"
        );
        assert_eq!(fs::read_to_string(dest.join("notes.md")).unwrap(), "my notes");
    }

    #[test]
    fn reinstall_overrides_edits_but_still_keeps_user_files() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let dest = tmp.path().join("dest");
        write(&source.join("SKILL.md"), "v1");
        sync_skill_dir(&source, &dest, false).unwrap();
        write(&dest.join("SKILL.md"), "my custom skill");
        write(&dest.join("notes.md"), "my notes");
        write(&source.join("SKILL.md"), "v2");

        sync_skill_dir(&source, &dest, true).unwrap();
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "v2");
        assert_eq!(fs::read_to_string(dest.join("notes.md")).unwrap(), "my notes");
    }

    #[test]
    fn removes_only_untouched_managed_files_dropped_from_source() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let dest = tmp.path().join("dest");
        write(&source.join("a.md"), "a");
        write(&source.join("b.md"), "b");
        sync_skill_dir(&source, &dest, false).unwrap();

        fs::remove_file(source.join("a.md")).unwrap();
        sync_skill_dir(&source, &dest, false).unwrap();
        assert!(!dest.join("a.md").exists());
        assert_eq!(fs::read_to_string(dest.join("b.md")).unwrap(), "b");
    }

    #[test]
    fn first_deploy_into_an_existing_directory_does_not_clobber_it() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let dest = tmp.path().join("dest");
        write(&source.join("SKILL.md"), "from terrain");
        // Pre-existing directory from before the manifest existed.
        write(&dest.join("SKILL.md"), "hand written");
        write(&dest.join("extra.md"), "keep me");

        sync_skill_dir(&source, &dest, false).unwrap();
        assert_eq!(fs::read_to_string(dest.join("SKILL.md")).unwrap(), "hand written");
        assert_eq!(fs::read_to_string(dest.join("extra.md")).unwrap(), "keep me");
    }
}
