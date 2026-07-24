use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::doc::{parse_markdown_at, read_doc};
use crate::error::Result;
use crate::paths::KnowledgePaths;
use crate::schema::DocType;

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub project: String,
    pub doc_type: DocType,
    pub title: Option<String>,
    pub snippet: String,
    pub score: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub project: Option<String>,
    pub doc_type: Option<DocType>,
    pub limit: usize,
}

pub struct KnowledgeSearch<'a> {
    paths: &'a KnowledgePaths,
}

impl<'a> KnowledgeSearch<'a> {
    pub fn new(paths: &'a KnowledgePaths) -> Self {
        Self { paths }
    }

    pub fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchHit>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let limit = if options.limit == 0 { 20 } else { options.limit };
        let mut hits = Vec::new();
        let project_roots = self.paths.indexed_project_roots();

        if project_roots.is_empty() {
            return Ok(hits);
        }

        for (slug, root) in project_roots {
            if let Some(project) = options.project.as_deref()
                && project != slug
            {
                continue;
            }

            for entry in WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| search_should_descend(e.path()))
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            {
                let path = entry.path();
                if !search_includes_markdown(path) {
                    continue;
                }

                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let (frontmatter, body) = match parse_markdown_at(&content, Some(path)) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if let Some(dt) = options.doc_type
                    && frontmatter.doc_type != dt
                {
                    continue;
                }

                let score = score_match(&q, &content, &body, &frontmatter.project);
                if score == 0 {
                    continue;
                }

                hits.push(SearchHit {
                    path: path.display().to_string(),
                    project: frontmatter.project.clone(),
                    doc_type: frontmatter.doc_type,
                    title: frontmatter.title.clone(),
                    snippet: snippet(&body, &q),
                    score,
                });
            }
        }

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let mut projects = Vec::new();
        let roots = self.paths.indexed_project_roots();
        let repo_paths = crate::registry::registry_repo_map().unwrap_or_default();

        for (slug, root) in roots {
            let index = root.join("index.md");
            if !index.is_file() {
                continue;
            }
            let doc = read_doc(&index)?;
            let repo_path = doc
                .frontmatter
                .source
                .as_deref()
                .and_then(|s| crate::path_portable::resolve_stored_repo_path(s, &slug))
                .or_else(|| repo_paths.get(&slug).cloned());
            projects.push(ProjectSummary {
                slug: slug.clone(),
                name: doc.frontmatter.title.unwrap_or(slug),
                path: index.display().to_string(),
                repo_path,
            });
        }

        projects.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(projects)
    }
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub slug: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
}

fn search_should_descend(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name == "repomix.md" {
        return false;
    }
    !matches!(
        name,
        "agent" | ".litho-agent" | ".sdd-agent" | ".meta" | "env"
    )
}

fn search_includes_markdown(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == "repomix.md") {
        return false;
    }
    if path
        .components()
        .any(|c| matches!(c.as_os_str().to_str(), Some("agent" | ".litho-agent" | ".sdd-agent")))
    {
        return path.file_name().is_some_and(|n| n == "context.md");
    }
    true
}

fn score_match(query: &str, full: &str, body: &str, project: &str) -> u32 {
    let mut score = 0;

    if contains_ascii_insensitive(project, query) {
        score += 50;
    }
    if contains_ascii_insensitive(full, query) {
        score += 30;
    }
    if contains_ascii_insensitive(body, query) {
        score += 20;
    }

    for token in query.split_whitespace() {
        if token.len() < 2 {
            continue;
        }
        if contains_ascii_insensitive(project, token) {
            score += 10;
        }
        if contains_ascii_insensitive(full, token) {
            score += 5;
        }
    }

    score
}

fn contains_ascii_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn snippet(body: &str, query: &str) -> String {
    let query = query.split_whitespace().next().unwrap_or(query);
    let body_l = body.to_lowercase();
    let query_l = query.to_lowercase();
    if let Some(idx) = body_l.find(&query_l) {
        let start = body.floor_char_boundary(idx.saturating_sub(40));
        let end = body.ceil_char_boundary((idx + query_l.len() + 80).min(body.len()));
        let slice = body[start..end].replace('\n', " ");
        return format!("…{}…", slice.trim());
    }

    body.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(120)
        .collect()
}

pub fn read_doc_at(paths: &KnowledgePaths, rel_or_abs: &str) -> Result<crate::doc::KnowledgeDoc> {
    read_doc_at_in_project(paths, rel_or_abs, None)
}

