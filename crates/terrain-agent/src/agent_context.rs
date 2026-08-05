use std::collections::HashMap;
use std::sync::Arc;

use terrain_core::{
    agent_context_baseline_head, agent_context_ready, agent_pack_ready,
    build_agent_context_prompt, build_agent_context_update_prompt, pack_agent_assets,
    plan_incremental_update, prepare_model_markdown, refresh_agent_context_baseline,
    write_agent_context, AgentContextGenerationResult, IncrementalOptions, IncrementalPlan,
    KnowledgePaths, KnowledgeRefreshMode, KnowledgeSettings, KnowledgeUpdateMode,
};

use crate::acp::{
    build_acp_config, default_agent_arch_acp_skill_dir, default_ask_acp_skill_dir,
    execution_pure_acp,
};
use crate::chat::ChatEngine;
use crate::settings::AcpSettings;

/// Generate `agent/context.md` — architecture-level context for agents (not source code).
///
/// With `force_full` unset and incremental refresh enabled, an existing document that has only
/// drifted a few commits is *updated* from `git diff` rather than rebuilt, which is the
/// difference between a short turn and a full architecture pass. `force_full` is what the
/// UI's 「重新生成」 sets — an explicit rebuild request always rebuilds.
pub async fn run_agent_context_generation(
    paths: &KnowledgePaths,
    engine: Option<Arc<ChatEngine>>,
    acp_settings: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
    knowledge: &KnowledgeSettings,
    force_full: bool,
) -> anyhow::Result<AgentContextGenerationResult> {
    let mode = if force_full {
        KnowledgeUpdateMode::Full {
            reason: "explicit_rebuild",
        }
    } else {
        plan_incremental_update(
            repo_path,
            agent_context_baseline_head(paths, project_slug).as_deref(),
            agent_context_ready(paths, project_slug),
            IncrementalOptions::from(knowledge),
        )
    };

    let update_plan = match mode {
        KnowledgeUpdateMode::Incremental(plan) => {
            tracing::info!(
                project = project_slug,
                baseline = plan.short_baseline(),
                touched = plan.touched_file_count(),
                "agent context: incremental update from git diff"
            );
            Some(plan)
        }
        // Only `.terrain/` output moved since the baseline, so the document is still accurate.
        // Re-stamp the baseline instead of paying for a regeneration that changes nothing.
        KnowledgeUpdateMode::UpToDate => {
            tracing::info!(
                project = project_slug,
                "agent context: no source drift since baseline — re-stamping baseline only"
            );
            let meta = refresh_agent_context_baseline(paths, project_slug, repo_path)?;
            return Ok(AgentContextGenerationResult {
                output_path: paths.agent_context_main(project_slug).display().to_string(),
                meta,
                response_excerpt: String::new(),
                refresh_mode: KnowledgeRefreshMode::Skipped,
            });
        }
        KnowledgeUpdateMode::Full { reason } => {
            tracing::info!(
                project = project_slug,
                reason,
                "agent context: full regeneration"
            );
            None
        }
    };
    let refresh_mode = if update_plan.is_some() {
        KnowledgeRefreshMode::Incremental
    } else {
        KnowledgeRefreshMode::Full
    };

    if !agent_pack_ready(paths, project_slug) {
        tracing::info!(
            project = project_slug,
            repo = %repo_path,
            "agent repomix pack missing — packing before context generation"
        );
        pack_agent_assets(paths, project_slug, repo_path).await?;
    }

    let raw_answer = run_agent_context_turn(
        paths,
        engine.as_deref(),
        acp_settings,
        project_slug,
        repo_path,
        update_plan.as_deref(),
    )
    .await?;

    paths.write_debug_file("last-agent-context-raw.md", &raw_answer);
    let mut body = prepare_model_markdown(&raw_answer);
    paths.write_debug_file("last-agent-context-sanitized.md", &body);

    let mut refresh_mode = refresh_mode;
    // An incremental turn asks the model to reproduce most of the document verbatim — a
    // truncated context, a dropped final message, or a model that only used its edit tool
    // without echoing the result back can all surface as an empty reply. That is not evidence
    // the *document* is unrecoverable, only that the diff-driven turn was: retry once as a
    // full regeneration rather than leaving the document stuck behind HEAD until someone
    // notices and clicks 「重新生成」 by hand.
    if body.trim().is_empty() && update_plan.is_some() {
        tracing::warn!(
            project = project_slug,
            "agent context: incremental update returned empty output — falling back to full regeneration"
        );
        let raw_answer = run_agent_context_turn(
            paths,
            engine.as_deref(),
            acp_settings,
            project_slug,
            repo_path,
            None,
        )
        .await?;
        paths.write_debug_file("last-agent-context-raw.md", &raw_answer);
        body = prepare_model_markdown(&raw_answer);
        paths.write_debug_file("last-agent-context-sanitized.md", &body);
        refresh_mode = KnowledgeRefreshMode::Full;
    }

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
        refresh_mode,
    })
}

