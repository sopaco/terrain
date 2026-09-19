//! Native Litho execution — runs the Litho documentation pipeline through the configured
//! LLM instead of an external ACP agent.
//!
//! Used when the execution mode is hybrid (`acp_native`) but the configured ACP command
//! cannot run (e.g. `ollama acp`, an invalid command): the Litho workload needs an agent
//! that reads the skill and writes files, so this module provides the same loop natively
//! with path-restricted read/write tools.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use adk_core::{Content, RunConfig, Tool};
use adk_runner::Runner;
use adk_session::{CreateRequest, InMemorySessionService, SessionService};
use adk_tool::FunctionTool;
use anyhow::{Context, Result};
use futures::StreamExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use walkdir::WalkDir;

use crate::model::{build_llm, ensure_llm, ModelConfig};
use crate::tools::{grep_agent_pack_tool, read_agent_pack_file_tool, read_agent_pack_meta_tool};
use terrain_core::{agent_pack_ready, pack_agent_assets, KnowledgePaths, LithoPlan};

const APP_NAME: &str = "terrain-litho";
const USER_ID: &str = "terrain-litho";
const MAX_READ_CHARS: usize = 40_000;

/// Everything one native Litho turn needs (clonable so it can move into the spawn).
#[derive(Clone)]
pub struct NativeLithoCtx {
    pub paths: KnowledgePaths,
    pub model_config: ModelConfig,
    pub plan: LithoPlan,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct LithoReadArgs {
    /// Markdown path relative to (or absolute under) the skill directory or the workspace.
    /// A directory path lists its markdown files instead of reading one.
    path: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct LithoWriteArgs {
    /// Markdown path relative to (or absolute under) the workspace or the output directory.
    path: String,
    /// Full file content to write (the file is replaced entirely).
    content: String,
}

/// Lexically normalize `..`/`.` without touching the filesystem, so a path can be checked
/// against the allowed roots even when the target file does not exist yet.
fn normalize_within(base: &Path, raw: &str) -> Option<PathBuf> {
    let raw_path = Path::new(raw);
    let joined = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        base.join(raw_path)
    };
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    Some(out)
}

fn within_any(roots: &[PathBuf], path: &Path) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

/// Read a markdown file from the skill directory or the Litho workspace, or list a
/// directory's markdown files. The agent's only access to skill templates and research
/// artifacts.
pub fn read_litho_file_tool(skill_dir: PathBuf, workspace: PathBuf) -> Arc<dyn Tool> {
    let roots = vec![skill_dir.clone(), workspace.clone()];
    Arc::new(
        FunctionTool::new(
            "read_litho_file",
            "Read a markdown file from the Litho skill directory or the intermediate \
             workspace (pass a directory path to list its markdown files instead).",
            move |_ctx, args| {
                let skill_dir = skill_dir.clone();
                let workspace = workspace.clone();
                let roots = roots.clone();
                async move {
                    let args: LithoReadArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::tool(format!("read_litho_file args: {e}"))
                    })?;
                    let path = normalize_within(&skill_dir, &args.path)
                        .or_else(|| normalize_within(&workspace, &args.path))
                        .ok_or_else(|| adk_core::AdkError::tool("invalid path"))?;
                    if !within_any(&roots, &path) {
                        return Err(adk_core::AdkError::tool(format!(
                            "path not under the skill directory or workspace: {}",
                            path.display()
                        )));
                    }
                    if path.is_dir() {
                        let mut names: Vec<String> = WalkDir::new(&path)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                            .filter_map(|e| {
                                e.path().strip_prefix(&path).ok().map(|p| {
                                    p.to_string_lossy().replace('\\', "/")
                                })
                            })
                            .collect();
                        names.sort();
                        return Ok(json!({ "files": names }));
                    }
                    let content = std::fs::read_to_string(&path).map_err(|e| {
                        adk_core::AdkError::tool(format!("{}: {e}", path.display()))
                    })?;
                    if content.len() > MAX_READ_CHARS {
                        let truncated: String = content.chars().take(MAX_READ_CHARS).collect();
                        return Ok(json!({
                            "path": path.display().to_string(),
                            "content": truncated,
                            "truncated": true,
                        }));
                    }
                    Ok(json!({ "path": path.display().to_string(), "content": content }))
                }
            },
        )
        .with_parameters_schema::<LithoReadArgs>(),
    )
}

/// Write a markdown file, but only inside the research workspace or the final docs
/// output directory — the native counterpart of what an external ACP agent may touch.
pub fn write_litho_file_tool(workspace: PathBuf, human_out: PathBuf) -> Arc<dyn Tool> {
    let roots = vec![workspace, human_out];
    Arc::new(
        FunctionTool::new(
            "write_litho_file",
            "Create or overwrite a markdown file. Allowed only inside the intermediate \
             workspace (research artifacts) or the final output directory (human docs).",
            move |_ctx, args| {
                let roots = roots.clone();
                async move {
                    let args: LithoWriteArgs = serde_json::from_value(args).map_err(|e| {
                        adk_core::AdkError::tool(format!("write_litho_file args: {e}"))
                    })?;
                    if !args.path.ends_with(".md") && !args.path.ends_with(".markdown") {
                        return Err(adk_core::AdkError::tool(
                            "only markdown files (.md) may be written",
                        ));
                    }
                    let path = roots
                        .iter()
                        .find_map(|root| normalize_within(root, &args.path))
                        .ok_or_else(|| adk_core::AdkError::tool("invalid path"))?;
                    if !within_any(&roots, &path) {
                        return Err(adk_core::AdkError::tool(format!(
                            "path not allowed (must be inside the workspace or the output \
                             directory): {}",
                            path.display()
                        )));
                    }
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            adk_core::AdkError::tool(format!("{}: {e}", parent.display()))
                        })?;
                    }
                    let bytes = content_len(&args.content);
                    std::fs::write(&path, &args.content).map_err(|e| {
                        adk_core::AdkError::tool(format!("{}: {e}", path.display()))
                    })?;
                    Ok(json!({
                        "written": true,
                        "path": path.display().to_string(),
                        "chars": bytes,
                    }))
                }
            },
        )
        .with_parameters_schema::<LithoWriteArgs>(),
    )
}

