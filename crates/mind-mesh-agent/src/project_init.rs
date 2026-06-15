use std::sync::Arc;

use mind_mesh_core::{
    agent_context_ready, count_human_docs, litho_human_complete_with_research, pack_agent_assets,
    ProjectScanner, ScanReport, KnowledgePaths,
};

use crate::acp::{acp_available, execution_uses_acp};
use crate::agent_context::run_agent_context_generation;
use crate::chat::ChatEngine;
use crate::litho::{run_litho_generation, LithoProgress};
use crate::model::{llm_status, ModelConfig};
use crate::settings::AcpSettings;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectInitProgress {
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectInitResult {
    pub project_slug: String,
    pub repo_path: String,
    pub scan_files_written: usize,
    pub repack_tokens: Option<usize>,
    pub agent_context_generated: bool,
    pub human_doc_count: usize,
    pub human_docs_complete: bool,
    pub litho_ran: bool,
    pub notes: Vec<String>,
}

async fn run_agent_context_if_needed(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    project_slug: &str,
    repo_path: &str,
    force_refresh: bool,
    on_progress: &impl Fn(ProjectInitProgress),
    notes: &mut Vec<String>,
) -> anyhow::Result<bool> {
    let needs_context = !agent_context_ready(paths, project_slug) || force_refresh;
    if !needs_context {
        return Ok(false);
    }
    if execution_uses_acp(acp) {
        if !acp_available(acp) {
            notes.push("Agent 友好的知识资产：请先在设置中配置 ACP 代理".into());
            return Ok(false);
        }
    } else if !llm_status(model_config).ready {
        notes.push("Agent 友好的知识资产：请先在设置中配置 LLM".into());
        return Ok(false);
    }

    on_progress(ProjectInitProgress {
        stage: "agent_context".into(),
        message: if force_refresh && agent_context_ready(paths, project_slug) {
            "正在根据 Litho 文档刷新 Agent 友好的知识资产…".into()
        } else {
            "正在生成 Agent 友好的知识资产…".into()
        },
    });
    if !mind_mesh_core::agent_pack_ready(paths, project_slug) {
        pack_agent_assets(paths, project_slug, repo_path).await?;
    }
    let engine = if execution_uses_acp(acp) {
        None
    } else {
        Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?))
    };
    run_agent_context_generation(paths, engine, acp, project_slug, repo_path).await?;
    Ok(true)
}

/// Scan the repo, then generate missing human docs and agent context.
pub async fn run_project_initialization(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    repo_path: &str,
    project_slug: Option<&str>,
    on_progress: impl Fn(ProjectInitProgress),
    on_litho_progress: impl Fn(LithoProgress),
) -> anyhow::Result<ProjectInitResult> {
    let mut notes = Vec::new();

    on_progress(ProjectInitProgress {
        stage: "scan".into(),
        message: "正在扫描仓库并建立索引…".into(),
    });

    let scanner = ProjectScanner::new(paths.clone());
    let ScanReport {
        project_slug,
        files_written,
        agent_pack,
        ..
    } = scanner
        .scan_repo(repo_path, project_slug)
        .await?;

    let repack_tokens = agent_pack.as_ref().map(|p| p.total_tokens);

    let human_dir = paths.human_docs_dir(&project_slug);
    let litho_workspace = paths.litho_workspace_dir(&project_slug);
    let needs_human =
        !litho_human_complete_with_research(&human_dir, Some(&litho_workspace));

    let mut human_doc_count = count_human_docs(paths, &project_slug);
    let mut human_docs_complete = !needs_human;
    let mut litho_ran = false;

    if needs_human {
        if acp_available(acp) {
            on_progress(ProjectInitProgress {
                stage: "human_docs".into(),
                message: "正在生成人类友好的知识库（Litho）…".into(),
            });
            let result =
                run_litho_generation(paths, &project_slug, repo_path, acp, &on_litho_progress)
                    .await?;
            human_doc_count = result.human_doc_count;
            human_docs_complete = result.human_docs_complete;
            litho_ran = true;
        } else {
            notes.push("人类友好的知识库：请先在设置中配置 ACP 代理".into());
        }
    }

    let agent_context_generated = run_agent_context_if_needed(
        paths,
        model_config,
        acp,
        &project_slug,
        repo_path,
        litho_ran,
        &on_progress,
        &mut notes,
    )
    .await?;

    on_progress(ProjectInitProgress {
        stage: "done".into(),
        message: "项目初始化完成".into(),
    });

    Ok(ProjectInitResult {
        project_slug,
        repo_path: repo_path.to_string(),
        scan_files_written: files_written,
        repack_tokens,
        agent_context_generated,
        human_doc_count,
        human_docs_complete,
        litho_ran,
        notes,
    })
}
