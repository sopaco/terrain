use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use terrain_core::{
    build_litho_composition_prompt, build_litho_generation_prompt, count_markdown_in_dir,
    has_litho_research_artifacts, litho_human_complete_with_research, litho_research_ready,
    plan_litho_generation, KnowledgePaths, LithoGenerationJob, LithoGenerationResult, LithoPlan,
    LithoProgress, ProgressEvent,
};

use crate::acp::{acp_spawn_command, build_acp_config};
use crate::settings::AcpSettings;

const POLL_INTERVAL_SECS: u64 = 3;
const POLL_INTERVAL_STABLE_SECS: u64 = 6;
const STABLE_TICKS: u32 = 10;
const MAX_COMPOSITION_ATTEMPTS: u32 = 3;
const DEFAULT_WALL_TIMEOUT_SECS: u64 = 45 * 60;

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

fn litho_wall_timeout() -> Duration {
    let secs = std::env::var("TERRAIN_LITHO_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_WALL_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

fn human_complete(human_dir: &Path, litho_workspace: &Path) -> bool {
    litho_human_complete_with_research(human_dir, Some(litho_workspace))
}

fn clear_litho_outputs(human_dir: &Path, litho_workspace: &Path) -> anyhow::Result<()> {
    if human_dir.is_dir() {
        std::fs::remove_dir_all(human_dir)?;
    }
    if litho_workspace.is_dir() {
        std::fs::remove_dir_all(litho_workspace)?;
    }
    Ok(())
}

fn build_result(
    job: LithoGenerationJob,
    response_excerpt: String,
    human_dir: &Path,
    litho_workspace: &Path,
) -> LithoGenerationResult {
    let human_doc_count = count_markdown_in_dir(human_dir);
    LithoGenerationResult {
        plan: job.plan,
        response_excerpt,
        human_doc_count,
        human_docs_complete: human_complete(human_dir, litho_workspace),
    }
}

/// Run Litho document generation via OpenCode ACP.
#[cfg(feature = "opencode")]
#[allow(clippy::too_many_arguments)]
async fn prompt_agent_with_doc_poll(
    config: adk_acp::AcpAgentConfig,
    prompt: String,
    human_dir: PathBuf,
    research_dir: Option<PathBuf>,
    litho_workspace: PathBuf,
    stage: &str,
    waiting_message: String,
    mut on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<String> {
    use adk_acp::prompt_agent;

    let stage_label = stage.to_string();
    let mut agent_handle = tokio::spawn(async move { prompt_agent(&config, &prompt).await });

    let poll_interval = Duration::from_secs(POLL_INTERVAL_SECS);
    let poll_interval_stable = Duration::from_secs(POLL_INTERVAL_STABLE_SECS);
    let wall_timeout = litho_wall_timeout();
    let started = Instant::now();

    let mut last_human = count_markdown_in_dir(&human_dir);
    let mut last_research = research_dir
        .as_ref()
        .map(count_markdown_in_dir)
        .unwrap_or(0);
    let mut stable_ticks = 0u32;

    loop {
        if started.elapsed() >= wall_timeout {
            agent_handle.abort();
            anyhow::bail!(
                "Litho ACP session exceeded wall timeout ({}s). \
                 human docs: {last_human}, research docs: {last_research}",
                wall_timeout.as_secs()
            );
        }

        tokio::select! {
            result = &mut agent_handle => {
                let inner = result.map_err(|e| anyhow::anyhow!("ACP litho task failed: {e}"))?;
                return inner.map_err(|e| anyhow::anyhow!("ACP litho agent failed: {e}"));
            }
            _ = tokio::time::sleep(if stable_ticks > 0 {
                poll_interval_stable
            } else {
                poll_interval
            }) => {
                let human = count_markdown_in_dir(&human_dir);
                let research = research_dir
                    .as_ref()
                    .map(count_markdown_in_dir)
                    .unwrap_or(last_research);

                if human > last_human || research > last_research {
                    last_human = human;
                    last_research = research;
                    stable_ticks = 0;
                    let detail = if research_dir.is_some() && research > 0 {
                        format!("{waiting_message}（human {human} 篇，研究稿 {research} 篇）")
                    } else {
                        format!("{waiting_message}（已写入 {human} 篇）")
                    };
                    on_progress(ProgressEvent::litho(stage_label.clone(), detail));
                } else if human_complete(&human_dir, &litho_workspace) {
                    stable_ticks += 1;
                    on_progress(ProgressEvent::litho(
                        stage_label.clone(),
                        format!("{waiting_message}（已写入 {human} 篇，等待 Agent 结束…）"),
                    ));
                    if stable_ticks >= STABLE_TICKS {
                        agent_handle.abort();
                        tracing::warn!(
                            human,
                            "litho: full human doc set detected but ACP session did not finish — completing early"
                        );
                        on_progress(ProgressEvent::litho(
                            "done",
                            format!(
                                "已检测到完整的 Litho 文档集（{human} 篇），Agent 会话超时已结束等待"
                            ),
                        ));
                        return Ok(String::new());
                    }
                } else {
                    stable_ticks = 0;
                    let detail = if research_dir.is_some() {
                        format!("{waiting_message}（human {human} 篇，研究稿 {research} 篇）")
                    } else {
                        waiting_message.clone()
                    };
                    on_progress(ProgressEvent::litho(stage_label.clone(), detail));
                }
            }
        }
    }
}

#[cfg(feature = "opencode")]
async fn run_composition_phase(
    acp_settings: &AcpSettings,
    repo_path: &str,
    job: &LithoGenerationJob,
    human_dir: PathBuf,
    litho_workspace: PathBuf,
    on_progress: &mut impl FnMut(LithoProgress),
) -> anyhow::Result<String> {
    on_progress(ProgressEvent::litho("composing", "正在将研究结果整理为人类友好的知识库…"));
    let composition_prompt = build_litho_composition_prompt(&job.plan);
    prompt_agent_with_doc_poll(
        litho_acp_config(acp_settings, repo_path, &job.plan),
        composition_prompt,
        human_dir,
        None,
        litho_workspace,
        "composing",
        "正在将研究结果整理为人类友好的知识库…".into(),
        on_progress,
    )
    .await
}

#[cfg(feature = "opencode")]
async fn run_composition_with_retries(
    acp_settings: &AcpSettings,
    repo_path: &str,
    job: &LithoGenerationJob,
    human_dir: PathBuf,
    litho_workspace: PathBuf,
    on_progress: &mut impl FnMut(LithoProgress),
) -> anyhow::Result<String> {
    let mut last_excerpt = String::new();
    for attempt in 1..=MAX_COMPOSITION_ATTEMPTS {
        if attempt > 1 {
            on_progress(ProgressEvent::litho(
                "composing",
                format!(
                    "Litho 文档仍不完整，正在重试编排（第 {attempt}/{MAX_COMPOSITION_ATTEMPTS} 次）…"
                ),
            ));
        }
        let response = run_composition_phase(
            acp_settings,
            repo_path,
            job,
            human_dir.clone(),
            litho_workspace.clone(),
            on_progress,
        )
        .await?;
        if !response.is_empty() {
            last_excerpt = response.chars().take(500).collect();
        }
        if human_complete(&human_dir, &litho_workspace) {
            return Ok(last_excerpt);
        }
    }
    let human_doc_count = count_markdown_in_dir(&human_dir);
    anyhow::bail!(
        "Litho composition finished but the human doc set is still incomplete \
         ({human_doc_count} file(s) under {}). \
         Research workspace: {}. \
         Try re-running Litho generation.",
        human_dir.display(),
        litho_workspace.display()
    );
}

/// Run Litho document generation via OpenCode ACP.
#[cfg(feature = "opencode")]
pub async fn run_litho_generation(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    acp_settings: &AcpSettings,
    force_refresh: bool,
    mut on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<LithoGenerationResult> {
    let job = prepare_litho_generation(paths, project_slug, repo_path, acp_settings);
    if !job.plan.skill_ready {
        anyhow::bail!("Litho skill not found at {}", job.plan.skill_dir);
    }

    let human_dir = paths.human_docs_dir(project_slug);
    let litho_workspace = PathBuf::from(&job.plan.litho_workspace_dir);

    if force_refresh {
        on_progress(ProgressEvent::litho("starting", "正在清理旧的人类友好知识库以便重新生成…"));
        clear_litho_outputs(&human_dir, &litho_workspace)?;
    } else if human_complete(&human_dir, &litho_workspace) {
        let human_doc_count = count_markdown_in_dir(&human_dir);
        on_progress(ProgressEvent::litho(
            "done",
            format!("人类友好的知识库已完整（{human_doc_count} 篇）"),
        ));
        return Ok(build_result(job, String::new(), &human_dir, &litho_workspace));
    }

    on_progress(ProgressEvent::litho(
        "starting",
        format!("Spawning ACP agent ({})…", acp_spawn_command(acp_settings)),
    ));

    std::fs::create_dir_all(&job.plan.human_output_dir)?;
    std::fs::create_dir_all(&job.plan.litho_workspace_dir)?;

    let research_ready = litho_research_ready(&litho_workspace);

    let response_excerpt = if research_ready {
        run_composition_with_retries(
            acp_settings,
            repo_path,
            &job,
            human_dir.clone(),
            litho_workspace.clone(),
            &mut on_progress,
        )
        .await?
    } else {
        on_progress(ProgressEvent::litho("generating", "Agent 正在分析仓库并生成人类友好的知识库…"));

        let prompt = build_litho_generation_prompt(&job.plan);
        let response = prompt_agent_with_doc_poll(
            litho_acp_config(acp_settings, repo_path, &job.plan),
            prompt,
            human_dir.clone(),
            Some(litho_workspace.clone()),
            litho_workspace.clone(),
            "generating",
            "Agent 正在分析仓库并生成人类友好的知识库…".into(),
            &mut on_progress,
        )
        .await?;
        let mut excerpt: String = response.chars().take(500).collect();

        if !human_complete(&human_dir, &litho_workspace)
            && (litho_research_ready(&litho_workspace) || has_litho_research_artifacts(&litho_workspace))
        {
            excerpt = run_composition_with_retries(
                acp_settings,
                repo_path,
                &job,
                human_dir.clone(),
                litho_workspace.clone(),
                &mut on_progress,
            )
            .await?;
        }
        excerpt
    };

    let human_doc_count = count_markdown_in_dir(&human_dir);
    if human_doc_count == 0 {
        anyhow::bail!(
            "Litho generation finished but no human docs were written to {}. \
             Research artifacts under {}. Agent response excerpt: {}",
            human_dir.display(),
            litho_workspace.display(),
            response_excerpt.chars().take(300).collect::<String>()
        );
    }

    if !human_complete(&human_dir, &litho_workspace) {
        anyhow::bail!(
            "Litho generation finished but the human doc set is still incomplete \
             ({human_doc_count} file(s) under {}). Research workspace: {}.",
            human_dir.display(),
            litho_workspace.display()
        );
    }

    on_progress(ProgressEvent::litho(
        "done",
        format!("Generation finished — {human_doc_count} human doc(s) on disk"),
    ));

    Ok(build_result(job, response_excerpt, &human_dir, &litho_workspace))
}

#[cfg(feature = "opencode")]
fn litho_acp_config(
    acp_settings: &AcpSettings,
    repo_path: &str,
    plan: &LithoPlan,
) -> adk_acp::AcpAgentConfig {
    use std::collections::HashMap;

    let mut env = HashMap::new();
    env.insert("TERRAIN_LITHO_SKILL".into(), plan.skill_dir.clone());
    env.insert(
        "TERRAIN_HUMAN_OUTPUT_DIR".into(),
        plan.human_output_dir.clone(),
    );
    env.insert(
        "TERRAIN_LITHO_WORKSPACE".into(),
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
    _force_refresh: bool,
    _on_progress: impl FnMut(LithoProgress),
) -> anyhow::Result<LithoGenerationResult> {
    anyhow::bail!("ACP support not enabled (rebuild with opencode feature)")
}