fn content_len(content: &str) -> usize {
    content.chars().count()
}

fn litho_instruction(plan: &LithoPlan) -> String {
    format!(
        "You are the Terrain Litho documentation agent.\
         \nProject slug (pass as `project` to the pack tools): {slug}\
         \nSkill directory: {skill}\
         \nIntermediate research workspace: {workspace}\
         \nFinal human docs output directory: {out}\n\
         Available workflow tools:\
         \n- read_litho_file: read SKILL.md and the skill references, or list workspace files.\
         \n- write_litho_file: persist markdown files (workspace for research artifacts, \
         output directory for final docs).\
         \n- read_agent_pack_meta / grep_agent_pack / read_agent_pack_file: indexed source \
         code access; always pass project=\"{slug}\".\
         \nNever read the live repository filesystem — use the pack tools.\
         \nPersist every research artifact under the workspace (module reports under \
         workspace/modules/) and every final document under the output directory using the \
         exact file names requested in the task. Do not print full document bodies in your \
         reply; write them with write_litho_file and reply with a short progress summary. \
         Do not finish until the full requested doc set exists on disk.",
        slug = plan.project_slug,
        skill = plan.skill_dir,
        workspace = plan.litho_workspace_dir,
        out = plan.human_output_dir,
    )
}

/// Run one Litho turn (generation / composition / update prompt) through a native LLM
/// agent and return its final reply text. The documents themselves are written to disk
/// by the agent via `write_litho_file`.
pub async fn run_native_litho_turn(ctx: &NativeLithoCtx, prompt: String) -> Result<String> {
    ensure_llm(&ctx.model_config)?;
    if !agent_pack_ready(&ctx.paths, &ctx.plan.project_slug) {
        pack_agent_assets(&ctx.paths, &ctx.plan.project_slug, &ctx.plan.repo_path).await?;
    }

    let llm = build_llm(&ctx.model_config)?;
    let skill_dir = PathBuf::from(&ctx.plan.skill_dir);
    let workspace = PathBuf::from(&ctx.plan.litho_workspace_dir);
    let human_out = PathBuf::from(&ctx.plan.human_output_dir);

    let instruction = litho_instruction(&ctx.plan);
    let builder = adk_agent::LlmAgentBuilder::new(APP_NAME)
        .instruction(instruction.as_str())
        .model(llm)
        .max_iterations(80)
        .max_output_tokens(64_000)
        .tool(read_agent_pack_meta_tool(ctx.paths.clone()))
        .tool(grep_agent_pack_tool(ctx.paths.clone()))
        .tool(read_agent_pack_file_tool(ctx.paths.clone()))
        .tool(read_litho_file_tool(skill_dir, workspace))
        .tool(write_litho_file_tool(
            PathBuf::from(&ctx.plan.litho_workspace_dir),
            human_out,
        ));

    let agent = builder
        .build()
        .context("failed to build native Litho agent")?;
    let agent: Arc<dyn adk_core::Agent> = Arc::new(agent);
    let session_service = Arc::new(InMemorySessionService::new());
    let runner = Runner::builder()
        .app_name(APP_NAME)
        .agent(agent)
        .session_service(session_service.clone())
        .run_config(RunConfig::builder().build())
        .build()
        .context("failed to build native Litho runner")?;

    let session_id = format!("litho-{}-{}", ctx.plan.project_slug, uuid::Uuid::new_v4());
    session_service
        .create(CreateRequest {
            app_name: APP_NAME.into(),
            user_id: USER_ID.into(),
            session_id: Some(session_id.clone()),
            state: HashMap::new(),
        })
        .await
        .context("failed to create native Litho session")?;

    let content = Content::new("user").with_text(prompt);
    let mut stream = runner
        .run_str(USER_ID, &session_id, content)
        .await
        .context("native Litho run failed")?;

    let mut last_text = String::new();
    while let Some(event) = stream.next().await {
        let event = event.map_err(|e| anyhow::anyhow!("native Litho agent failed: {e}"))?;
        if let Some(content) = &event.llm_response.content {
            for part in &content.parts {
                if let Some(text) = part.text() {
                    last_text.push_str(text);
                }
            }
        }
    }
    Ok(last_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_within_resolves_parents() {
        let base = PathBuf::from("/tmp/ws");
        assert_eq!(
            normalize_within(&base, "modules/../c1.md").unwrap(),
            PathBuf::from("/tmp/ws/c1.md")
        );
        assert_eq!(
            normalize_within(&base, "/tmp/ws/out/x.md").unwrap(),
            PathBuf::from("/tmp/ws/out/x.md")
        );
        assert_eq!(
            normalize_within(&base, "../escape.md").unwrap(),
            PathBuf::from("/tmp/escape.md")
        );
    }

    #[test]
    fn within_any_rejects_outside_roots() {
        let roots = vec![PathBuf::from("/tmp/ws"), PathBuf::from("/tmp/out")];
        assert!(within_any(&roots, Path::new("/tmp/ws/a.md")));
        assert!(within_any(&roots, Path::new("/tmp/out/a.md")));
        assert!(!within_any(&roots, Path::new("/tmp/escape.md")));
        assert!(!within_any(&roots, Path::new("/tmp/ws2/a.md")));
    }
}