use std::path::Path;

use chrono::Utc;
use crate::doc::read_json;
use crate::freshness::{baseline_matches_head, git_snapshot};
use crate::path_portable::stored_repo_path;
use repomix_core::{OutputStyle, PackOptions, RepomixConfig, pack_with_options};

use crate::doc::write_json;
use crate::error::{CoreError, Result};
use crate::paths::KnowledgePaths;
use crate::schema::{AgentPackMeta, AssetGenerator};

use super::pack_read::{agent_pack_ready, invalidate_pack_text_cache, write_pack_file_index};

/// Architecture-oriented agent context — not a full code dump.
pub const AGENT_PACK_STRATEGY: &str = "architecture-context";

const AGENT_CONTEXT_HEADER: &str = "\
Terrain Agent Source Pack (repomix-core / architecture-context)
Purpose: Indexed snapshot of project source code for Ask-mode retrieval.
Use grep_agent_pack and read_agent_pack_file on demand — never load this entire file into LLM context.
Auto-packed on first Ask when missing; use 重建源码索引 in the Terrain UI to refresh after large codebase changes.
";

/// Paths excluded from agent context to reduce noise and refresh cost.
fn architecture_ignore_patterns() -> Vec<String> {
    [
        "**/test/**",
        "**/tests/**",
        "**/__tests__/**",
        "**/*_test.*",
        "**/*.test.*",
        "**/*.spec.*",
        "**/fixtures/**",
        "**/__snapshots__/**",
        "**/migrations/**",
        "**/generated/**",
        "**/vendor/**",
        "**/node_modules/**",
        "**/target/**",
        "**/dist/**",
        "**/build/**",
        "**/coverage/**",
        "**/.git/**",
        "**/.terrain/**",
        "**/.litho-agent/**",
        "**/.sdd-agent/**",
        "**/*.lock",
        "**/*.min.js",
        "**/*.min.css",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentPackReport {
    pub project_slug: String,
    pub output_path: String,
    pub meta_path: String,
    pub total_files: usize,
    pub total_tokens: usize,
    pub skipped: bool,
}

/// True when pack exists and its Git baseline matches current HEAD (ignores dirty working tree).
///
/// Use this to decide whether repomix packing can be skipped. A dirty tree may still differ from
/// the last pack on disk; freshness scoring and Ask trust blocks surface that separately.
pub fn agent_pack_synced_with_head(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> bool {
    if !agent_pack_ready(paths, project_slug) {
        return false;
    }
    let Ok(meta) = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug)) else {
        return false;
    };
    baseline_matches_head(repo_path, meta.baseline_git_head.as_deref())
}

/// True when pack matches current HEAD and the working tree is clean (excluding `.terrain/`).
pub fn agent_pack_fresh(paths: &KnowledgePaths, project_slug: &str, repo_path: &str) -> bool {
    if !agent_pack_synced_with_head(paths, project_slug, repo_path) {
        return false;
    }
    let git = git_snapshot(repo_path);
    !git.is_git_repo || !git.dirty
}

fn report_from_meta(
    paths: &KnowledgePaths,
    project_slug: &str,
    meta: &AgentPackMeta,
) -> AgentPackReport {
    AgentPackReport {
        project_slug: project_slug.to_string(),
        output_path: paths.agent_pack_main(project_slug).display().to_string(),
        meta_path: paths.agent_pack_meta(project_slug).display().to_string(),
        total_files: meta.total_files,
        total_tokens: meta.total_tokens,
        skipped: true,
    }
}

/// Pack only when missing or Git baseline drifted; otherwise return existing meta.
pub async fn maybe_pack_agent_assets(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<AgentPackReport> {
    if agent_pack_synced_with_head(paths, project_slug, repo_path) {
        let meta = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug))?;
        return Ok(report_from_meta(paths, project_slug, &meta));
    }
    pack_agent_assets(paths, project_slug, repo_path).await
}

