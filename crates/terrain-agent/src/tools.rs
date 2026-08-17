use std::sync::Arc;

use adk_core::{ErrorComponent, Tool};
use adk_tool::FunctionTool;
use terrain_core::{
    agent_context_ready, agent_pack_ready, build_context_overview, resolve_freshness_summary,
    extract_context_section, split_context_sections, AgentPackMeta, FRESH_THRESHOLD,
    KnowledgePaths, KnowledgeSearch, SearchOptions, grep_repomix_pack, list_human_docs,
    read_agent_pack_file, read_json, resolve_project_repo_path, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::context_generator::AgentContextGenerator;
use crate::tool_session_cache::{
    context_call_fingerprint, duplicate_call_response, get_cached, pack_file_call_fingerprint,
    store_cached, truncate_with_notice,
};

const MAX_PACK_META_TREE_CHARS: usize = 2_500;

fn truncate_tool_json(value: serde_json::Value, max_chars: usize) -> serde_json::Value {
    let serialized = serde_json::to_string(&value).unwrap_or_default();
    if serialized.len() <= max_chars {
        return value;
    }
    json!({
        "truncated": true,
        "original_chars": serialized.len(),
        "preview": serialized.chars().take(max_chars).collect::<String>(),
    })
}

const MAX_TOOL_JSON_CHARS: usize = 24_000;
const DEFAULT_PACK_FILE_LINES: u32 = 150;

fn map_core_err(err: terrain_core::CoreError) -> adk_core::AdkError {
    adk_core::AdkError::tool(err.to_string())
}

pub fn list_projects_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "list_projects",
            "List indexed projects in the Terrain knowledge base.",
            move |_ctx, _args| {
                let paths = paths.clone();
                async move {
                    KnowledgeSearch::new(&paths)
                        .list_projects()
                        .map_err(map_core_err)
                        .map(|projects| json!({ "projects": projects }))
                }
            },
        )
        .with_parameters_schema::<EmptyArgs>(),
    )
}

pub fn read_agent_context_tool(
    paths: KnowledgePaths,
    generator: Arc<dyn AgentContextGenerator>,
) -> Arc<dyn Tool> {
    let lang = terrain_core::current_language();
    let s = lang.agent_context_sections();
    let description = format!(
        "Meso layer: read agent/context.md by section. Overview is preloaded in the user message — \
         pass section (e.g. \"{}\", \"{}\") only when needed. \
         For source code use grep_agent_pack → read_agent_pack_file (micro layer). \
         Do not repeat identical project+section.",
        s[3], s[5],
    );
    Arc::new(
        FunctionTool::new(
            "read_agent_context",
            description,
            move |ctx, args| {
                let paths = paths.clone();
                let generator = generator.clone();
                async move {
                    let params: ReadAgentContextArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;

                    let session_id = ctx.session_id().to_string();
                    let fingerprint =
                        context_call_fingerprint(&params.project, params.section.as_deref());
                    if let Some(cached) =
                        get_cached(&session_id, "read_agent_context", &fingerprint)
                    {
                        return Ok(duplicate_call_response(cached));
                    }

                    let section_query = params
                        .section
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty());
                    if section_query.is_none() && agent_context_ready(&paths, &params.project) {
                        return Err(adk_core::AdkError::tool(format!(
                            "Macro overview is already preloaded in the user message. \
                             Do not call read_agent_context without section. \
                             For a specific heading use section=\"{}\" (etc.). \
                             For code use grep_agent_pack → read_agent_pack_file.",
                            s[3],
                        )));
                    }

                    let was_pack_ready = agent_pack_ready(&paths, &params.project);
                    let was_context_ready = agent_context_ready(&paths, &params.project);

                    let report = generator
                        .ensure_ready(&params.project, None)
                        .await
                        .map_err(|e| adk_core::AdkError::tool(e.to_string()))?;

                    let doc = terrain_core::read_doc_at_in_project(
                        &paths,
                        "agent/context.md",
                        Some(&params.project),
                    )
                    .map_err(map_core_err)?;

                    let sections = split_context_sections(&doc.body);
                    let (body, mode, section_title) =
                        if let Some(query) = section_query {
                            let section = extract_context_section(&sections, query).ok_or_else(|| {
                                adk_core::AdkError::tool(format!(
                                    "section not found: {query}. Available: {}",
                                    sections
                                        .iter()
                                        .map(|s| s.title.as_str())
                                        .filter(|t| !t.is_empty())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ))
                            })?;
                            let (part, _truncated) = truncate_with_notice(
                                &render_context_section(&section.title, &section.body),
                                AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
                            );
                            (part, "section", Some(section.title.clone()))
                        } else {
                            let overview =
                                build_context_overview(&doc.body, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS);
                            (overview.macro_markdown, "overview", None)
                        };

                    let truncated = body.contains("...[truncated");

                    let freshness_warning = resolve_project_repo_path(&paths, &params.project, None)
                        .ok()
                        .and_then(|repo| resolve_freshness_summary(&paths, &params.project, &repo).ok())
                        .filter(|f| f.agent_context_score < FRESH_THRESHOLD)
                        .map(|f| {
                            format!(
                                "freshness_score={}/100; commits_since_baseline={}; \
                                 verify architecture claims with grep_agent_pack",
                                f.agent_context_score, f.commits_since_baseline
                            )
                        });

                    let response = json!({
                        "path": doc.path,
                        "mode": mode,
                        "section": section_title,
                        "body": body,
                        "truncated": truncated,
                        "auto_packed": !was_pack_ready && report.packed,
                        "auto_generated": !was_context_ready && report.context_generated,
                        "freshness_warning": freshness_warning,
                    });
                    store_cached(
                        &session_id,
                        "read_agent_context",
                        &fingerprint,
                        response.clone(),
                    );
                    Ok(response)
                }
            },
        )
        .with_parameters_schema::<ReadAgentContextArgs>(),
    )
}

