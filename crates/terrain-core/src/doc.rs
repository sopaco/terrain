use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{CoreError, Result};
use crate::schema::{DocFrontmatter, DocType};

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeDoc {
    pub path: String,
    pub frontmatter: DocFrontmatter,
    pub body: String,
}

/// Parse markdown with optional path context for Litho `human/` docs without frontmatter.
pub fn parse_markdown_at(content: &str, path: Option<&Path>) -> Result<(DocFrontmatter, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        if let Some(path) = path {
            return Ok((infer_frontmatter(path, content), content.to_string()));
        }
        return Err(CoreError::InvalidDoc(
            "document must start with YAML frontmatter".into(),
        ));
    }

    let rest = &trimmed[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| CoreError::InvalidDoc("unclosed frontmatter".into()))?;
    let yaml = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let frontmatter: DocFrontmatter = serde_yaml::from_str(yaml)?;
    Ok((frontmatter, body))
}

pub fn parse_markdown(content: &str) -> Result<(DocFrontmatter, String)> {
    parse_markdown_at(content, None)
}

fn infer_frontmatter(path: &Path, body: &str) -> DocFrontmatter {
    let project = infer_project_slug(path);
    let title = extract_markdown_title(body).or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
    });
    DocFrontmatter {
        doc_type: DocType::Human,
        project,
        module: None,
        title,
        source: None,
        refs: vec![],
        deps: vec![],
        extra: Default::default(),
    }
}

fn infer_project_slug(path: &Path) -> String {
    if let Some(repo) = repo_parent_of_terrain(path) {
        let repo_s = repo.display().to_string();
        if let Ok(entries) = crate::registry::load_registry() {
            if let Some(entry) = entries.iter().find(|e| e.repo_path == repo_s) {
                return entry.slug.clone();
            }
        }
        if let Some(name) = repo.file_name().and_then(|s| s.to_str()) {
            return name.to_string();
        }
    }
    "unknown".into()
}

fn repo_parent_of_terrain(path: &Path) -> Option<std::path::PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.file_name().and_then(|s| s.to_str()) == Some(".terrain") {
            return ancestor.parent().map(std::path::Path::to_path_buf);
        }
    }
    None
}

fn extract_markdown_title(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

pub fn render_markdown(frontmatter: &DocFrontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    Ok(format!("---\n{yaml}---\n\n{body}"))
}

pub fn read_doc(path: impl AsRef<Path>) -> Result<KnowledgeDoc> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)?;
    let (frontmatter, body) = parse_markdown_at(&content, Some(path))?;
    Ok(KnowledgeDoc {
        path: path.display().to_string(),
        frontmatter,
        body,
    })
}

pub fn write_doc(path: impl AsRef<Path>, frontmatter: &DocFrontmatter, body: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render_markdown(frontmatter, body)?)?;
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn write_json<T: serde::Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
