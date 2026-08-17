use std::path::Path;

use chrono::Utc;
use crate::freshness::{baseline_matches_head, git_snapshot};

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

/// Recorded baseline commit for `agent/context.md` (falls back to the repomix pack baseline).
pub fn agent_context_baseline_head(
    paths: &KnowledgePaths,
    project_slug: &str,
) -> Option<String> {
    context_baseline_head(paths, project_slug)
}

/// Body of the existing `agent/context.md`, without frontmatter.
pub fn read_agent_context_body(paths: &KnowledgePaths, project_slug: &str) -> Option<String> {
    read_doc(paths.agent_context_main(project_slug))
        .ok()
        .map(|doc| doc.body)
        .filter(|body| !body.trim().is_empty())
}

fn context_baseline_head(
    paths: &KnowledgePaths,
    project_slug: &str,
) -> Option<String> {
    let ctx_meta =
        crate::doc::read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug)).ok();
    let pack_meta =
        crate::doc::read_json::<crate::schema::AgentPackMeta>(paths.agent_pack_meta(project_slug))
            .ok();
    ctx_meta
        .as_ref()
        .and_then(|m| m.baseline_git_head.clone())
        .or_else(|| pack_meta.and_then(|m| m.baseline_git_head.clone()))
}

/// True when context exists and its baseline matches current HEAD (ignores dirty working tree).
pub fn agent_context_synced_with_head(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> bool {
    if !agent_context_ready(paths, project_slug) {
        return false;
    }
    baseline_matches_head(repo_path, context_baseline_head(paths, project_slug).as_deref())
}

/// True when context matches current HEAD and the working tree is clean (excluding `.terrain/`).
pub fn agent_context_fresh(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> bool {
    if !agent_context_synced_with_head(paths, project_slug, repo_path) {
        return false;
    }
    let git = git_snapshot(repo_path);
    !git.is_git_repo || !git.dirty
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
    let lang = crate::language::current_language();
    let sections = lang.agent_context_sections();

    let index = read_doc(paths.project_index(project_slug))?;
    let meta_path = paths.agent_pack_meta(project_slug);
    let pack_meta = crate::doc::read_json::<crate::schema::AgentPackMeta>(&meta_path).ok();

    // Prefer the overview doc in the current language, fall back to the Chinese
    // name used by older assets, then any `1.*.md`.
    let human_dir = paths.human_docs_dir(project_slug);
    let human_hint = [
        lang.litho_overview_filename().to_string(),
        crate::language::ResolvedLanguage::ZhCn
            .litho_overview_filename()
            .to_string(),
    ]
    .iter()
    .find_map(|name| read_doc(human_dir.join(name)).ok())
    .or_else(|| {
        let mut names: Vec<String> = std::fs::read_dir(&human_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("1.") && n.ends_with(".md"))
            .collect();
        names.sort();
        names
            .into_iter()
            .find_map(|n| read_doc(human_dir.join(n)).ok())
    })
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
                 **Use it as authoritative structured input** when writing {s2}, {s5}, and {s6}. \
                 Supplement with grep_agent_pack discovery; do not invent modules that contradict this bundle.\n\n\
                 {body}\n\n",
                s2 = sections[2],
                s5 = sections[5],
                s6 = sections[6],
            )
        }
    };

    Ok(format!(
        "Generate an Agent-facing architecture context document for project \"{project_slug}\".\n\n\
         {lang_directive}\n\n\
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
         - {s6}: file paths only — details live in agent/repomix.md\n\
         - **Hard limit: ≤14000 characters total** — tables & bullets over prose; trimmed to 16 KiB on save\n\
         - Keep {s0} + {s1} + {s2} dense; other sections concise\n\
         - Use grep_agent_pack only to discover paths, not to paste code\n\n\
         {meta_section}\
         ## Project index\n{index_body}\n\n\
         ## Directory structure (from agent pack)\n{directory_structure}\n\n\
         {human_section}\n\
         Write the complete markdown to the output path. Required sections:\n\
         1. ## {s0}\n\
         2. ## {s1}\n\
         3. ## {s2}\n\
         4. ## {s3}\n\
         5. ## {s4}\n\
         6. ## {s5}\n\
         7. ## {s6}\n\n\
         Return ONLY the final markdown document in your reply. \
         Do not include reasoning, thinking, or commentary outside the document.\n",
        project_slug = project_slug,
        lang_directive = lang.asset_language_directive(),
        skill_dir_display = skill_dir_display,
        skill_excerpt = skill_excerpt,
        output_path = output_path,
        repo_path = repo_path,
        index_body = index.body,
        directory_structure = directory_structure,
        meta_section = meta_section,
        s0 = sections[0],
        s1 = sections[1],
        s2 = sections[2],
        s3 = sections[3],
        s4 = sections[4],
        s5 = sections[5],
        s6 = sections[6],
        human_section = if human_hint.is_empty() {
            String::new()
        } else {
            format!(
                "## Human doc excerpt ({overview})\n{human_hint}\n\n",
                overview = lang.litho_overview_filename()
            )
        },
    ))
}

