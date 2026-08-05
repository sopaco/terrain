use std::sync::Arc;

use terrain_core::{
    agent_context_synced_with_head, agent_pack_synced_with_head, maybe_pack_agent_assets,
    resolve_project_repo_path, KnowledgePaths,
};

use crate::acp::{execution_uses_native_llm, resolve_acp_settings};
use crate::settings::resolve_knowledge_settings;
use crate::agent_context::run_agent_context_generation;
use crate::chat::{ChatEngine, ChatPhase};
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
    if agent_pack_synced_with_head(paths, project_slug, &repo_path) {
        return Ok((repo_path, false));
    }
    tracing::info!(
        project = project_slug,
        repo = %repo_path,
        "agent repomix pack missing or stale — packing automatically"
    );
    maybe_pack_agent_assets(paths, project_slug, &repo_path).await?;
    Ok((repo_path, true))
}

/// Generate context.md when missing or baseline drifted (repomix must already exist).
pub async fn generate_agent_context_if_missing(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<bool> {
    if agent_context_synced_with_head(paths, project_slug, repo_path) {
        return Ok(false);
    }
    tracing::info!(
        project = project_slug,
        "agent context missing or stale — generating automatically"
    );
    let acp = resolve_acp_settings();
    let engine = if execution_uses_native_llm(&acp) {
        Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?))
    } else {
        None
    };
    run_agent_context_generation(
        paths,
        engine,
        &acp,
        project_slug,
        repo_path,
        &resolve_knowledge_settings(),
        false,
    )
    .await?;
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
    mut on_phase: impl FnMut(ChatPhase),
) -> anyhow::Result<()> {
    let repo_path = resolve_project_repo_path(paths, project_slug, repo_path_hint)?;

    if !agent_pack_synced_with_head(paths, project_slug, &repo_path) {
        on_phase(ChatPhase::PreparingPack);
        ensure_agent_pack(paths, project_slug, repo_path_hint).await?;
    }

    if !agent_context_synced_with_head(paths, project_slug, &repo_path) {
        on_phase(ChatPhase::PreparingContext);
        generate_agent_context_if_missing(paths, model_config, project_slug, &repo_path).await?;
    }

    Ok(())
}
