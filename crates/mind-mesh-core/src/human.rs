use std::path::Path;

use walkdir::WalkDir;

use crate::error::{CoreError, Result};
use crate::paths::KnowledgePaths;
use crate::schema::HumanDocEntry;

pub fn list_human_docs(paths: &KnowledgePaths, project_slug: &str) -> Result<Vec<HumanDocEntry>> {
    let mut entries = list_human_section(paths, project_slug)?;
    entries.extend(list_agent_docs(paths, project_slug)?);
    entries.extend(list_structured_docs(paths, project_slug)?);
    entries.sort_by(|a, b| {
        a.section
            .cmp(&b.section)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });
    Ok(entries)
}

fn list_human_section(paths: &KnowledgePaths, project_slug: &str) -> Result<Vec<HumanDocEntry>> {
    let human_dir = paths.human_docs_dir(project_slug);
    if !human_dir.is_dir() {
        return Ok(Vec::new());
    }

    let project_root = paths.project_dir(project_slug);
    let mut entries = Vec::new();

    for entry in WalkDir::new(&human_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(&project_root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());

        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_string();

        entries.push(HumanDocEntry {
            path: path.display().to_string(),
            title,
            relative_path: relative,
            section: "human".into(),
        });
    }

    Ok(entries)
}

fn list_structured_docs(paths: &KnowledgePaths, project_slug: &str) -> Result<Vec<HumanDocEntry>> {
    let project_root = paths.project_dir(project_slug);
    let mut entries = Vec::new();

    // OpenAPI-derived docs only; module maps come from developer meta + Agent context LLM.
    for subdir in ["interfaces", "routes", "events"] {
        let base = project_root.join(subdir);
        if !base.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        {
            let path = entry.path();
            let relative = path
                .strip_prefix(&project_root)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());

            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("document")
                .to_string();

            entries.push(HumanDocEntry {
                path: path.display().to_string(),
                title,
                relative_path: relative,
                section: "structured".into(),
            });
        }
    }

    Ok(entries)
}

fn list_agent_docs(paths: &KnowledgePaths, project_slug: &str) -> Result<Vec<HumanDocEntry>> {
    let project_root = paths.project_dir(project_slug);
    let agent_dir = paths.agent_pack_dir(project_slug);
    if !agent_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    let candidates = [
        (
            paths.agent_context_main(project_slug),
            "架构上下文".to_string(),
            "agent/context.md",
        ),
        (
            agent_dir.join("meta-inputs.md"),
            "Developer Meta".to_string(),
            "agent/meta-inputs.md",
        ),
    ];

    for (path, title, relative) in candidates {
        if !path.is_file() {
            continue;
        }
        let relative_path = relative.to_string();
        entries.push(HumanDocEntry {
            path: path.display().to_string(),
            title,
            relative_path,
            section: "agent".into(),
        });
    }

    // Ignore repomix.md — too large for the documentation tree.
    let _ = project_root;
    Ok(entries)
}

pub fn read_human_doc(paths: &KnowledgePaths, project_slug: &str, relative_path: &str) -> Result<String> {
    let project_root = paths.project_dir(project_slug);
    let target = sanitize_relative(&project_root, relative_path)?;
    let human_dir = paths.human_docs_dir(project_slug);
    let agent_dir = paths.agent_pack_dir(project_slug);
    let human_base = human_dir.canonicalize().unwrap_or(human_dir);
    let agent_base = agent_dir.canonicalize().unwrap_or(agent_dir);
    if !target.starts_with(&human_base) && !target.starts_with(&agent_base) {
        return Err(CoreError::InvalidDoc(format!(
            "path is outside human/agent docs: {relative_path}"
        )));
    }
    std::fs::read_to_string(&target).map_err(CoreError::from)
}

fn sanitize_relative(base: &Path, relative: &str) -> Result<std::path::PathBuf> {
    let rel = relative.trim_start_matches('/');
    let joined = base.join(rel);
    let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let canonical = joined.canonicalize().unwrap_or(joined);
    if !canonical.starts_with(&canonical_base) {
        return Err(CoreError::InvalidDoc(format!("invalid path: {relative}")));
    }
    Ok(canonical)
}
