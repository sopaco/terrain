use std::path::Path;

use chrono::Utc;
use crate::freshness::git_snapshot;

use crate::assets::project_meta::{
    collect_project_meta, format_meta_bundle_for_prompt, persist_meta_inputs,
};
use crate::doc::{read_doc, write_doc};
use crate::error::Result;
use crate::model_text::prepare_model_markdown;
use crate::path_portable::stored_repo_path;
use crate::paths::KnowledgePaths;
use crate::preset_skills::default_agent_arch_skill_dir;
use crate::schema::{AgentContextMeta, AgentContextStatus, DocFrontmatter, DocType};

pub fn agent_context_ready(paths: &KnowledgePaths, project_slug: &str) -> bool {
    let path = paths.agent_context_main(project_slug);
    if !path.is_file() {
        return false;
    }
    read_doc(&path)
        .ok()
        .map(|doc| {
            let body = doc.body.trim();
            body.len() >= 500 && body.matches("\n## ").count() >= 4
        })
        .unwrap_or(false)
}

/// True when context exists, meets the ready heuristic, and its baseline matches current HEAD.
pub fn agent_context_fresh(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> bool {
    if !agent_context_ready(paths, project_slug) {
        return false;
    }
    let git = git_snapshot(repo_path);
    if !git.is_git_repo || git.dirty {
        return false;
    }
    let ctx_meta =
        crate::doc::read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug)).ok();
    let pack_meta =
        crate::doc::read_json::<crate::schema::AgentPackMeta>(paths.agent_pack_meta(project_slug))
            .ok();
    let baseline = ctx_meta
        .as_ref()
        .and_then(|m| m.baseline_git_head.clone())
        .or_else(|| pack_meta.and_then(|m| m.baseline_git_head.clone()));
    match (baseline.as_deref(), git.head.as_deref()) {
        (Some(baseline), Some(head)) => baseline == head,
        _ => false,
    }
}

pub fn read_agent_context_status(paths: &KnowledgePaths, project_slug: &str) -> AgentContextStatus {
    let path = paths.agent_context_main(project_slug);
    if !path.is_file() {
        return AgentContextStatus {
            ready: false,
            path: path.display().to_string(),
            excerpt: None,
            generated_at: None,
            section_count: 0,
        };
    }

    let doc = match read_doc(&path) {
        Ok(doc) => doc,
        Err(_) => {
            return AgentContextStatus {
                ready: false,
                path: path.display().to_string(),
                excerpt: None,
                generated_at: None,
                section_count: 0,
            };
        }
    };

    let excerpt = Some(doc.body.chars().take(600).collect::<String>());
    let section_count = doc.body.matches("\n## ").count();

    let generated_at = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

    AgentContextStatus {
        ready: true,
        path: path.display().to_string(),
        excerpt,
        generated_at,
        section_count,
    }
}

