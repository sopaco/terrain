use std::path::Path;

use chrono::Utc;
use repomix_core::{OutputStyle, PackOptions, RepomixConfig, pack_with_options};

use crate::doc::write_json;
use crate::error::{CoreError, Result};
use crate::paths::KnowledgePaths;
use crate::schema::{AgentPackMeta, AssetGenerator};

/// Architecture-oriented agent context — not a full code dump.
pub const AGENT_PACK_STRATEGY: &str = "architecture-context";

const AGENT_CONTEXT_HEADER: &str = "\
MindMesh Agent Source Pack (repomix-rs / architecture-context)
Purpose: Indexed snapshot of project source code for Ask-mode retrieval.
Use grep_agent_pack and read_agent_pack_file on demand — never load this entire file into LLM context.
Auto-packed on first Ask when missing; use Pack Context in the UI to refresh after large codebase changes.
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
        "**/*.lock",
        "**/*.min.js",
        "**/*.min.css",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentPackReport {
    pub project_slug: String,
    pub output_path: String,
    pub meta_path: String,
    pub total_files: usize,
    pub total_tokens: usize,
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

    let meta = AgentPackMeta {
        project: project_slug.to_string(),
        repo_path: repo_path.to_string(),
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
    };
    write_json(&meta_path, &meta)?;

    Ok(AgentPackReport {
        project_slug: project_slug.to_string(),
        output_path: output_path.display().to_string(),
        meta_path: meta_path.display().to_string(),
        total_files: meta.total_files,
        total_tokens: meta.total_tokens,
    })
}
