use std::sync::Arc;

use mind_mesh_core::{
    build_sdd_llm_prompt, build_sdd_phase_prompt, plan_sdd_workflow, sdd_phase_output_path,
    KnowledgePaths, SddPhase, SddPhaseResult, SddPlan,
};

use crate::acp::{acp_spawn_command, build_acp_config, execution_uses_acp};
use crate::chat::ChatEngine;
use crate::settings::AcpSettings;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SddProgress {
    pub stage: String,
    pub message: String,
}

/// Run a single SDD workflow phase.
pub async fn run_sdd_phase(
    paths: &KnowledgePaths,
    engine: Option<Arc<ChatEngine>>,
    project_slug: &str,
    repo_path: &str,
    phase: SddPhase,
    user_input: &str,
    acp_settings: &AcpSettings,
    mut on_progress: impl FnMut(SddProgress),
) -> anyhow::Result<SddPhaseResult> {
    let plan = plan_sdd_workflow(paths, project_slug, repo_path);
    std::fs::create_dir_all(&plan.sdd_workspace_dir)?;
    std::fs::create_dir_all(&plan.sdd_output_dir)?;

    let output_path = sdd_phase_output_path(&plan.sdd_output_dir, phase);

    if phase.order() > 0 {
        let prior = SddPhase::all()
            .into_iter()
            .find(|p| p.order() + 1 == phase.order());
        if let Some(prior_phase) = prior {
            let prior_path = sdd_phase_output_path(&plan.sdd_output_dir, prior_phase);
            if !prior_path.is_file() {
                anyhow::bail!(
                    "Complete \"{}\" before running \"{}\"",
                    prior_phase.label(),
                    phase.label()
                );
            }
        }
    }

    on_progress(SddProgress {
        stage: "starting".into(),
        message: format!("Running SDD phase: {}", phase.label()),
    });

    let response = if execution_uses_acp(acp_settings) || phase == SddPhase::CodeGen {
        run_sdd_acp_phase(&plan, phase, user_input, acp_settings, &mut on_progress).await?
    } else {
        let engine = engine.ok_or_else(|| {
            anyhow::anyhow!("LLM not configured — set up model in Settings first")
        })?;
        run_sdd_llm_phase(
            &engine,
            &plan,
            project_slug,
            repo_path,
            phase,
            user_input,
            &mut on_progress,
        )
        .await?
    };

    let content = if output_path.is_file() {
        std::fs::read_to_string(&output_path)?
    } else {
        std::fs::write(&output_path, &response)?;
        response.clone()
    };

    if content.trim().is_empty() {
        anyhow::bail!(
            "SDD phase \"{}\" produced empty output at {}",
            phase.label(),
            output_path.display()
        );
    }

    on_progress(SddProgress {
        stage: "done".into(),
        message: format!("{} complete", phase.label()),
    });

    let excerpt: String = content.chars().take(500).collect();
    Ok(SddPhaseResult {
        phase,
        output_path: output_path.display().to_string(),
        response_excerpt: excerpt,
    })
}

async fn run_sdd_llm_phase(
    engine: &ChatEngine,
    plan: &SddPlan,
    project_slug: &str,
    repo_path: &str,
    phase: SddPhase,
    user_input: &str,
    on_progress: &mut impl FnMut(SddProgress),
) -> anyhow::Result<String> {
    on_progress(SddProgress {
        stage: "generating".into(),
        message: "LLM is drafting the document…".into(),
    });

    let prompt = build_sdd_llm_prompt(plan, phase, user_input);
    let session_id = format!("sdd-{project_slug}-{}", phase.order());
    let reply = engine
        .ask(
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
async fn run_sdd_acp_phase(
    plan: &SddPlan,
    phase: SddPhase,
    user_input: &str,
    acp_settings: &AcpSettings,
    on_progress: &mut impl FnMut(SddProgress),
) -> anyhow::Result<String> {
    use adk_acp::prompt_agent;

    if !plan.skill_ready {
        anyhow::bail!("SDD skill not found at {}", plan.skill_dir);
    }

    let action = if phase == SddPhase::CodeGen {
        "implementing changes"
    } else {
        "drafting the document"
    };
    on_progress(SddProgress {
        stage: "generating".into(),
        message: format!(
            "ACP agent ({}) is {action}…",
            acp_spawn_command(acp_settings)
        ),
    });

    let prompt = build_sdd_phase_prompt(plan, phase, user_input);
    let config = sdd_acp_config(acp_settings, &plan.repo_path, plan);
    let response = prompt_agent(&config, &prompt)
        .await
        .map_err(|e| anyhow::anyhow!("ACP SDD phase failed: {e}"))?;
    Ok(response)
}

#[cfg(not(feature = "opencode"))]
async fn run_sdd_acp_phase(
    _plan: &SddPlan,
    _phase: SddPhase,
    _user_input: &str,
    _acp_settings: &AcpSettings,
    _on_progress: &mut impl FnMut(SddProgress),
) -> anyhow::Result<String> {
    anyhow::bail!("ACP support not enabled (rebuild with opencode feature)")
}

#[cfg(feature = "opencode")]
fn sdd_acp_config(
    acp_settings: &AcpSettings,
    repo_path: &str,
    plan: &SddPlan,
) -> adk_acp::AcpAgentConfig {
    use std::collections::HashMap;

    let mut env = HashMap::new();
    env.insert("MIND_MESH_SDD_SKILL".into(), plan.skill_dir.clone());
    env.insert(
        "MIND_MESH_SDD_WORKSPACE".into(),
        plan.sdd_workspace_dir.clone(),
    );
    env.insert("MIND_MESH_SDD_OUTPUT_DIR".into(), plan.sdd_output_dir.clone());
    env.insert(
        "MIND_MESH_HUMAN_OUTPUT_DIR".into(),
        plan.human_output_dir.clone(),
    );
    build_acp_config(acp_settings, Some(repo_path), env)
}