pub fn build_agent_context_prompt(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<String> {
    let index = read_doc(paths.project_index(project_slug))?;
    let meta_path = paths.agent_pack_meta(project_slug);
    let pack_meta = crate::doc::read_json::<crate::schema::AgentPackMeta>(&meta_path).ok();

    let human_hint = read_doc(paths.human_docs_dir(project_slug).join("1.概述.md"))
        .ok()
        .map(|d| d.body.chars().take(1200).collect::<String>())
        .unwrap_or_default();

    let output_path = paths.agent_context_main(project_slug).display().to_string();
    let skill_dir = default_agent_arch_skill_dir();
    let skill_dir_display = skill_dir.display().to_string();
    let skill_excerpt = std::fs::read_to_string(skill_dir.join("SKILL.md"))
        .unwrap_or_default()
        .chars()
        .take(6000)
        .collect::<String>();

    let directory_structure = pack_meta
        .as_ref()
        .map(|m| m.directory_structure.as_str())
        .unwrap_or("(agent pack not generated — use index structure below)");

    let repo = Path::new(repo_path);
    let meta_bundle = collect_project_meta(repo)?;
    persist_meta_inputs(paths, project_slug, &meta_bundle)?;
    let meta_section = {
        let body = format_meta_bundle_for_prompt(&meta_bundle);
        if body.is_empty() {
            String::new()
        } else {
            format!(
                "## Developer meta (`terrain-meta.json`)\n\
                 The following was collected programmatically from repository meta config. \
                 **Use it as authoritative structured input** when writing 模块地图, 系统边界, and 代码映射索引. \
                 Supplement with grep_agent_pack discovery; do not invent modules that contradict this bundle.\n\n\
                 {body}\n\n"
            )
        }
    };

    Ok(format!(
        "Generate an Agent-facing architecture context document for project \"{project_slug}\".\n\n\
         Output file (write with absolute path): {output_path}\n\
         Repository: {repo_path}\n\n\
         ## Agent architecture skill (preloaded — do NOT call read_doc on skill files)\n\
         Skill directory: {skill_dir_display}\n\n\
         {skill_excerpt}\n\n\
         ## Requirements (macro layer — no code细节)\n\
         - NO function bodies, NO large code blocks, NO pasted grep output\n\
         - Do NOT use read_doc on preset_skills/ or SKILL.md — skill text is above\n\
         - Focus: architecture, modules, core flows, tech choices, system boundaries\n\
         - **Structured entries (模块/接口/边界)**: synthesize from Developer meta below + repomix discovery — not from fixed directory rules\n\
         - Module ↔ path mappings as **compact tables** (≤12 rows per table)\n\
         - 代码映射索引: file paths only — details live in agent/repomix.md\n\
         - **Hard limit: ≤14000 characters total** — tables & bullets over prose; trimmed to 16 KiB on save\n\
         - Keep 项目概览 + 架构设计 + 模块地图 dense; other sections concise\n\
         - Use grep_agent_pack only to discover paths, not to paste code\n\n\
         {meta_section}\
         ## Project index\n{index_body}\n\n\
         ## Directory structure (from agent pack)\n{directory_structure}\n\n\
         {human_section}\n\
         Write the complete markdown to the output path. Required sections:\n\
         1. ## 项目概览\n\
         2. ## 架构设计\n\
         3. ## 模块地图\n\
         4. ## 核心流程\n\
         5. ## 技术选型\n\
         6. ## 系统边界\n\
         7. ## 代码映射索引\n\n\
         Return ONLY the final markdown document in your reply. \
         Do not include reasoning, thinking, or commentary outside the document.\n",
        project_slug = project_slug,
        skill_dir_display = skill_dir_display,
        skill_excerpt = skill_excerpt,
        output_path = output_path,
        repo_path = repo_path,
        index_body = index.body,
        directory_structure = directory_structure,
        meta_section = meta_section,
        human_section = if human_hint.is_empty() {
            String::new()
        } else {
            format!("## Human doc excerpt (1.概述.md)\n{human_hint}\n\n")
        },
    ))
}

/// Normalize `## 1. Title` → `## Title` so section tools match the skill contract.
fn normalize_context_headings(body: &str) -> String {
    let re = regex::Regex::new(r"(?m)^##\s+\d+\.\s*").expect("context heading regex");
    re.replace_all(body, "## ").into_owned()
}

/// Persist generated agent context markdown and sidecar meta.
pub fn write_agent_context(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    body: &str,
) -> Result<AgentContextMeta> {
    std::fs::create_dir_all(paths.agent_pack_dir(project_slug))?;
    let path = paths.agent_context_main(project_slug);
    let body = normalize_context_headings(&prepare_model_markdown(body));
    let (body, _size_capped) = crate::assets::enforce_context_max_size(
        &body,
        crate::assets::AGENT_CONTEXT_SAVE_MAX_CHARS,
    );

    if body.trim().is_empty() {
        return Err(crate::error::CoreError::InvalidDoc(
            "agent context body is empty after sanitization".into(),
        ));
    }

    let fm = DocFrontmatter {
        doc_type: DocType::AgentContext,
        project: project_slug.to_string(),
        module: None,
        title: Some("Agent Architecture Context".into()),
        source: Some(stored_repo_path(Path::new(repo_path))),
        refs: vec![],
        deps: vec![],
        extra: serde_json::Map::new(),
    };
    write_doc(&path, &fm, &body)?;

    let meta = AgentContextMeta {
        project: project_slug.to_string(),
        repo_path: stored_repo_path(Path::new(repo_path)),
        output_file: "context.md".into(),
        generated_at: Utc::now().to_rfc3339(),
        section_count: body.matches("\n## ").count(),
        char_count: body.len(),
        baseline_git_head: git_snapshot(repo_path).head,
    };
    crate::doc::write_json(paths.agent_context_meta(project_slug), &meta)?;
    Ok(meta)
}