fn render_context_section(title: &str, body: &str) -> String {
    if title.is_empty() {
        body.to_string()
    } else {
        format!("## {title}\n{body}")
    }
}

pub fn search_knowledge_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "search_knowledge",
            "Full-text search for structured docs (interfaces/, routes/, modules/) only. \
             Architecture is preloaded — do not search for overview/architecture topics. \
             Each hit includes `rel_path` for read_doc.",
            move |_ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: SearchArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;
                    KnowledgeSearch::new(&paths)
                        .search(
                            &params.query,
                            SearchOptions {
                                project: params.project,
                                doc_type: None,
                                limit: params.limit.unwrap_or(10),
                            },
                        )
                        .map_err(map_core_err)
                        .map(|hits| json!({ "hits": hits }))
                }
            },
        )
        .with_parameters_schema::<SearchArgs>(),
    )
}

pub fn read_agent_pack_meta_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "read_agent_pack_meta",
            "Fallback: repomix pack metadata when NOT preloaded in the user message. \
             Call at most once per session. For code, use grep_agent_pack next.",
            move |ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: ProjectSlugArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;

                    let session_id = ctx.session_id().to_string();
                    let fingerprint = params.project.clone();
                    if let Some(cached) =
                        get_cached(&session_id, "read_agent_pack_meta", &fingerprint)
                    {
                        return Ok(duplicate_call_response(cached));
                    }

                    let meta_path = paths.agent_pack_meta(&params.project);
                    let meta: AgentPackMeta = read_json(&meta_path).map_err(map_core_err)?;
                    let (directory_structure, tree_truncated) =
                        truncate_with_notice(&meta.directory_structure, MAX_PACK_META_TREE_CHARS);

                    let response = json!({
                        "meta": {
                            "project": meta.project,
                            "repo_path": meta.repo_path,
                            "pack_strategy": meta.pack_strategy,
                            "synced_at": meta.synced_at,
                            "total_files": meta.total_files,
                            "total_tokens": meta.total_tokens,
                            "top_files_by_tokens": meta.top_files_by_tokens,
                            "directory_structure": directory_structure,
                            "directory_structure_truncated": tree_truncated,
                        },
                        "pack_path": paths.agent_pack_main(&params.project).display().to_string(),
                    });
                    store_cached(
                        &session_id,
                        "read_agent_pack_meta",
                        &fingerprint,
                        response.clone(),
                    );
                    Ok(response)
                }
            },
        )
        .with_parameters_schema::<ProjectSlugArgs>(),
    )
}

