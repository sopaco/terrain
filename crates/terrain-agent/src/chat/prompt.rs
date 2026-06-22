use anyhow::Result;
use terrain_core::{
    agent_pack_ready, build_context_overview, compute_freshness, read_agent_context_status,
    read_freshness_ledger, read_json, resolve_project_repo_path, AgentPackMeta, KnowledgePaths,
    AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS, MACRO_PRELOAD_THRESHOLD,
};

use crate::acp::default_ask_acp_skill_dir;
use crate::tool_session_cache::truncate_with_notice;

pub(crate) const ASK_INJECT_DIR_TREE_MAX_CHARS: usize = 2_000;

pub(crate) fn build_ask_acp_prompt(
    query: &str,
    project: Option<&str>,
    paths: &KnowledgePaths,
) -> Result<String> {
    let skill_dir = default_ask_acp_skill_dir();
    let skill_dir_s = skill_dir.display().to_string();
    let knowledge_root = paths
        .knowledge_root_for(project)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let base = build_ask_prompt(query, project, paths)?;

    Ok(format!(
        "You are Terrain Ask running in **ACP mode**. Native function tools are NOT available.\n\
         Read the skill at `{skill_dir_s}/SKILL.md` and use the **`terrain tools`** CLI \
         (subprocess) to query knowledge — same capabilities as native tools.\n\n\
         Environment:\n\
         - TERRAIN_KNOWLEDGE_ROOT={knowledge_root}\n\
         - TERRAIN_ASK_SKILL={skill_dir_s}\n\
         - Project slug: {}\n\n\
         Workflow: macro context below → `terrain tools read-context` / `grep-pack` / \
         `read-pack-file` as needed → answer with citations (paths:line).\n\n\
         {base}",
        project.unwrap_or("(none — pass --project to tools commands)"),
    ))
}

pub(crate) fn build_ask_prompt(
    query: &str,
    project: Option<&str>,
    paths: &KnowledgePaths,
) -> Result<String> {
    let Some(slug) = project else {
        return Ok(query.to_string());
    };

    let repo_path = resolve_project_repo_path(paths, slug, None).unwrap_or_default();
    let pack_meta = read_json::<AgentPackMeta>(paths.agent_pack_meta(slug)).ok();

    let freshness = if !repo_path.is_empty() {
        compute_freshness(paths, slug, &repo_path).ok().or_else(|| {
            read_freshness_ledger(paths, slug).map(|l| l.summary)
        })
    } else {
        None
    };

    let macro_preload = freshness
        .as_ref()
        .map(|f| f.macro_preload_allowed)
        .unwrap_or(true);

    let macro_rule = if macro_preload {
        "• Macro: architecture overview preloaded below (do NOT call read_agent_context without section)"
    } else {
        "• Macro: architecture context is STALE — do NOT trust any preloaded overview; use grep_agent_pack to verify"
    };

    let mut sections = vec![format!(
        "Current project slug: {slug}\n\
Repository path (citations / UI only): {repo_path}\n\n\
TOOL RULES — three layers:\n\
{macro_rule}\n\
• Meso: read_agent_context(section=\"…\") for a specific heading when needed\n\
• Micro: grep_agent_pack → read_agent_pack_file for source code\n\
Do NOT call read_agent_pack_meta when pack metadata is preloaded."
    )];

    if let Some(ref fresh) = freshness {
        sections.push(terrain_core::format_freshness_trust_block(fresh));
    }

    if agent_pack_ready(paths, slug) {
        if let Some(meta) = pack_meta.as_ref() {
            let top_files = meta
                .top_files_by_tokens
                .iter()
                .take(8)
                .map(|f| format!("  - {} ({} tokens)", f.path, f.tokens))
                .collect::<Vec<_>>()
                .join("\n");
            let (dir_preview, dir_truncated) =
                truncate_with_notice(&meta.directory_structure, ASK_INJECT_DIR_TREE_MAX_CHARS);
            let baseline = meta
                .baseline_git_head
                .as_deref()
                .unwrap_or("(not recorded)");
            sections.push(format!(
                "## Preloaded repomix pack metadata (do NOT call read_agent_pack_meta)\n\
synced_at: {}\nbaseline_git_head: {}\nfiles: {}\ntokens: {}\nstrategy: {}\n\n\
Top files by tokens:\n{}\n\n\
Directory tree (preview{}):\n{}\n",
                meta.synced_at,
                baseline,
                meta.total_files,
                meta.total_tokens,
                meta.pack_strategy,
                if top_files.is_empty() {
                    "  (none)".into()
                } else {
                    top_files
                },
                if dir_truncated { ", truncated" } else { "" },
                dir_preview,
            ));
        }
    } else {
        sections.push(
            "## Repomix pack: NOT READY\n\
Assets will be auto-generated; you may call read_agent_context once if needed.\n"
                .to_string(),
        );
    }

    let ctx = read_agent_context_status(paths, slug);
    if ctx.ready && macro_preload {
        let context_path = paths.agent_context_main(slug);
        let body = terrain_core::read_doc(&context_path)
            .map(|d| d.body)
            .unwrap_or_default();
        let overview = build_context_overview(&body, AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS);
        sections.push(format!(
            "## Preloaded architecture context (macro layer)\n\
Total stored: {} chars in {} sections{}. Do NOT re-fetch overview.\n\n\
{}\n",
            overview.total_chars,
            overview.section_titles.len(),
            if overview.size_capped {
                " (overview capped)"
            } else {
                ""
            },
            overview.macro_markdown,
        ));
    } else if ctx.ready {
        sections.push(format!(
            "## Architecture context: STALE (score < {MACRO_PRELOAD_THRESHOLD})\n\
Macro overview withheld due to low freshness. Use read_agent_context(section=\"…\") \
only after grep_agent_pack verification, or tell the user to run 快速保鲜 in Terrain.\n"
        ));
    } else {
        sections.push(
            "## Architecture context: NOT READY\n\
Call read_agent_context (no section) once to load or auto-generate context.md.\n"
                .to_string(),
        );
    }

    sections.push(format!(
        "## Source code (micro layer)\n\
grep_agent_pack → read_agent_pack_file (≤150 lines; use grep file_line, not line_number). \
Do not read the live repository.\n\n\
## User question\n{query}"
    ));

    Ok(sections.join("\n"))
}
