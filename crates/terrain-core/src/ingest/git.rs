use std::path::Path;

use walkdir::WalkDir;

use crate::doc::write_doc;
use crate::error::Result;
use crate::paths::KnowledgePaths;
use crate::render::{project_frontmatter, project_index_body};
use crate::schema::ProjectMeta;

pub struct GitScanner<'a> {
    paths: &'a KnowledgePaths,
    project_slug: &'a str,
}

impl<'a> GitScanner<'a> {
    pub fn new(paths: &'a KnowledgePaths, project_slug: &'a str) -> Self {
        Self {
            paths,
            project_slug,
        }
    }

    pub fn scan(&self, repo_path: &str) -> Result<usize> {
        let repo = Path::new(repo_path);
        let name = repo
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.project_slug.to_string());

        let tech_stack = detect_tech_stack(repo);
        let tree = summarize_tree(repo, 2);

        let meta = ProjectMeta {
            name,
            repo_path: repo_path.to_string(),
            owner: None,
            tech_stack,
        };

        let fm = project_frontmatter(self.project_slug, &meta);
        let body = project_index_body(&meta, &tree);
        write_doc(self.paths.project_index(self.project_slug), &fm, &body)?;
        Ok(1)
    }
}

fn detect_tech_stack(repo: &Path) -> Vec<String> {
    let mut stack = Vec::new();
    let markers = [
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js"),
        ("go.mod", "Go"),
        ("pyproject.toml", "Python"),
        ("pom.xml", "Java"),
        ("build.gradle", "Java/Kotlin"),
    ];

    for (file, label) in markers {
        if repo.join(file).exists() {
            stack.push(label.to_string());
        }
    }
    stack
}

fn summarize_tree(repo: &Path, max_depth: usize) -> String {
    let mut lines = Vec::new();
    for entry in WalkDir::new(repo)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if should_skip(path) {
            continue;
        }
        let depth = entry.depth();
        let indent = "  ".repeat(depth);
        let name = path
            .strip_prefix(repo)
            .ok()
            .and_then(|p| p.components().next_back())
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".into());
        if depth == 0 {
            continue;
        }
        let suffix = if entry.file_type().is_dir() { "/" } else { "" };
        lines.push(format!("{indent}- {name}{suffix}"));
        if lines.len() >= 80 {
            lines.push("  - …".into());
            break;
        }
    }
    if lines.is_empty() {
        "_Empty repository_".into()
    } else {
        lines.join("\n")
    }
}

fn should_skip(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_str(),
            Some(
                ".git" | "node_modules" | "target" | "dist" | ".svelte-kit" | ".terrain"
                    | ".DS_Store"
            )
        )
    })
}