/// Strip repo-relative `.terrain/` prefixes so paths match knowledge-root layout.
fn normalize_knowledge_doc_rel(path: &str) -> String {
    let p = path.trim().trim_start_matches("./").trim_start_matches('/');
    if let Some(rest) = p.strip_prefix(".terrain/") {
        return rest.to_string();
    }
    if let Some(idx) = p.find("/.terrain/") {
        return p[idx + "/.terrain/".len()..].to_string();
    }
    if p == "context.md" {
        return "agent/context.md".to_string();
    }
    p.to_string()
}

/// Resolve a knowledge document path against registry-backed project roots.
pub fn read_doc_at_in_project(
    paths: &KnowledgePaths,
    rel_or_abs: &str,
    project_slug: Option<&str>,
) -> Result<crate::doc::KnowledgeDoc> {
    use std::collections::HashSet;

    let trimmed = rel_or_abs.trim();
    if trimmed.is_empty() {
        return Err(crate::error::CoreError::InvalidDoc(
            "document path is empty".into(),
        ));
    }

    let mut tried = HashSet::new();
    let mut push = |candidate: PathBuf, queue: &mut Vec<PathBuf>| {
        if tried.insert(candidate.clone()) {
            queue.push(candidate);
        }
    };

    let mut candidates = Vec::new();
    let path = Path::new(trimmed);

    if path.is_absolute() {
        push(path.to_path_buf(), &mut candidates);
    }

    let rel = normalize_knowledge_doc_rel(trimmed);

    if let Some(slug) = project_slug
        && let Ok(root) = paths.try_project_dir(slug) {
            push(root.join(&rel), &mut candidates);
        }

    if let Some(root) = paths.workspace_knowledge_root() {
        push(root.join(&rel), &mut candidates);
    }

    for (_slug, root) in paths.indexed_project_roots() {
        push(root.join(&rel), &mut candidates);
    }

    for candidate in candidates {
        if candidate.is_file() {
            return read_doc(&candidate);
        }
    }

    Err(crate::error::CoreError::InvalidDoc(format!(
        "document not found: {rel_or_abs}"
    )))
}

#[cfg(test)]
mod read_doc_tests {
    use super::*;
    use crate::registry::registry_test_lock;
    use crate::schema::{DocFrontmatter, DocType};
    use crate::write_doc;

    struct RegistryTestGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        _registry_dir: tempfile::TempDir,
    }

    fn test_setup(slug: &str) -> (KnowledgePaths, String, RegistryTestGuard) {
        let lock = registry_test_lock();
        let registry_dir = tempfile::tempdir().unwrap();
        let registry_file = registry_dir.path().join("registry.json");
        unsafe {
            std::env::set_var("TERRAIN_REGISTRY_FILE", &registry_file);
        }
        let repo = registry_dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        crate::registry::register_project(slug, &repo.display().to_string()).unwrap();
        let paths = KnowledgePaths::new();
        paths.ensure_project_layout(slug).unwrap();
        (
            paths,
            slug.to_string(),
            RegistryTestGuard {
                _lock: lock,
                _registry_dir: registry_dir,
            },
        )
    }

    #[test]
    fn resolves_project_relative_human_doc() {
        let (paths, slug, _guard) = test_setup("read-doc-test-proj");
        let human_dir = paths.human_docs_dir(&slug);
        let doc_path = human_dir.join("1.概述.md");
        let fm = DocFrontmatter {
            doc_type: DocType::Human,
            project: slug.clone(),
            title: Some("概述".into()),
            source: None,
            refs: vec![],
            deps: vec![],
            extra: Default::default(),
            module: None,
        };
        write_doc(&doc_path, &fm, "# 概述\n\nHello").unwrap();

        let doc = read_doc_at_in_project(&paths, "human/1.概述.md", Some(&slug)).unwrap();
        assert!(doc.body.contains("Hello"));
    }

    #[test]
    fn resolves_terrain_prefixed_agent_context() {
        let (paths, slug, _guard) = test_setup("read-doc-ctx-proj");
        let context_path = paths.agent_context_main(&slug);
        std::fs::create_dir_all(context_path.parent().unwrap()).unwrap();
        std::fs::write(&context_path, "# Agent context\n\nModules overview.").unwrap();

        let doc =
            read_doc_at_in_project(&paths, ".terrain/agent/context.md", Some(&slug)).unwrap();
        assert!(doc.body.contains("Modules overview"));
    }
}
