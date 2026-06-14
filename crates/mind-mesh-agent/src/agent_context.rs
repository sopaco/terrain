use std::sync::Arc;

use mind_mesh_core::{
    agent_context_ready, agent_pack_ready, build_agent_context_prompt, pack_agent_assets,
    prepare_model_markdown, write_agent_context, AgentContextMeta, KnowledgePaths,
};

use crate::chat::ChatEngine;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentContextGenerationResult {
    pub output_path: String,
    pub meta: AgentContextMeta,
    pub response_excerpt: String,
}

/// Generate `agent/context.md` — architecture-level context for agents (not source code).
pub async fn run_agent_context_generation(
    paths: &KnowledgePaths,
    engine: Arc<ChatEngine>,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<AgentContextGenerationResult> {
    if !agent_pack_ready(paths, project_slug) {
        tracing::info!(
            project = project_slug,
            repo = %repo_path,
            "agent repomix pack missing — packing before context generation"
        );
        pack_agent_assets(paths, project_slug, repo_path).await?;
    }

    let prompt = build_agent_context_prompt(paths, project_slug, repo_path)?;
    let session_id = format!("agent-ctx-{project_slug}");

    let reply = engine
        .run_turn(
            &session_id,
            &prompt,
            Some(project_slug),
            Some(repo_path),
            |_| {},
            |_| {},
            |_| {},
            |_| {},
        )
        .await?;

    paths.write_debug_file("last-agent-context-raw.md", &reply.answer);
    let body = prepare_model_markdown(&reply.answer);
    paths.write_debug_file("last-agent-context-sanitized.md", &body);
    if body.trim().is_empty() {
        anyhow::bail!("Agent context generation produced empty output after sanitization");
    }

    let meta = write_agent_context(paths, project_slug, repo_path, &body)?;
    let output_path = paths.agent_context_main(project_slug);
    let excerpt: String = body.chars().take(300).collect();

    Ok(AgentContextGenerationResult {
        output_path: output_path.display().to_string(),
        meta,
        response_excerpt: excerpt,
    })
}

pub fn agent_context_exists(paths: &KnowledgePaths, project_slug: &str) -> bool {
    agent_context_ready(paths, project_slug)
}