pub fn grep_agent_pack_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "grep_agent_pack",
            "Regex search in agent/repomix.md. Each hit includes file_path and file_line (source line \
             within the pack slice). Use file_line for read_agent_pack_file — NOT line_number (repomix.md \
             position). Identical pattern+project returns cached hits.",
            move |ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: GrepPackArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;

                    let session_id = ctx.session_id().to_string();
                    let fingerprint = format!("{}:{}", params.project, params.pattern);
                    if let Some(cached) =
                        get_cached(&session_id, "grep_agent_pack", &fingerprint)
                    {
                        return Ok(duplicate_call_response(cached));
                    }

                    let pack = paths.agent_pack_main(&params.project);
                    let pack_text = terrain_core::read_pack_text_cached(&pack).map_err(|e| {
                        map_core_err(terrain_core::CoreError::InvalidDoc(format!(
                            "cannot read {}: {e}",
                            pack.display()
                        )))
                    })?;
                    let hits = grep_repomix_pack(
                        &pack_text,
                        &params.pattern,
                        params.context.unwrap_or(2),
                        params.limit.unwrap_or(20),
                    )
                    .map_err(map_core_err)?;
                    let response = truncate_tool_json(
                        json!({ "hits": hits, "pack_path": pack.display().to_string() }),
                        MAX_TOOL_JSON_CHARS,
                    );
                    store_cached(
                        &session_id,
                        "grep_agent_pack",
                        &fingerprint,
                        response.clone(),
                    );
                    Ok(response)
                }
            },
        )
        .with_parameters_schema::<GrepPackArgs>(),
    )
}

pub fn list_human_docs_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "list_human_docs",
            "List Litho human-facing documents for a project.",
            move |_ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: ProjectSlugArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;
                    list_human_docs(&paths, &params.project)
                        .map_err(map_core_err)
                        .map(|docs| json!({ "documents": docs }))
                }
            },
        )
        .with_parameters_schema::<ProjectSlugArgs>(),
    )
}

pub fn read_agent_pack_file_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "read_agent_pack_file",
            "Read a source file slice from agent/repomix.md. Use file_line from grep_agent_pack hits \
             for start_line/end_line (≤150 lines). Do NOT use grep line_number or context.md repo line \
             numbers — the pack may contain a folded/truncated slice. Omit line args to read the full \
             packed section. Identical args return cached content.",
            move |ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: ReadPackFileArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;
                    let pack = paths.agent_pack_main(&params.project);
                    let start_line = params.start_line.unwrap_or(1);
                    let end_line = params
                        .end_line
                        .unwrap_or_else(|| start_line.saturating_add(DEFAULT_PACK_FILE_LINES - 1));

                    let session_id = ctx.session_id().to_string();
                    let fingerprint = pack_file_call_fingerprint(
                        &params.project,
                        &params.file_path,
                        start_line,
                        end_line,
                    );
                    if let Some(cached) =
                        get_cached(&session_id, "read_agent_pack_file", &fingerprint)
                    {
                        return Ok(duplicate_call_response(cached));
                    }

                    let slice = read_agent_pack_file(
                        &pack,
                        &params.file_path,
                        Some(start_line),
                        Some(end_line),
                    )
                    .map_err(map_core_err)?;
                    let mut response = json!({
                        "file": slice,
                        "pack_path": pack.display().to_string(),
                    });
                    if slice.range_clamped {
                        response["hint"] = json!(format!(
                            "Requested lines {:?}-{:?} but pack section has {} lines (may be folded/truncated). \
                             Returned lines {}-{}. Use grep_agent_pack file_line values or omit line range.",
                            slice.requested_start_line,
                            slice.requested_end_line,
                            slice.total_lines,
                            slice.start_line,
                            slice.end_line,
                        ));
                    }
                    let response = truncate_tool_json(response, MAX_TOOL_JSON_CHARS);
                    store_cached(
                        &session_id,
                        "read_agent_pack_file",
                        &fingerprint,
                        response.clone(),
                    );
                    Ok(response)
                }
            },
        )
        .with_parameters_schema::<ReadPackFileArgs>(),
    )
}


