use std::sync::Arc;

use mind_mesh_core::{
    agent_context_ready, agent_pack_ready, pack_agent_assets, resolve_project_repo_path,
    KnowledgePaths,
};

use crate::acp::{execution_uses_acp, resolve_acp_settings};
use crate::agent_context::run_agent_context_generation;
use crate::chat::ChatEngine;
use crate::model::ModelConfig;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AgentAssetsEnsureReport {
    pub packed: bool,
    pub context_generated: bool,
}

/// Pack repomix when missing. Returns true if a new pack was written.
pub async fn ensure_agent_pack(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path_hint: Option<&str>,
) -> anyhow::Result<(String, bool)> {
    let repo_path = resolve_project_repo_path(paths, project_slug, repo_path_hint)?;
    if agent_pack_ready(paths, project_slug) {
        return Ok((repo_path, false));
    }
    tracing::info!(
        project = project_slug,
        repo = %repo_path,
        "agent repomix pack missing — packing automatically"
    );
    pack_agent_assets(paths, project_slug, &repo_path).await?;
    Ok((repo_path, true))
}

/// Generate context.md when missing (repomix must already exist).
pub async fn generate_agent_context_if_missing(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<bool> {
    if agent_context_ready(paths, project_slug) {
        return Ok(false);
    }
    tracing::info!(
        project = project_slug,
        "agent context missing — generating automatically"
    );
    let acp = resolve_acp_settings();
    let engine = if execution_uses_acp(&acp) {
        None
    } else {
        Some(Arc::new(ChatEngine::new(paths.clone(), model_config.clone())?))
    };
    run_agent_context_generation(paths, engine, &acp, project_slug, repo_path).await?;
    Ok(true)
}

/// Ensure repomix pack and agent/context.md exist; run generation when missing.
pub async fn ensure_agent_assets(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    project_slug: &str,
    repo_path_hint: Option<&str>,
) -> anyhow::Result<AgentAssetsEnsureReport> {
    let (repo_path, packed) = ensure_agent_pack(paths, project_slug, repo_path_hint).await?;
    let context_generated =
        generate_agent_context_if_missing(paths, model_config, project_slug, &repo_path).await?;
    Ok(AgentAssetsEnsureReport {
        packed,
        context_generated,
    })
}

/// Prepare agent assets before an Ask turn (pack + context). Skipped for `agent-ctx-*` sessions.
pub async fn prepare_agent_assets_for_ask(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    project_slug: &str,
    repo_path_hint: Option<&str>,
) -> anyhow::Result<()> {
    let (repo_path, _) = ensure_agent_pack(paths, project_slug, repo_path_hint).await?;
    generate_agent_context_if_missing(paths, model_config, project_slug, &repo_path).await?;
    Ok(())
}
