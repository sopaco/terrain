use anyhow::{Context, Result};
use terrain_core::{
    build_context_overview, extract_context_section, grep_file, read_agent_pack_file,
    read_doc_at_in_project, split_context_sections, AgentPackMeta, KnowledgePaths,
    KnowledgeSearch, SearchOptions, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
};

use crate::cli::ToolsCommands;
use crate::util::print_json;

pub fn run(paths: &KnowledgePaths, command: ToolsCommands) -> Result<()> {
    match command {
        ToolsCommands::ListProjects => {
            let projects = KnowledgeSearch::new(paths).list_projects()?;
            print_json(&projects)
        }
        ToolsCommands::PackMeta { project } => {
            let meta_path = paths.agent_pack_meta(&project);
            let meta: AgentPackMeta = terrain_core::read_json(&meta_path)
                .with_context(|| format!("read {}", meta_path.display()))?;
            print_json(&meta)
        }
        ToolsCommands::GrepPack {
            project,
            pattern,
            context,
            limit,
        } => {
            let pack = paths.agent_pack_main(&project);
            let hits = grep_file(&pack, &pattern, context, limit)?;
            print_json(&hits)
        }
        ToolsCommands::ReadPackFile {
            project,
            file,
            start_line,
            end_line,
        } => {
            let pack = paths.agent_pack_main(&project);
            let content = read_agent_pack_file(&pack, &file, start_line, end_line)?;
            print_json(&content)
        }
        ToolsCommands::ReadContext { project, section } => {
            let doc = read_doc_at_in_project(paths, "agent/context.md", Some(&project))?;
            let sections = split_context_sections(&doc.body);
            let (body, mode, title) =
                if let Some(ref query) = section.filter(|s| !s.trim().is_empty()) {
                    let sec = extract_context_section(&sections, query)
                        .with_context(|| format!("section not found: {query}"))?;
                    (sec.body.clone(), "section", sec.title.clone())
                } else {
                    let overview =
                        build_context_overview(&doc.body, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS);
                    (overview.macro_markdown, "overview", "macro".into())
                };
            print_json(&serde_json::json!({
                "project": project,
                "mode": mode,
                "section": title,
                "body": body,
            }))
        }
        ToolsCommands::Search {
            query,
            project,
            limit,
        } => {
            let hits = KnowledgeSearch::new(paths).search(
                &query,
                SearchOptions {
                    project,
                    doc_type: None,
                    limit,
                },
            )?;
            print_json(&hits)
        }
        ToolsCommands::ReadDoc { project, path } => {
            let doc = read_doc_at_in_project(paths, &path, Some(&project))?;
            print_json(&doc)
        }
    }
}