pub fn read_doc_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    let overview_doc = terrain_core::current_language().litho_overview_filename();
    Arc::new(
        FunctionTool::new(
            "read_doc",
            format!(
                "Read a Litho or structured knowledge Markdown document. \
                 `path` accepts: absolute paths; knowledge-root-relative paths like \
                 `human/{overview_doc}` or `modules/core.md`; bare filenames like \
                 `{overview_doc}` or `core` (resolved under human/, modules/, etc.); \
                 or `.terrain/`-prefixed paths. Use `rel_path` from search_knowledge hits. \
                 Pass `project` when the path is ambiguous."
            ),
            move |_ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: ReadDocArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;
                    terrain_core::read_doc_at_in_project(
                        &paths,
                        &params.path,
                        params.project.as_deref(),
                    )
                    .map_err(map_core_err)
                    .map(|doc| {
                        json!({
                            "path": doc.path,
                            "frontmatter": doc.frontmatter,
                            "body": doc.body,
                        })
                    })
                }
            },
        )
        .with_parameters_schema::<ReadDocArgs>(),
    )
}

pub fn read_doc_ask_tool(paths: KnowledgePaths) -> Arc<dyn Tool> {
    Arc::new(
        FunctionTool::new(
            "read_doc",
            "Read a knowledge Markdown document. Prefer read_agent_context for architecture. \
             Use for agent/context.md, structured docs (modules/, interfaces/, routes/). \
             `path` accepts absolute paths, knowledge-root-relative paths, or bare filenames \
             (e.g. `core` → modules/core.md). Prefer `rel_path` from search_knowledge hits. \
             Do NOT read human/ Litho docs when agent/context.md exists — use read_agent_context instead.",
            move |_ctx, args| {
                let paths = paths.clone();
                async move {
                    let params: ReadDocArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::new(
                            ErrorComponent::Tool,
                            adk_core::ErrorCategory::InvalidInput,
                            "tool.invalid_args",
                            e.to_string(),
                        )
                    })?;

                    let path_lower = params.path.to_lowercase();
                    if path_lower.contains("/human/") || path_lower.starts_with("human/") {
                        let slug = params.project.as_deref();
                        if let Some(slug) = slug
                            && agent_context_ready(&paths, slug) {
                                return Err(adk_core::AdkError::tool(
                                    "human/ Litho docs are disabled in Ask mode when agent/context.md \
                                     exists. Call read_agent_context(section=\"…\") for architecture, \
                                     grep_agent_pack for code.",
                                ));
                            }
                    }
                    if (path_lower.ends_with("agent/context.md") || path_lower == "agent/context.md")
                        && let Some(slug) = params.project.as_deref()
                            && agent_context_ready(&paths, slug) {
                                return Err(adk_core::AdkError::tool(
                                    "agent/context.md overview is preloaded in Ask mode. \
                                     Use read_agent_context(section=\"…\") for one section, not read_doc.",
                                ));
                            }

                    terrain_core::read_doc_at_in_project(
                        &paths,
                        &params.path,
                        params.project.as_deref(),
                    )
                    .map_err(map_core_err)
                    .map(|doc| {
                        json!({
                            "path": doc.path,
                            "frontmatter": doc.frontmatter,
                            "body": doc.body,
                        })
                    })
                }
            },
        )
        .with_parameters_schema::<ReadDocArgs>(),
    )
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct EmptyArgs {}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct SearchArgs {
    /// Search query (interface name, route path, module, etc.)
    query: String,
    /// Optional project slug filter
    project: Option<String>,
    /// Max results (default 10)
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct ReadDocArgs {
    /// Document path: absolute, knowledge-root-relative (`human/1.概述.md`, `modules/core.md`),
    /// or bare filename (`1.概述.md`, `core`) resolved under known subdirectories.
    path: String,
    /// Project slug when `path` is relative or ambiguous (e.g. bare filename)
    project: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct ReadAgentContextArgs {
    /// Project slug
    project: String,
    /// Optional `##` section heading substring (e.g. `核心流程`, `系统边界`). Omit for macro overview.
    section: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct ProjectSlugArgs {
    /// Project slug
    project: String,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct GrepPackArgs {
    /// Project slug
    project: String,
    /// Regex pattern
    pattern: String,
    /// Context lines before/after (default 2)
    context: Option<usize>,
    /// Max hits (default 20)
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
struct ReadPackFileArgs {
    /// Project slug
    project: String,
    /// File path relative to repository root (as in repomix pack headers)
    file_path: String,
    /// 1-based start line within the packed file slice (from grep file_line, not repomix line_number)
    start_line: Option<u32>,
    /// 1-based end line within the packed file slice (inclusive)
    end_line: Option<u32>,
}
