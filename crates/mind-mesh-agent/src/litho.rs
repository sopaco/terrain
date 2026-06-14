use std::path::Path;

use mind_mesh_core::{
    build_litho_composition_prompt, build_litho_generation_prompt, has_litho_research_artifacts,
    plan_litho_generation, KnowledgePaths, LithoPlan,
};

use crate::acp::{acp_spawn_command, build_acp_config};
use crate::settings::AcpSettings;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LithoGenerationJob {
    pub plan: LithoPlan,
    pub prompt: String,
    pub acp_command: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LithoProgress {
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LithoGenerationResult {
    pub plan: LithoPlan,
    pub response_excerpt: String,
    pub human_doc_count: usize,
}

pub fn prepare_litho_generation(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    acp_settings: &AcpSettings,
) -> LithoGenerationJob {
    let plan = plan_litho_generation(paths, project_slug, repo_path);
    let prompt = build_litho_generation_prompt(&plan);
    let spawn = acp_spawn_command(acp_settings);
    let acp_command = format!("{spawn} --cwd {repo_path}");

    let status = if plan.skill_ready {
        "ready".into()
    } else {
        "skill_not_found".into()
    };

    LithoGenerationJob {
        plan,
        prompt,
        acp_command,
        status,
    }
}

/// Run Litho document generation via OpenCode ACP.
#[cfg(feature = "opencode")]
async fn prompt_agent_with_doc_poll(
    config: adk_acp::AcpAgentConfig,
    prompt: String,
    human_dir: std::path::PathBuf,
    stage: &str,
    waiting_message: String,
    mut on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<String> {
    use adk_acp::prompt_agent;

    let stage_label = stage.to_string();
    let mut agent_handle = tokio::spawn(async move { prompt_agent(&config, &prompt).await });

    let poll_interval = std::time::Duration::from_secs(3);
    const STABLE_TICKS: u32 = 10;

    let mut last_count = count_markdown_files(&human_dir);
    let mut stable_ticks = 0u32;

    loop {
        tokio::select! {
            result = &mut agent_handle => {
                let inner = result.map_err(|e| anyhow::anyhow!("ACP litho task failed: {e}"))?;
                return inner.map_err(|e| anyhow::anyhow!("ACP litho agent failed: {e}"));
            }
            _ = tokio::time::sleep(poll_interval) => {
                let count = count_markdown_files(&human_dir);
                if count > last_count {
                    last_count = count;
                    stable_ticks = 0;
                    on_progress(LithoProgress {
                        stage: stage_label.clone(),
                        message: format!("{waiting_message}（已写入 {count} 篇）"),
                    });
                } else if count > 0 {
                    stable_ticks += 1;
                    on_progress(LithoProgress {
                        stage: stage_label.clone(),
                        message: format!(
                            "{waiting_message}（已写入 {count} 篇，等待 Agent 结束…）"
                        ),
                    });
                    if stable_ticks >= STABLE_TICKS {
                        agent_handle.abort();
                        tracing::warn!(
                            count,
                            "litho: human docs stopped changing but ACP session did not finish — completing early"
                        );
                        on_progress(LithoProgress {
                            stage: "done".into(),
                            message: format!(
                                "已检测到 {count} 篇 Human 文档，Agent 会话超时已结束等待"
                            ),
                        });
                        return Ok(String::new());
                    }
                } else {
                    on_progress(LithoProgress {
                        stage: stage_label.clone(),
                        message: waiting_message.clone(),
                    });
                }
            }
        }
    }
}

/// Run Litho document generation via OpenCode ACP.
#[cfg(feature = "opencode")]
pub async fn run_litho_generation(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    acp_settings: &AcpSettings,
    mut on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<LithoGenerationResult> {
    let job = prepare_litho_generation(paths, project_slug, repo_path, acp_settings);
    if !job.plan.skill_ready {
        anyhow::bail!("Litho skill not found at {}", job.plan.skill_dir);
    }

    on_progress(LithoProgress {
        stage: "starting".into(),
        message: format!("Spawning ACP agent ({})…", acp_spawn_command(acp_settings)),
    });

    std::fs::create_dir_all(&job.plan.human_output_dir)?;
    std::fs::create_dir_all(&job.plan.litho_workspace_dir)?;

    let human_dir = paths.human_docs_dir(project_slug);
    let mut human_doc_count = count_markdown_files(&human_dir);

    if human_doc_count > 0 {
        on_progress(LithoProgress {
            stage: "done".into(),
            message: format!("Human docs already exist ({human_doc_count} file(s))"),
        });
        return Ok(LithoGenerationResult {
            plan: job.plan,
            response_excerpt: String::new(),
            human_doc_count,
        });
    }

    let research_ready = has_litho_research_artifacts(&job.plan.litho_workspace_dir);
    let response = if research_ready {
        on_progress(LithoProgress {
            stage: "composing".into(),
            message: "Research artifacts found — running composition/output phase…".into(),
        });
        let composition_prompt = build_litho_composition_prompt(&job.plan);
        prompt_agent_with_doc_poll(
            litho_acp_config(acp_settings, repo_path, &job.plan),
            composition_prompt,
            human_dir.clone(),
            "composing",
            "正在将研究结果整理为 Human 文档…".into(),
            &mut on_progress,
        )
        .await?
    } else {
        on_progress(LithoProgress {
            stage: "generating".into(),
            message: "Agent 正在分析仓库并生成 Human 文档…".into(),
        });

        let prompt = build_litho_generation_prompt(&job.plan);
        prompt_agent_with_doc_poll(
            litho_acp_config(acp_settings, repo_path, &job.plan),
            prompt,
            human_dir.clone(),
            "generating",
            "Agent 正在分析仓库并生成 Human 文档…".into(),
            &mut on_progress,
        )
        .await?
    };

    human_doc_count = count_markdown_files(&human_dir);

    if human_doc_count == 0 && !research_ready && has_litho_research_artifacts(&job.plan.litho_workspace_dir) {
        on_progress(LithoProgress {
            stage: "composing".into(),
            message: "Research complete — running composition/output phase…".into(),
        });

        let composition_prompt = build_litho_composition_prompt(&job.plan);
        let composition_response = prompt_agent_with_doc_poll(
            litho_acp_config(acp_settings, repo_path, &job.plan),
            composition_prompt,
            human_dir.clone(),
            "composing",
            "正在将研究结果整理为 Human 文档…".into(),
            &mut on_progress,
        )
        .await?;

        human_doc_count = count_markdown_files(&human_dir);
        if human_doc_count == 0 {
            anyhow::bail!(
                "Litho composition finished but no human docs were written to {}. \
                 Research artifacts are under {}. Agent response excerpt: {}",
                human_dir.display(),
                job.plan.litho_workspace_dir,
                composition_response.chars().take(300).collect::<String>()
            );
        }

        let excerpt: String = composition_response.chars().take(500).collect();
        on_progress(LithoProgress {
            stage: "done".into(),
            message: format!("Generation finished — {human_doc_count} human doc(s) on disk"),
        });

        return Ok(LithoGenerationResult {
            plan: job.plan,
            response_excerpt: excerpt,
            human_doc_count,
        });
    }

    if human_doc_count == 0 {
        anyhow::bail!(
            "Litho generation finished but no human docs were written to {}. \
             Agent response excerpt: {}",
            human_dir.display(),
            response.chars().take(300).collect::<String>()
        );
    }

    on_progress(LithoProgress {
        stage: "done".into(),
        message: format!("Generation finished — {human_doc_count} human doc(s) on disk"),
    });

    let excerpt: String = response.chars().take(500).collect();

    Ok(LithoGenerationResult {
        plan: job.plan,
        response_excerpt: excerpt,
        human_doc_count,
    })
}

#[cfg(feature = "opencode")]
fn litho_acp_config(
    acp_settings: &AcpSettings,
    repo_path: &str,
    plan: &LithoPlan,
) -> adk_acp::AcpAgentConfig {
    use std::collections::HashMap;

    let mut env = HashMap::new();
    env.insert("MIND_MESH_LITHO_SKILL".into(), plan.skill_dir.clone());
    env.insert(
        "MIND_MESH_HUMAN_OUTPUT_DIR".into(),
        plan.human_output_dir.clone(),
    );
    env.insert(
        "MIND_MESH_LITHO_WORKSPACE".into(),
        plan.litho_workspace_dir.clone(),
    );
    build_acp_config(acp_settings, Some(repo_path), env)
}

#[cfg(not(feature = "opencode"))]
pub async fn run_litho_generation(
    _paths: &KnowledgePaths,
    _project_slug: &str,
    _repo_path: &str,
    _acp_settings: &AcpSettings,
    _on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<LithoGenerationResult> {
    anyhow::bail!("ACP support not enabled (rebuild with opencode feature)")
}

fn count_markdown_files(dir: &Path) -> usize {
    if !dir.is_dir() {
        return 0;
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count()
}