pub async fn pack_agent_assets(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<AgentPackReport> {
    let repo = Path::new(repo_path);
    if !repo.is_dir() {
        return Err(CoreError::InvalidDoc(format!(
            "repository path is not a directory: {repo_path}"
        )));
    }

    paths.ensure_project_layout(project_slug)?;
    let agent_dir = paths.agent_pack_dir(project_slug);
    std::fs::create_dir_all(&agent_dir)?;

    let output_path = paths.agent_pack_main(project_slug);
    let meta_path = paths.agent_pack_meta(project_slug);

    let mut config = RepomixConfig::default();
    config.output.style = OutputStyle::Markdown;
    config.output.file_path = output_path.display().to_string();
    config.output.header_text = Some(AGENT_CONTEXT_HEADER.into());
    config.output.compress = true;
    config.output.remove_comments = true;
    config.output.show_line_numbers = true;
    config.output.top_files_length = 20;
    config.ignore.custom_ignore = architecture_ignore_patterns();

    let options = PackOptions::new(repo.into()).with_config(config);
    let result = pack_with_options(options)
        .await
        .map_err(|e| CoreError::Pack(e.to_string()))?;

    let content = result
        .output_contents
        .first()
        .cloned()
        .unwrap_or_default();
    if content.is_empty() && output_path.is_file() {
        // repomix wrote directly to disk
    } else if !content.is_empty() {
        std::fs::write(&output_path, &content)?;
    }

    let pack_text = if !content.is_empty() {
        content
    } else {
        std::fs::read_to_string(&output_path).unwrap_or_default()
    };
    if !pack_text.is_empty() {
        let _ = write_pack_file_index(&output_path, &pack_text);
        invalidate_pack_text_cache(&output_path);
    }

    let baseline_git_head = git_snapshot(repo_path).head;

    let meta = AgentPackMeta {
        project: project_slug.to_string(),
        repo_path: stored_repo_path(repo),
        generator: AssetGenerator::RepomixCore,
        pack_strategy: AGENT_PACK_STRATEGY.into(),
        output_file: "repomix.md".into(),
        total_files: result.total_files,
        total_tokens: result.total_tokens,
        total_characters: result.total_characters,
        top_files_by_tokens: result
            .top_files_by_tokens
            .into_iter()
            .take(20)
            .map(|(path, tokens)| crate::schema::TokenHeavyFile { path, tokens })
            .collect(),
        directory_structure: result.directory_structure,
        synced_at: Utc::now().to_rfc3339(),
        baseline_git_head,
    };
    write_json(&meta_path, &meta)?;

    Ok(AgentPackReport {
        project_slug: project_slug.to_string(),
        output_path: output_path.display().to_string(),
        meta_path: meta_path.display().to_string(),
        total_files: meta.total_files,
        total_tokens: meta.total_tokens,
        skipped: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::TokenHeavyFile;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn init_git_repo(repo: &Path) -> String {
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .expect("git init");
        Command::new("git")
            .args(["config", "user.email", "t@test.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(repo)
            .output()
            .unwrap();
        fs::write(repo.join("src.rs"), "fn main() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .output()
            .unwrap();
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(repo)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    }

    fn write_pack_assets(paths: &KnowledgePaths, slug: &str, repo_path: &str, head: &str) {
        fs::write(paths.agent_pack_main(slug), "# pack\n").unwrap();
        let meta = AgentPackMeta {
            project: slug.to_string(),
            repo_path: repo_path.to_string(),
            generator: AssetGenerator::RepomixCore,
            pack_strategy: AGENT_PACK_STRATEGY.into(),
            output_file: "repomix.md".into(),
            total_files: 1,
            total_tokens: 10,
            total_characters: 100,
            top_files_by_tokens: vec![TokenHeavyFile {
                path: "src.rs".into(),
                tokens: 10,
            }],
            directory_structure: "src.rs".into(),
            synced_at: Utc::now().to_rfc3339(),
            baseline_git_head: Some(head.to_string()),
        };
        write_json(&paths.agent_pack_meta(slug), &meta).unwrap();
    }

    #[test]
    fn pack_synced_with_head_ignores_dirty_working_tree() {
        let _lock = crate::registry::registry_test_lock();
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let head = init_git_repo(repo);
        let slug = "dirty-pack";
        crate::registry::register_project(slug, &repo.display().to_string()).unwrap();
        let paths = KnowledgePaths::new();
        paths.ensure_project_layout(slug).unwrap();
        write_pack_assets(&paths, slug, &repo.display().to_string(), &head);

        fs::write(repo.join("dirty.rs"), "x").unwrap();

        assert!(agent_pack_synced_with_head(
            &paths,
            slug,
            &repo.display().to_string()
        ));
        assert!(!agent_pack_fresh(&paths, slug, &repo.display().to_string()));
    }

    #[test]
    fn pack_not_synced_when_head_advanced() {
        let _lock = crate::registry::registry_test_lock();
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let head = init_git_repo(repo);
        let slug = "stale-pack";
        crate::registry::register_project(slug, &repo.display().to_string()).unwrap();
        let paths = KnowledgePaths::new();
        paths.ensure_project_layout(slug).unwrap();
        write_pack_assets(&paths, slug, &repo.display().to_string(), &head);

        fs::write(repo.join("next.rs"), "y").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "next"])
            .current_dir(repo)
            .output()
            .unwrap();

        assert!(!agent_pack_synced_with_head(
            &paths,
            slug,
            &repo.display().to_string()
        ));
    }
}
