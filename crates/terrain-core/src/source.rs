use std::path::{Path, PathBuf};

use crate::assets::read_agent_pack_file;
use crate::doc::read_json;
use crate::error::{CoreError, Result};
use crate::human::read_human_doc;
use crate::paths::KnowledgePaths;
use crate::project::resolve_project_repo_path;
use crate::schema::{AgentPackMeta, SourceSlice};
use crate::search::read_doc_at_in_project;

fn is_path_within_repo(file: &Path, repo: &Path) -> bool {
    if !file.is_file() {
        return false;
    }
    let file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    if file.starts_with(&repo) {
        return true;
    }
    let private_repo = PathBuf::from("/private").join(&repo);
    if file.starts_with(&private_repo) {
        return true;
    }
    // macOS: /var → /private/var while repo may be under /Users/...
    if let Ok(stripped) = file.strip_prefix("/private") {
        if stripped.starts_with(&repo) {
            return true;
        }
    }
    false
}

/// Resolve a citation path to a file under `repo` (relative, absolute, or repo-prefixed).
fn resolve_source_file_in_repo(repo: &Path, file_path: &str) -> Result<PathBuf> {
    let trimmed = file_path.trim();
    let path = Path::new(trimmed);

    if path.is_absolute() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if is_path_within_repo(&canonical, repo) {
            return Ok(canonical);
        }
        return Err(CoreError::InvalidDoc(format!(
            "source file not found: {file_path}"
        )));
    }

    let normalized = normalize_doc_ref(file_path);
    let repo_canon = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let repo_prefix = repo_canon.to_string_lossy();

    let rel = if let Some(rest) = normalized.strip_prefix(repo_prefix.as_ref()) {
        rest.trim_start_matches('/').trim_start_matches('\\')
    } else {
        normalized.as_str()
    };

    let target = repo.join(rel);
    if !is_path_within_repo(&target, repo) {
        return Err(CoreError::InvalidDoc(format!(
            "source file not found: {file_path}"
        )));
    }
    Ok(target.canonicalize().unwrap_or(target))
}

pub fn read_source_slice(
    repo_path: &str,
    file_path: &str,
    start_line: u32,
    end_line: u32,
) -> Result<SourceSlice> {
    let repo = Path::new(repo_path);
    if !repo.is_dir() {
        return Err(CoreError::InvalidDoc(format!(
            "repository path is not a directory: {repo_path}"
        )));
    }

    let canonical_file = resolve_source_file_in_repo(repo, file_path)?;
    let rel = canonical_file
        .strip_prefix(repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf()))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| normalize_doc_ref(file_path));

    let (start, end) = if start_line == 0 || end_line == 0 {
        (1, u32::MAX)
    } else if start_line > end_line {
        (end_line, start_line)
    } else {
        (start_line, end_line)
    };

    let content = std::fs::read_to_string(&canonical_file)?;
    let lines: Vec<&str> = content.lines().collect();
    let max_line = lines.len() as u32;
    let end = end.min(max_line);
    let start = start.max(1).min(end);

    let slice: String = lines
        .get((start - 1) as usize..end as usize)
        .unwrap_or(&[])
        .join("\n");

    Ok(SourceSlice {
        repo_path: repo_path.to_string(),
        file_path: rel.to_string(),
        start_line: start,
        end_line: end,
        content: slice,
    })
}

fn is_agent_pack_index(path: &str) -> bool {
    let path = path.trim_start_matches('/');
    path == "agent/repomix.md" || path.ends_with("/agent/repomix.md")
}

fn is_repomix_source_path(path: &str) -> bool {
    let path = path.trim_start_matches("./").trim_start_matches('/');
    path == "agent/repomix.md" || path.ends_with("/agent/repomix.md")
}

