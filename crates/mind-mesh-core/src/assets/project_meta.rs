//! Developer-defined project metadata (`mind-mesh-meta.json`) for Agent context generation.
//!
//! Repositories may place one or more `mind-mesh-meta.json` files. Before generating
//! `agent/context.md`, MindMesh collects referenced files and injects them into the LLM prompt.

use std::path::{Path, PathBuf};

use glob::glob;
use serde::{Deserialize, Serialize};

use crate::doc::write_json;
use crate::error::{CoreError, Result};
use crate::paths::KnowledgePaths;
use crate::repo_walk::{discover_repo_walk, is_path_gitignored};

pub const META_FILENAME: &str = "mind-mesh-meta.json";
const DEFAULT_INPUT_MAX_CHARS: usize = 3_500;
const PROMPT_BUNDLE_MAX_CHARS: usize = 10_000;
const KNOWLEDGE_DIR: &str = "knowledge";
const KNOWLEDGE_MAX_FILES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetaFile {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub inputs: Vec<MetaInputSpec>,
    #[serde(default)]
    pub hints: MetaHints,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInputSpec {
    pub label: String,
    #[serde(flatten)]
    pub source: MetaInputSource,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "default_input_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetaInputSource {
    File {
        path: String,
    },
    Glob {
        pattern: String,
    },
    Inline {
        content: String,
    },
}

fn default_input_max_chars() -> usize {
    DEFAULT_INPUT_MAX_CHARS
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaHints {
    #[serde(default)]
    pub module_roots: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedMetaInput {
    pub label: String,
    pub source: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetaBundle {
    pub meta_files: Vec<String>,
    pub inputs: Vec<CollectedMetaInput>,
    pub hints: MetaHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInputsManifest {
    pub collected_at: String,
    pub meta_files: Vec<String>,
    pub input_count: usize,
    pub sources: Vec<MetaInputSourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInputSourceRef {
    pub label: String,
    pub source: String,
    pub chars: usize,
    pub truncated: bool,
}

/// Fast check for overview UI — only canonical locations (no repo walk).
pub fn has_repo_meta_configured(repo: &Path) -> bool {
    repo.join(META_FILENAME).is_file() || repo.join(".mind-mesh").join(META_FILENAME).is_file()
}

/// Discover `mind-mesh-meta.json` under the repository (root, `.mind-mesh/`, gitignore-aware walk).
pub fn discover_meta_files(repo: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    for candidate in [
        repo.join(META_FILENAME),
        repo.join(".mind-mesh").join(META_FILENAME),
    ] {
        if candidate.is_file() {
            found.push(candidate);
        }
    }

    for entry in discover_repo_walk(repo).filter_map(|e| e.ok()) {
        let path = entry.path();
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) != Some(META_FILENAME) {
            continue;
        }
        if !found.iter().any(|p| p == path) {
            found.push(path.to_path_buf());
        }
    }

    found.sort();
    found.dedup();
    found
}

/// Auto-scan `.mind-mesh/knowledge/**/*.md` for private domain knowledge.
pub fn collect_knowledge_dir_inputs(repo: &Path) -> Vec<CollectedMetaInput> {
    let dir = repo.join(".mind-mesh").join(KNOWLEDGE_DIR);
    if !dir.is_dir() {
        return Vec::new();
    }

    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&dir)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "md" || ext == "markdown")
        })
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    files
        .into_iter()
        .take(KNOWLEDGE_MAX_FILES)
        .filter_map(|path| {
            let rel = path
                .strip_prefix(repo)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            let content = std::fs::read_to_string(&path).ok()?;
            let (body, truncated) = truncate_chars(&content, DEFAULT_INPUT_MAX_CHARS);
            Some(CollectedMetaInput {
                label: format!("Private knowledge ({})", rel),
                source: path.display().to_string(),
                content: body,
                truncated,
            })
        })
        .collect()
}

/// Load and merge all discovered meta files, then resolve file/glob/inline inputs.
pub fn collect_project_meta(repo: &Path) -> Result<ProjectMetaBundle> {
    let meta_files = discover_meta_files(repo);
    let mut merged_inputs = Vec::new();
    let mut hints = MetaHints::default();
    let mut resolved = Vec::new();

    for meta_path in &meta_files {
        let raw = std::fs::read_to_string(meta_path)?;
        let file: ProjectMetaFile = serde_json::from_str(&raw).map_err(|e| {
            CoreError::InvalidDoc(format!(
                "invalid {} at {}: {e}",
                META_FILENAME,
                meta_path.display()
            ))
        })?;
        if !file.hints.module_roots.is_empty() {
            hints.module_roots = file.hints.module_roots.clone();
        }
        if !file.hints.notes.is_empty() {
            if !hints.notes.is_empty() {
                hints.notes.push_str("\n\n");
            }
            hints.notes.push_str(&file.hints.notes);
        }
        for spec in file.inputs {
            merged_inputs.push((meta_path.clone(), spec));
        }
    }

    for (meta_path, spec) in merged_inputs {
        match resolve_input(repo, &meta_path, &spec) {
            Ok(items) => resolved.extend(items),
            Err(_) if spec.optional => {}
            Err(e) => return Err(e),
        }
    }

    resolved.extend(collect_knowledge_dir_inputs(repo));

    Ok(ProjectMetaBundle {
        meta_files: meta_files
            .into_iter()
            .map(|p| p.display().to_string())
            .collect(),
        inputs: resolved,
        hints,
    })
}

fn resolve_input(
    repo: &Path,
    meta_path: &Path,
    spec: &MetaInputSpec,
) -> Result<Vec<CollectedMetaInput>> {
    match &spec.source {
        MetaInputSource::Inline { content } => Ok(vec![CollectedMetaInput {
            label: spec.label.clone(),
            source: format!("inline ({})", meta_path.display()),
            content: truncate_chars(content, spec.max_chars).0,
            truncated: content.chars().count() > spec.max_chars,
        }]),
        MetaInputSource::File { path } => {
            let abs = resolve_repo_path(repo, meta_path, path)?;
            if !abs.is_file() {
                return Err(CoreError::InvalidDoc(format!(
                    "meta input file not found: {path} (resolved {})",
                    abs.display()
                )));
            }
            let content = std::fs::read_to_string(&abs)?;
            let (body, truncated) = truncate_chars(&content, spec.max_chars);
            Ok(vec![CollectedMetaInput {
                label: spec.label.clone(),
                source: abs.display().to_string(),
                content: body,
                truncated,
            }])
        }
        MetaInputSource::Glob { pattern } => {
            let base = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
            let pattern_path = resolve_repo_path(repo, meta_path, pattern)?;
            let glob_pattern = pattern_path.display().to_string();
            let mut out = Vec::new();

            for entry in glob(&glob_pattern).map_err(|e| {
                CoreError::InvalidDoc(format!("invalid glob pattern {pattern}: {e}"))
            })? {
                let path = entry.map_err(|e| CoreError::InvalidDoc(e.to_string()))?;
                if !path.is_file() || is_path_gitignored(repo, &path) {
                    continue;
                }
                let rel = path
                    .strip_prefix(&base)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.display().to_string());
                let content = std::fs::read_to_string(&path)?;
                let (body, truncated) = truncate_chars(&content, spec.max_chars);
                out.push(CollectedMetaInput {
                    label: format!("{} ({})", spec.label, rel),
                    source: path.display().to_string(),
                    content: body,
                    truncated,
                });
            }

            if out.is_empty() && !spec.optional {
                return Err(CoreError::InvalidDoc(format!(
                    "meta glob matched no files: {pattern}"
                )));
            }
            Ok(out)
        }
    }
}

