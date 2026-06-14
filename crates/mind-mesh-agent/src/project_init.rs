use std::sync::Arc;

use mind_mesh_core::{
    agent_context_ready, list_human_docs, pack_agent_assets, ProjectScanner, ScanReport,
    KnowledgePaths,
};

use crate::acp_available;
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
    pub notes: Vec<String>,
}

/// Scan the repo, then generate missing agent context and human docs.
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

    let needs_context = !agent_context_ready(paths, &project_slug);
    let needs_human = list_human_docs(paths, &project_slug)
        .map(|docs| docs.is_empty())
        .unwrap_or(true);

    let mut agent_context_generated = false;
    if needs_context {
        if llm_status(model_config).ready {
            on_progress(ProjectInitProgress {
                stage: "agent_context".into(),
                message: "正在生成 Agent 友好的知识资产…".into(),
            });
            if !mind_mesh_core::agent_pack_ready(paths, &project_slug) {
                pack_agent_assets(paths, &project_slug, repo_path).await?;
            }
            let engine = Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?);
            run_agent_context_generation(paths, engine, &project_slug, repo_path).await?;
            agent_context_generated = true;
        } else {
            notes.push("Agent 友好的知识资产：请先在设置中配置 LLM".into());
        }
    }

    let mut human_doc_count = list_human_docs(paths, &project_slug)
        .map(|d| d.len())
        .unwrap_or(0);

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
        } else {
            notes.push("人类友好的知识库：请先在设置中配置 ACP 代理".into());
        }
    }

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
        notes,
    })
}