/// Prompt for an **incremental** refresh of `agent/context.md` from a Git change set.
///
/// Unlike [`build_agent_context_prompt`] this carries the existing document verbatim and asks
/// for surgical edits, so an unrelated commit costs a short turn instead of a full rewrite.
/// The reply is still the complete document — persistence is whole-file (see
/// [`write_agent_context`]) — but the model is instructed to copy untouched sections as-is.
pub fn build_agent_context_update_prompt(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    plan: &crate::assets::IncrementalPlan,
) -> Result<String> {
    let existing = read_agent_context_body(paths, project_slug).ok_or_else(|| {
        crate::error::CoreError::InvalidDoc(
            "agent context is missing or empty — cannot update incrementally".into(),
        )
    })?;

    let output_path = paths.agent_context_main(project_slug).display().to_string();
    let meta_path = paths.agent_pack_meta(project_slug);
    let pack_meta = crate::doc::read_json::<crate::schema::AgentPackMeta>(&meta_path).ok();
    let directory_structure = pack_meta
        .as_ref()
        .map(|m| m.directory_structure.as_str())
        .unwrap_or("(agent pack not generated)");

    // Structured meta is cheap and authoritative; refresh it so renamed modules are visible.
    let lang = crate::language::current_language();
    let sections = lang.agent_context_sections();
    let repo = Path::new(repo_path);
    let meta_bundle = collect_project_meta(repo)?;
    persist_meta_inputs(paths, project_slug, &meta_bundle)?;
    let meta_body = format_meta_bundle_for_prompt(&meta_bundle);
    let meta_section = if meta_body.is_empty() {
        String::new()
    } else {
        format!(
            "## Developer meta (`terrain-meta.json`, current)\n\
             Authoritative structured input for {s2} / {s5} / {s6}.\n\n\
             {meta_body}\n\n",
            s2 = sections[2],
            s5 = sections[5],
            s6 = sections[6],
        )
    };

    Ok(format!(
        "Update the existing Agent-facing architecture context document for project \
         \"{project_slug}\".\n\n\
         {lang_directive}\n\n\
         Output file (write with absolute path): {output_path}\n\
         Repository: {repo_path}\n\n\
         {rules}\
         {evidence}\
         {meta_section}\
         ## Current directory structure (from agent pack)\n{directory_structure}\n\n\
         ## Existing document (baseline — edit this, do not rewrite it)\n\
         ```markdown\n{existing}\n```\n\n\
         ## Output contract\n\
         - Return the **complete updated markdown document** — Terrain persists your reply as the \
           whole file, so omitting an untouched section deletes it.\n\
         - Keep the same seven `##` sections in the same order: {s0}, {s1}, {s2}, \
           {s3}, {s4}, {s5}, {s6}.\n\
         - Keep tables and bullets over prose; hard limit ≤14000 characters.\n\
         - Use grep_agent_pack only on the changed paths above; never paste code.\n\
         - Return ONLY the markdown document. No reasoning, no commentary, no diff markers.\n",
        project_slug = project_slug,
        lang_directive = lang.asset_language_directive(),
        output_path = output_path,
        repo_path = repo_path,
        rules = plan.update_rules_block(crate::assets::IncrementalOutputMode::WholeDocumentReply),
        evidence = plan.evidence_block(),
        meta_section = meta_section,
        s0 = sections[0],
        s1 = sections[1],
        s2 = sections[2],
        s3 = sections[3],
        s4 = sections[4],
        s5 = sections[5],
        s6 = sections[6],
        directory_structure = directory_structure,
        existing = existing,
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
        language: Some(crate::language::current_language().code().to_string()),
    };
    crate::doc::write_json(paths.agent_context_meta(project_slug), &meta)?;
    Ok(meta)
}

/// Re-stamp `context-meta.json` at the current HEAD without touching `context.md`.
///
/// Used when a commit advanced HEAD but changed nothing the document describes (typically a
/// commit of regenerated `.terrain/` assets). Without this, such a commit leaves the context
/// permanently "behind HEAD" and every refresh pays for a full regeneration that changes nothing.
pub fn refresh_agent_context_baseline(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> Result<AgentContextMeta> {
    let body = read_agent_context_body(paths, project_slug).ok_or_else(|| {
        crate::error::CoreError::InvalidDoc("agent context is missing — nothing to re-stamp".into())
    })?;
    let previous =
        crate::doc::read_json::<AgentContextMeta>(paths.agent_context_meta(project_slug)).ok();

    let meta = AgentContextMeta {
        project: project_slug.to_string(),
        repo_path: stored_repo_path(Path::new(repo_path)),
        output_file: "context.md".into(),
        // Keep the original generation time — the document itself was not regenerated.
        generated_at: previous
            .as_ref()
            .map(|m| m.generated_at.clone())
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        section_count: body.matches("\n## ").count(),
        char_count: body.len(),
        baseline_git_head: git_snapshot(repo_path).head,
        // Re-stamping does not change the document, so keep its language.
        language: previous.and_then(|m| m.language),
    };
    crate::doc::write_json(paths.agent_context_meta(project_slug), &meta)?;
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn write_ready_context(paths: &KnowledgePaths, slug: &str, repo_path: &str, head: &str) {
        let sections = (1..=4)
            .map(|i| format!("## Section {i}\n\n{}", "x".repeat(130)))
            .collect::<Vec<_>>()
            .join("\n\n");
        let body = format!("Overview paragraph.\n\n{sections}");
        let fm = DocFrontmatter {
            doc_type: DocType::AgentContext,
            project: slug.to_string(),
            module: None,
            title: Some("Agent Architecture Context".into()),
            source: Some(stored_repo_path(Path::new(repo_path))),
            refs: vec![],
            deps: vec![],
            extra: serde_json::Map::new(),
        };
        write_doc(&paths.agent_context_main(slug), &fm, &body).unwrap();
        let meta = AgentContextMeta {
            project: slug.to_string(),
            repo_path: stored_repo_path(Path::new(repo_path)),
            output_file: "context.md".into(),
            generated_at: Utc::now().to_rfc3339(),
            section_count: 4,
            char_count: body.len(),
            baseline_git_head: Some(head.to_string()),
            language: None,
        };
        crate::doc::write_json(paths.agent_context_meta(slug), &meta).unwrap();
    }

    #[test]
    fn context_synced_with_head_ignores_dirty_working_tree() {
        let _lock = crate::registry::registry_test_lock();
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let head = init_git_repo(repo);
        let slug = "dirty-ctx";
        crate::registry::register_project(slug, &repo.display().to_string()).unwrap();
        let paths = KnowledgePaths::new();
        paths.ensure_project_layout(slug).unwrap();
        write_ready_context(&paths, slug, &repo.display().to_string(), &head);

        fs::write(repo.join("dirty.rs"), "x").unwrap();

        assert!(agent_context_synced_with_head(
            &paths,
            slug,
            &repo.display().to_string()
        ));
        assert!(!agent_context_fresh(
            &paths,
            slug,
            &repo.display().to_string()
        ));
    }
}
