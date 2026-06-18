use std::collections::HashMap;
use std::sync::Arc;

use mind_mesh_core::{
    agent_context_ready, agent_pack_ready, build_agent_context_prompt, pack_agent_assets,
    prepare_model_markdown, write_agent_context, AgentContextMeta, KnowledgePaths,
};

use crate::acp::{
    build_acp_config, default_agent_arch_acp_skill_dir, default_ask_acp_skill_dir,
    execution_pure_acp,
};
use crate::chat::ChatEngine;
use crate::settings::AcpSettings;

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentContextGenerationResult {
    pub output_path: String,
    pub meta: AgentContextMeta,
    pub response_excerpt: String,
}

/// Generate `agent/context.md` — architecture-level context for agents (not source code).
pub async fn run_agent_context_generation(
    paths: &KnowledgePaths,
    engine: Option<Arc<ChatEngine>>,
    acp_settings: &AcpSettings,
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

    let raw_answer = if execution_pure_acp(acp_settings) {
        run_agent_context_acp(paths, acp_settings, project_slug, repo_path).await?
    } else {
        let engine = engine
            .ok_or_else(|| anyhow::anyhow!("native agent context generation requires ChatEngine"))?;
        run_agent_context_native(paths, &engine, project_slug, repo_path).await?
    };

    paths.write_debug_file("last-agent-context-raw.md", &raw_answer);
    let body = prepare_model_markdown(&raw_answer);
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

async fn run_agent_context_native(
    paths: &KnowledgePaths,
    engine: &ChatEngine,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<String> {
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

    Ok(reply.answer)
}

#[cfg(feature = "opencode")]
async fn run_agent_context_acp(
    paths: &KnowledgePaths,
    acp_settings: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<String> {
    use adk_acp::prompt_agent;

    let prompt = build_agent_context_acp_prompt(paths, project_slug, repo_path)?;
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
) -> anyhow::Result<String> {
    anyhow::bail!("ACP agent context generation requires opencode feature")
}

fn build_agent_context_acp_prompt(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> anyhow::Result<String> {
    let base = build_agent_context_prompt(paths, project_slug, repo_path)?;
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
        "You are MindMesh Agent Context generation running in **ACP mode**. \
         Native function tools are NOT available.\n\
         Read the output contract at `{skill_dir_s}/SKILL.md`.\n\
         For repomix discovery use the **`mind-mesh tools`** CLI (subprocess) — \
         command reference in `{ask_skill_s}/SKILL.md` (`grep-pack`, `read-pack-file`, `pack-meta`).\n\
         Do NOT read the live repository filesystem for code discovery.\n\n\
         Environment:\n\
         - MIND_MESH_AGENT_ARCH_SKILL={skill_dir_s}\n\
         - MIND_MESH_AGENT_CONTEXT_OUTPUT={output_path}\n\
         - MIND_MESH_KNOWLEDGE_ROOT={knowledge_root}\n\
         - MIND_MESH_PROJECT_SLUG={project_slug}\n\
         - MIND_MESH_REPO_PATH={repo_path}\n\
         - MIND_MESH_ASK_SKILL={ask_skill_s}\n\n\
         Return ONLY the final markdown document in your reply \
         (MindMesh will persist it). Do not include reasoning outside the document.\n\n\
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
        "MIND_MESH_AGENT_ARCH_SKILL".into(),
        skill_dir.display().to_string(),
    );
    env.insert(
        "MIND_MESH_AGENT_CONTEXT_OUTPUT".into(),
        output_path.display().to_string(),
    );
    env.insert(
        "MIND_MESH_ASK_SKILL".into(),
        default_ask_acp_skill_dir().display().to_string(),
    );
    env.insert(
        "MIND_MESH_KNOWLEDGE_ROOT".into(),
        paths
            .knowledge_root_for(Some(project_slug))
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    );
    env.insert("MIND_MESH_PROJECT_SLUG".into(), project_slug.to_string());
    env.insert("MIND_MESH_REPO_PATH".into(), repo_path.to_string());
    build_acp_config(acp_settings, Some(repo_path), env)
}

pub fn agent_context_exists(paths: &KnowledgePaths, project_slug: &str) -> bool {
    agent_context_ready(paths, project_slug)
}