fn normalize_doc_ref(file_path: &str) -> String {
    file_path
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn knowledge_doc_relative_candidates(project_slug: &str, file_path: &str) -> Vec<String> {
    let p = project_relative_knowledge_path(project_slug, file_path);
    let mut out = Vec::new();
    let mut push = |value: String| {
        if !value.is_empty() && !out.iter().any(|existing| existing == &value) {
            out.push(value);
        }
    };

    push(p.clone());
    if p == "context.md" {
        push("agent/context.md".into());
    }
    out
}

fn project_relative_knowledge_path(_project_slug: &str, file_path: &str) -> String {
    let p = normalize_doc_ref(file_path);
    if p == "context.md" {
        return "agent/context.md".into();
    }
    if let Some(rest) = p.strip_prefix(".terrain/") {
        return rest.to_string();
    }
    // Legacy citation prefix (pre–repo-local layout)
    if let Some(idx) = p.find("/.terrain/") {
        return p[idx + "/.terrain/".len()..].to_string();
    }
    p
}

fn try_resolve_knowledge_markdown(
    paths: &KnowledgePaths,
    project_slug: &str,
    file_path: &str,
    repo_path: Option<&str>,
) -> Option<Result<SourceSlice>> {
    let p = normalize_doc_ref(file_path);
    if !p.ends_with(".md") || is_repomix_source_path(&p) {
        return None;
    }

    let project_rel = project_relative_knowledge_path(project_slug, file_path);
    if project_rel == "agent/context.md" {
        let context_path = paths.agent_context_main(project_slug);
        if context_path.is_file() {
            if let Ok(body) = std::fs::read_to_string(&context_path) {
                return Some(Ok(SourceSlice {
                    repo_path: repo_path.unwrap_or("").to_string(),
                    file_path: project_rel,
                    start_line: 0,
                    end_line: 0,
                    content: body,
                }));
            }
        }
    }

    if project_rel.starts_with("human/") || project_rel == "agent/context.md" {
        if let Ok(body) = read_human_doc(paths, project_slug, &project_rel) {
            return Some(Ok(SourceSlice {
                repo_path: repo_path.unwrap_or("").to_string(),
                file_path: project_rel,
                start_line: 0,
                end_line: 0,
                content: body,
            }));
        }
    }

    for rel in knowledge_doc_relative_candidates(project_slug, file_path) {
        if let Ok(doc) = read_doc_at_in_project(paths, &rel, Some(project_slug)) {
            return Some(Ok(SourceSlice {
                repo_path: repo_path.unwrap_or("").to_string(),
                file_path: rel,
                start_line: 0,
                end_line: 0,
                content: doc.body,
            }));
        }
    }

    None
}

fn agent_pack_index_slice(paths: &KnowledgePaths, project_slug: &str, repo_path: Option<&str>) -> Result<SourceSlice> {
    let meta_path = paths.agent_pack_meta(project_slug);
    let meta: AgentPackMeta = read_json(&meta_path)?;
    let top_files = meta
        .top_files_by_tokens
        .iter()
        .take(10)
        .map(|f| format!("- `{}` ({} tokens)", f.path, f.tokens))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "# Agent source pack\n\n\
        Terrain indexes project source in `agent/repomix.md` (Repomix). \
        Open a **specific source file** citation (e.g. `src/lib.rs:42`) to view code from the pack.\n\n\
        - **Synced:** {synced}\n\
        - **Files:** {files}\n\
        - **Tokens:** {tokens}\n\
        - **Strategy:** {strategy}\n\n\
        ## Top files by tokens\n\n{top_files}\n",
        synced = meta.synced_at,
        files = meta.total_files,
        tokens = meta.total_tokens,
        strategy = meta.pack_strategy,
        top_files = if top_files.is_empty() {
            "_No file breakdown in meta._".into()
        } else {
            top_files
        },
    );
    Ok(SourceSlice {
        repo_path: repo_path.unwrap_or(&meta.repo_path).to_string(),
        file_path: "agent/repomix.md".into(),
        start_line: 0,
        end_line: 0,
        content,
    })
}

fn repo_path_candidates(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: Option<&str>,
) -> Vec<String> {
    match resolve_project_repo_path(paths, project_slug, repo_path) {
        Ok(repo) if !repo.is_empty() => vec![repo],
        _ => Vec::new(),
    }
}

fn normalize_source_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .replace('\\', "/")
}