fn resolve_repo_path(repo: &Path, meta_path: &Path, raw: &str) -> Result<PathBuf> {
    let trimmed = raw.trim();
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    // Paths relative to repository root by default.
    let from_root = repo.join(trimmed);
    if from_root.exists() {
        return Ok(from_root);
    }
    // Fallback: relative to the meta file directory.
    if let Some(parent) = meta_path.parent() {
        let from_meta = parent.join(trimmed);
        if from_meta.exists() {
            return Ok(from_meta);
        }
    }
    Ok(from_root)
}

fn truncate_chars(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_string(), false);
    }
    let cut: String = text.chars().take(max).collect();
    (format!("{cut}\n\n…"), true)
}

/// Format collected inputs for injection into the agent-context LLM prompt.
pub fn format_meta_bundle_for_prompt(bundle: &ProjectMetaBundle) -> String {
    if bundle.inputs.is_empty() && bundle.hints.notes.is_empty() && bundle.hints.module_roots.is_empty()
    {
        return String::new();
    }

    let mut parts = Vec::new();

    if !bundle.hints.module_roots.is_empty() {
        parts.push(format!(
            "### Hints: module roots\n{}",
            bundle
                .hints
                .module_roots
                .iter()
                .map(|r| format!("- `{r}`"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !bundle.hints.notes.is_empty() {
        parts.push(format!("### Hints: notes\n{}", bundle.hints.notes.trim()));
    }

    for input in &bundle.inputs {
        parts.push(format!(
            "### {} (source: `{}`{})\n\n{}",
            input.label,
            input.source,
            if input.truncated { ", truncated" } else { "" },
            input.content.trim()
        ));
    }

    let mut body = parts.join("\n\n");
    if body.chars().count() > PROMPT_BUNDLE_MAX_CHARS {
        body = truncate_chars(&body, PROMPT_BUNDLE_MAX_CHARS).0;
    }
    body
}

pub fn format_meta_bundle_markdown(bundle: &ProjectMetaBundle) -> String {
    let mut md = String::from(
        "# Developer Meta Inputs\n\n\
         Compiled from `mind-mesh-meta.json` before Agent context generation.\n\n",
    );

    if !bundle.meta_files.is_empty() {
        md.push_str("## Meta files\n\n");
        for f in &bundle.meta_files {
            md.push_str(&format!("- `{f}`\n"));
        }
        md.push('\n');
    }

    if !bundle.hints.module_roots.is_empty() || !bundle.hints.notes.is_empty() {
        md.push_str("## Hints\n\n");
        if !bundle.hints.module_roots.is_empty() {
            md.push_str("**Module roots:**\n");
            for r in &bundle.hints.module_roots {
                md.push_str(&format!("- `{r}`\n"));
            }
            md.push('\n');
        }
        if !bundle.hints.notes.is_empty() {
            md.push_str(&format!("{}\n\n", bundle.hints.notes.trim()));
        }
    }

    for input in &bundle.inputs {
        md.push_str(&format!("## {}\n\n", input.label));
        md.push_str(&format!(
            "_Source: `{}`{}_\n\n",
            input.source,
            if input.truncated { " (truncated)" } else { "" }
        ));
        md.push_str(input.content.trim());
        md.push_str("\n\n");
    }

    md
}

/// Write `agent/meta-inputs.md` and `agent/meta-inputs-manifest.json`.
pub fn persist_meta_inputs(
    paths: &KnowledgePaths,
    project_slug: &str,
    bundle: &ProjectMetaBundle,
) -> Result<()> {
    let agent_dir = paths.agent_pack_dir(project_slug);
    std::fs::create_dir_all(&agent_dir)?;

    let md_path = agent_dir.join("meta-inputs.md");
    let md_body = format_meta_bundle_markdown(bundle);
    let write_md = std::fs::read_to_string(&md_path)
        .ok()
        .is_none_or(|existing| existing != md_body);
    if write_md {
        std::fs::write(&md_path, &md_body)?;
    }

    let manifest = MetaInputsManifest {
        collected_at: chrono::Utc::now().to_rfc3339(),
        meta_files: bundle.meta_files.clone(),
        input_count: bundle.inputs.len(),
        sources: bundle
            .inputs
            .iter()
            .map(|i| MetaInputSourceRef {
                label: i.label.clone(),
                source: i.source.clone(),
                chars: i.content.chars().count(),
                truncated: i.truncated,
            })
            .collect(),
    };
    write_json(agent_dir.join("meta-inputs-manifest.json"), &manifest)?;
    Ok(())
}

pub fn meta_inputs_ready(paths: &KnowledgePaths, project_slug: &str) -> bool {
    paths
        .agent_pack_dir(project_slug)
        .join("meta-inputs.md")
        .is_file()
}

/// Summary for project overview (`ready`, one-line summary).
pub fn meta_inputs_status(paths: &KnowledgePaths, project_slug: &str) -> (bool, String) {
    let manifest_path = paths
        .agent_pack_dir(project_slug)
        .join("meta-inputs-manifest.json");
    if let Ok(m) = crate::doc::read_json::<MetaInputsManifest>(&manifest_path) {
        let summary = if m.meta_files.is_empty() {
            "无 mind-mesh-meta.json".into()
        } else if m.input_count == 0 {
            format!("{} meta 文件 · 0 inputs", m.meta_files.len())
        } else {
            format!(
                "{} inputs · {} meta 文件",
                m.input_count,
                m.meta_files.len()
            )
        };
        return (meta_inputs_ready(paths, project_slug), summary);
    }
    (false, "未收集（生成 Agent 上下文时写入）".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn collects_file_and_inline_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();

        fs::create_dir_all(repo.join("docs")).unwrap();
        fs::write(
            repo.join("docs/module.md"),
            "# Core\n\nHandles packing.",
        )
        .unwrap();
        fs::write(
            repo.join(META_FILENAME),
            r#"{
  "version": 1,
  "inputs": [
    { "label": "Modules", "type": "file", "path": "docs/module.md" },
    { "label": "Note", "type": "inline", "content": "Use Tokio." }
  ],
  "hints": { "module_roots": ["crates/"], "notes": "Rust workspace" }
}"#,
        )
        .unwrap();

        let bundle = collect_project_meta(repo).unwrap();
        assert_eq!(bundle.inputs.len(), 2);
        assert!(bundle.inputs[0].content.contains("packing"));
        assert!(format_meta_bundle_for_prompt(&bundle).contains("Use Tokio."));
    }
}