/// One generation attempt, ACP or native depending on settings. Shared so a failed incremental
/// turn can be retried as a full regeneration without duplicating the dispatch logic.
async fn run_agent_context_turn(
    paths: &KnowledgePaths,
    engine: Option<&ChatEngine>,
    acp_settings: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
    update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    if execution_pure_acp(acp_settings) {
        run_agent_context_acp(paths, acp_settings, project_slug, repo_path, update).await
    } else {
        let engine = engine
            .ok_or_else(|| anyhow::anyhow!("native agent context generation requires ChatEngine"))?;
        run_agent_context_native(paths, engine, project_slug, repo_path, update).await
    }
}

/// Full-rebuild prompt, or the incremental update prompt when a plan is supplied.
fn context_prompt_for(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    Ok(match update {
        Some(plan) => build_agent_context_update_prompt(paths, project_slug, repo_path, plan)?,
        None => build_agent_context_prompt(paths, project_slug, repo_path)?,
    })
}

async fn run_agent_context_native(
    paths: &KnowledgePaths,
    engine: &ChatEngine,
    project_slug: &str,
    repo_path: &str,
    update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    let prompt = context_prompt_for(paths, project_slug, repo_path, update)?;
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
            |_| {},
        )
        .await?;

    Ok(reply.answer)
}

#[cfg(feature = "opencode")]
async fn run_agent_context_acp(
    paths: &KnowledgePaths,
    acp_settings: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
    update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    use adk_acp::prompt_agent;

    let prompt = build_agent_context_acp_prompt(paths, project_slug, repo_path, update)?;
    let config = agent_context_acp_config(paths, acp_settings, project_slug, repo_path);
    prompt_agent(&config, &prompt)
        .await
        .map_err(|e| anyhow::anyhow!("ACP agent context generation failed: {e}"))
}

#[cfg(not(feature = "opencode"))]
async fn run_agent_context_acp(
    _paths: &KnowledgePaths,
    _acp_settings: &AcpSettings,
    _project_slug: &str,
    _repo_path: &str,
    _update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    anyhow::bail!("ACP agent context generation requires opencode feature")
}

fn build_agent_context_acp_prompt(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    update: Option<&IncrementalPlan>,
) -> anyhow::Result<String> {
    let base = context_prompt_for(paths, project_slug, repo_path, update)?;
    let skill_dir = default_agent_arch_acp_skill_dir();
    let skill_dir_s = skill_dir.display().to_string();
    let ask_skill_dir = default_ask_acp_skill_dir();
    let ask_skill_s = ask_skill_dir.display().to_string();
    let knowledge_root = paths
        .knowledge_root_for(Some(project_slug))
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let output_path = paths.agent_context_main(project_slug).display().to_string();

    Ok(format!(
        "You are Terrain Agent Context generation running in **ACP mode**. \
         Native function tools are NOT available.\n\
         Read the output contract at `{skill_dir_s}/SKILL.md`.\n\
         For repomix discovery use the **`terrain tools`** CLI (subprocess) — \
         command reference in `{ask_skill_s}/SKILL.md` (`grep-pack`, `read-pack-file`, `pack-meta`).\n\
         Do NOT read the live repository filesystem for code discovery.\n\n\
         Environment:\n\
         - TERRAIN_AGENT_ARCH_SKILL={skill_dir_s}\n\
         - TERRAIN_AGENT_CONTEXT_OUTPUT={output_path}\n\
         - TERRAIN_KNOWLEDGE_ROOT={knowledge_root}\n\
         - TERRAIN_PROJECT_SLUG={project_slug}\n\
         - TERRAIN_REPO_PATH={repo_path}\n\
         - TERRAIN_ASK_SKILL={ask_skill_s}\n\n\
         Return ONLY the final markdown document in your reply \
         (Terrain will persist it). Do not include reasoning outside the document.\n\n\
         {base}"
    ))
}

#[cfg(feature = "opencode")]
fn agent_context_acp_config(
    paths: &KnowledgePaths,
    acp_settings: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
) -> adk_acp::AcpAgentConfig {
    let skill_dir = default_agent_arch_acp_skill_dir();
    let output_path = paths.agent_context_main(project_slug);
    let mut env = HashMap::new();
    env.insert(
        "TERRAIN_AGENT_ARCH_SKILL".into(),
        skill_dir.display().to_string(),
    );
    env.insert(
        "TERRAIN_AGENT_CONTEXT_OUTPUT".into(),
        output_path.display().to_string(),
    );
    env.insert(
        "TERRAIN_ASK_SKILL".into(),
        default_ask_acp_skill_dir().display().to_string(),
    );
    env.insert(
        "TERRAIN_KNOWLEDGE_ROOT".into(),
        paths
            .knowledge_root_for(Some(project_slug))
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    );
    env.insert("TERRAIN_PROJECT_SLUG".into(), project_slug.to_string());
    env.insert("TERRAIN_REPO_PATH".into(), repo_path.to_string());
    build_acp_config(acp_settings, Some(repo_path), env)
}

pub fn agent_context_exists(paths: &KnowledgePaths, project_slug: &str) -> bool {
    agent_context_ready(paths, project_slug)
}