fn push_unique_path(out: &mut Vec<String>, value: &str) {
    let normalized = normalize_source_path(value);
    if !normalized.is_empty() && !out.iter().any(|existing| existing == &normalized) {
        out.push(normalized);
    }
}

/// Alternate paths to try when citations include project prefixes or common typos.
fn source_path_candidates(project_slug: &str, file_path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let normalized = normalize_source_path(file_path);
    push_unique_path(&mut out, &normalized);

    if let Some(rest) = normalized.strip_prefix(&format!("{project_slug}/")) {
        push_unique_path(&mut out, rest);
    }

    if let Some(file_name) = Path::new(&normalized).file_name().and_then(|s| s.to_str()) {
        push_unique_path(&mut out, file_name);
    }

    for (from, to) in [
        (".gradle.kt", ".gradle.kts"),
        (".gradle.kt", ".gradle"),
        (".gradle.kts", ".gradle"),
    ] {
        if let Some(stem) = normalized.strip_suffix(from) {
            push_unique_path(&mut out, &format!("{stem}{to}"));
        }
    }

    out
}

fn try_live_source_slice(
    repos: &[String],
    file_path: &str,
    start_line: u32,
    end_line: u32,
) -> Option<SourceSlice> {
    for repo in repos {
        if let Ok(slice) = read_source_slice(repo, file_path, start_line, end_line) {
            return Some(slice);
        }
    }
    None
}

fn try_live_source_slice_with_candidates(
    repos: &[String],
    candidates: &[String],
    start_line: u32,
    end_line: u32,
) -> Option<SourceSlice> {
    for path in candidates {
        if let Some(slice) = try_live_source_slice(repos, path, start_line, end_line) {
            return Some(slice);
        }
    }
    None
}

fn try_agent_pack_slice(
    pack: &Path,
    candidates: &[String],
    start_line: u32,
    end_line: u32,
) -> Result<crate::assets::AgentPackFileContent> {
    let start = if start_line == 0 { None } else { Some(start_line) };
    let end = if end_line == 0 { None } else { Some(end_line) };
    let mut last_err = None;
    for path in candidates {
        match read_agent_pack_file(pack, path, start, end) {
            Ok(slice) => return Ok(slice),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        CoreError::InvalidDoc(format!(
            "file not found in agent pack: {}",
            candidates.first().cloned().unwrap_or_default()
        ))
    }))
}

fn is_source_code_path(file_path: &str) -> bool {
    let p = normalize_doc_ref(file_path);
    !p.ends_with(".md") || p.ends_with("/repomix.md") || p == "agent/repomix.md"
}

/// Resolve a source citation for the UI source panel.
///
/// Prefers the live repository filesystem. When the cited path does not resolve directly,
/// uses the Repomix pack only to discover the canonical path, then retries the live file.
/// Pack content is returned only when no live repository is available.
pub fn resolve_source_citation(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: Option<&str>,
    file_path: &str,
    start_line: u32,
    end_line: u32,
) -> Result<SourceSlice> {
    if is_agent_pack_index(file_path) {
        return agent_pack_index_slice(paths, project_slug, repo_path);
    }

    if !is_source_code_path(file_path) {
        if let Some(result) = try_resolve_knowledge_markdown(paths, project_slug, file_path, repo_path)
        {
            return result;
        }
    }

    let path_candidates = source_path_candidates(project_slug, file_path);
    let repos = repo_path_candidates(paths, project_slug, repo_path);
    if let Some(slice) =
        try_live_source_slice_with_candidates(&repos, &path_candidates, start_line, end_line)
    {
        return Ok(slice);
    }

    if repos.is_empty() {
        return Err(CoreError::InvalidDoc(format!(
            "cannot resolve repository path for project '{project_slug}' — \
             configure the repo path in project settings or re-scan the project"
        )));
    }

    let pack = paths.agent_pack_main(project_slug);
    if !pack.is_file() {
        return Err(CoreError::InvalidDoc(format!(
            "source file not found: {file_path}"
        )));
    }

    let pack_file = try_agent_pack_slice(&pack, &path_candidates, start_line, end_line)?;

    if let Some(slice) = try_live_source_slice_with_candidates(
        &repos,
        &[pack_file.matched_path.clone()],
        start_line,
        end_line,
    ) {
        return Ok(slice);
    }

    let resolved_repo = repos.into_iter().next().unwrap_or_default();
    Ok(SourceSlice {
        repo_path: resolved_repo,
        file_path: pack_file.matched_path,
        start_line: pack_file.start_line,
        end_line: pack_file.end_line,
        content: pack_file.content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{self, registry_test_lock};

    #[test]
    fn reads_line_range() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("src/lib.rs");
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

        let slice = read_source_slice(
            dir.path().to_str().unwrap(),
            "src/lib.rs",
            2,
            3,
        )
        .unwrap();
        assert_eq!(slice.content, "line2\nline3");
    }

    struct RegistryTestGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        _registry_dir: tempfile::TempDir,
    }

    fn test_setup(slug: &str) -> (KnowledgePaths, String, std::path::PathBuf, RegistryTestGuard) {
        let lock = registry_test_lock();
        let registry_dir = tempfile::tempdir().unwrap();
        let registry_file = registry_dir.path().join("registry.json");
        unsafe {
            std::env::set_var("TERRAIN_REGISTRY_FILE", &registry_file);
        }
        let repo = registry_dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        registry::register_project(slug, &repo.display().to_string()).unwrap();
        let paths = KnowledgePaths::new();
        paths.ensure_project_layout(slug).unwrap();
        (
            paths,
            slug.to_string(),
            repo,
            RegistryTestGuard {
                _lock: lock,
                _registry_dir: registry_dir,
            },
        )
    }

    fn write_repo_file(repo: &std::path::Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn resolves_bare_context_md_from_agent_docs() {
        let (paths, slug, _, _guard) = test_setup("unit-test-context-md");
        std::fs::write(
            paths.agent_context_main(&slug),
            "---\ntitle: Context\n---\n\n# Architecture\n",
        )
        .unwrap();

        let slice = resolve_source_citation(&paths, &slug, None, "context.md", 0, 0).unwrap();
        assert_eq!(slice.file_path, "agent/context.md");
        assert!(slice.content.contains("# Architecture"));
    }

    #[test]
    fn prefers_live_repo_over_agent_pack() {
        let (paths, slug, repo, _guard) = test_setup("live-repo-priority");
        write_repo_file(&repo, "src/lib.rs", "live repo line\n");
        std::fs::write(
            paths.agent_pack_main(&slug),
            "# Repomix\n\n## Files\n\n### src/lib.rs (1 lines)\n\n```rust\n1: pack line\n```\n",
        )
        .unwrap();
        std::fs::write(
            paths.agent_pack_meta(&slug),
            format!(
                r#"{{"repo_path":"{}","synced_at":"now","total_files":1,"total_tokens":1,"pack_strategy":"test","top_files_by_tokens":[]}}"#,
                repo.display()
            ),
        )
        .unwrap();

        let slice = resolve_source_citation(
            &paths,
            &slug,
            Some(repo.to_str().unwrap()),
            "src/lib.rs",
            0,
            0,
        )
        .unwrap();
        assert!(slice.content.contains("live repo line"));
        assert!(!slice.content.contains("pack line"));
    }

    #[test]
    fn falls_back_to_agent_pack_when_live_file_missing() {
        let (paths, slug, repo, _guard) = test_setup("no-pack-fallback");
        std::fs::write(
            paths.agent_pack_main(&slug),
            "# Repomix\n\n## Files\n\n### src/missing.rs (1 lines)\n\n```rust\n1: pack only\n```\n",
        )
        .unwrap();
        std::fs::write(
            paths.agent_pack_meta(&slug),
            format!(
                r#"{{"repo_path":"{}","synced_at":"now","total_files":1,"total_tokens":1,"pack_strategy":"test","top_files_by_tokens":[]}}"#,
                repo.display()
            ),
        )
        .unwrap();

        let slice = resolve_source_citation(
            &paths,
            &slug,
            Some(repo.to_str().unwrap()),
            "src/missing.rs",
            0,
            0,
        )
        .unwrap();
        assert!(slice.content.contains("pack only"));
    }

    #[test]
    fn resolves_repo_from_index_frontmatter_when_hint_missing() {
        let (paths, slug, repo, _guard) = test_setup("index-frontmatter-repo");
        write_repo_file(&repo, "src/main.rs", "fn main() {}\n");
        std::fs::write(
            paths.project_index(&slug),
            format!(
                "---\ntype: project\ntitle: Test\nproject: {slug}\nsource: {}\n---\n\n# Project\n",
                repo.display()
            ),
        )
        .unwrap();

        let slice = resolve_source_citation(&paths, &slug, None, "src/main.rs", 0, 0).unwrap();
        assert!(slice.content.contains("fn main()"));
    }

    #[test]
    fn resolves_pack_alias_to_live_file() {
        let (paths, slug, repo, _guard) = test_setup("pack-alias-live");
        write_repo_file(&repo, "src/lib.rs", "live via alias\n");
        std::fs::write(
            paths.agent_pack_main(&slug),
            "# Repomix\n\n## Files\n\n### src/lib.rs (1 lines)\n\n```rust\n1: pack line\n```\n",
        )
        .unwrap();
        std::fs::write(
            paths.agent_pack_meta(&slug),
            format!(
                r#"{{"repo_path":"{}","synced_at":"now","total_files":1,"total_tokens":1,"pack_strategy":"test","top_files_by_tokens":[]}}"#,
                repo.display()
            ),
        )
        .unwrap();

        let slice = resolve_source_citation(
            &paths,
            &slug,
            Some(repo.to_str().unwrap()),
            "lib.rs",
            0,
            0,
        )
        .unwrap();
        assert!(slice.content.contains("live via alias"));
        assert!(!slice.content.contains("pack line"));
    }

    #[test]
    fn resolves_prefixed_citation_with_gradle_extension_variant() {
        let (paths, slug, repo, _guard) = test_setup("gradle-path-variant");
        write_repo_file(
            &repo,
            "au_home/build.gradle.kts",
            "plugins { id(\"android\") }\n",
        );
        std::fs::write(
            paths.agent_pack_main(&slug),
            "# Repomix\n\n## Files\n\n### au_home/build.gradle.kts (1 lines)\n\n```kotlin\n1: plugins { id(\"android\") }\n```\n",
        )
        .unwrap();
        std::fs::write(
            paths.agent_pack_meta(&slug),
            format!(
                r#"{{"repo_path":"{}","synced_at":"now","total_files":1,"total_tokens":1,"pack_strategy":"test","top_files_by_tokens":[]}}"#,
                repo.display()
            ),
        )
        .unwrap();

        let slice = resolve_source_citation(
            &paths,
            &slug,
            Some(repo.to_str().unwrap()),
            "android-au/au_home/build.gradle.kt",
            0,
            0,
        )
        .unwrap();
        assert!(slice.content.contains("plugins"));
        assert_eq!(slice.file_path, "au_home/build.gradle.kts");
    }

    #[test]
    fn resolves_repo_from_marker_hint_via_pack_meta() {
        let (paths, slug, repo, _guard) = test_setup("marker-hint-repo");
        write_repo_file(&repo, "src/lib.rs", "marker hint live\n");
        std::fs::write(
            paths.agent_pack_meta(&slug),
            r#"{"repo_path":".","synced_at":"now","total_files":1,"total_tokens":1,"pack_strategy":"test","top_files_by_tokens":[]}"#,
        )
        .unwrap();

        let slice = resolve_source_citation(&paths, &slug, Some("."), "src/lib.rs", 0, 0).unwrap();
        assert!(slice.content.contains("marker hint live"));
    }

    #[test]
    fn resolves_absolute_path_under_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/lib.rs"), "absolute path test\n").unwrap();
        let abs = repo.join("src/lib.rs");
        let slice = read_source_slice(repo.to_str().unwrap(), abs.to_str().unwrap(), 0, 0)
            .expect("absolute path under repo should resolve");
        assert!(slice.content.contains("absolute path test"));
    }
}
