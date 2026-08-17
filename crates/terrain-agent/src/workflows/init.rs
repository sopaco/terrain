use std::sync::Arc;

use terrain_core::{
    agent_context_ready, count_human_docs, litho_human_complete_with_research, pack_agent_assets,
    KnowledgePaths, KnowledgeSettings, ProgressEvent, ProjectInitResult, ProjectScanner, ScanReport,
};

use crate::acp::{acp_available, execution_pure_acp, execution_uses_native_llm};
use crate::agent_context::run_agent_context_generation;
use crate::chat::ChatEngine;
use crate::litho::LithoRunMode;
use crate::model::{llm_status, ModelConfig};
use crate::settings::AcpSettings;

#[allow(clippy::too_many_arguments)]
async fn run_agent_context_if_needed(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    knowledge: &KnowledgeSettings,
    project_slug: &str,
    repo_path: &str,
    force_refresh: bool,
    force_full: bool,
    on_progress: &impl Fn(ProgressEvent),
    notes: &mut Vec<String>,
) -> anyhow::Result<bool> {
    let lang = terrain_core::current_language();
    let context_ready = agent_context_ready(paths, project_slug);
    // `force_refresh` here means Litho rewrote the human docs the context is derived from.
    let needs_context = !context_ready || force_refresh;
    if !needs_context {
        return Ok(false);
    }
    if execution_pure_acp(acp) {
        if !acp_available(acp) {
            notes.push(
                lang.tr(
                    "Agent 友好的知识资产：请先在设置中配置 ACP 代理",
                    "Agent knowledge assets: please configure an ACP agent in Settings first",
                )
                .into(),
            );
            return Ok(false);
        }
    } else if !acp_available(acp) {
        notes.push(
            lang.tr(
                "Agent 友好的知识资产：请先在设置中配置 ACP 代理",
                "Agent knowledge assets: please configure an ACP agent in Settings first",
            )
            .into(),
        );
        return Ok(false);
    } else if !llm_status(model_config).ready {
        notes.push(
            lang.tr(
                "Agent 友好的知识资产：请先在设置中配置 LLM",
                "Agent knowledge assets: please configure an LLM in Settings first",
            )
            .into(),
        );
        return Ok(false);
    }

    on_progress(ProgressEvent::project_init(
        "agent_context",
        if force_refresh && context_ready {
            lang.tr(
                "正在根据 Litho 文档刷新 Agent 友好的知识资产…",
                "Refreshing agent knowledge assets from the Litho docs…",
            )
        } else {
            lang.tr(
                "正在生成 Agent 友好的知识资产…",
                "Generating agent knowledge assets…",
            )
        },
    ));
    if !terrain_core::agent_pack_ready(paths, project_slug) {
        pack_agent_assets(paths, project_slug, repo_path).await?;
    }
    let engine = if execution_uses_native_llm(acp) {
        Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?))
    } else {
        None
    };
    // A fresh Litho rebuild replaces the narrative the context summarizes, so a diff-driven
    // update would have nothing useful to anchor on — rebuild the context too.
    run_agent_context_generation(
        paths,
        engine,
        acp,
        project_slug,
        repo_path,
        knowledge,
        force_full,
    )
    .await?;
    Ok(true)
}

/// Scan the repo, then generate missing human docs and agent context.
#[allow(clippy::too_many_arguments)]
pub async fn run_project_initialization(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    knowledge: &KnowledgeSettings,
    repo_path: &str,
    project_slug: Option<&str>,
    on_progress: impl Fn(ProgressEvent),
    on_litho_progress: impl Fn(ProgressEvent),
) -> anyhow::Result<ProjectInitResult> {
    let lang = terrain_core::current_language();
    let mut notes = Vec::new();

    on_progress(ProgressEvent::project_init(
        "scan",
        lang.tr(
            "正在扫描仓库并建立索引…",
            "Scanning the repository and building the index…",
        ),
    ));

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
    let needs_human = !litho_human_complete_with_research(&human_dir, Some(&litho_workspace));

    let mut human_doc_count = count_human_docs(paths, &project_slug);
    let mut human_docs_complete = !needs_human;
    let mut litho_ran = false;

    if needs_human {
        if acp_available(acp) {
            on_progress(ProgressEvent::project_init(
                "human_docs",
                lang.tr(
                    "正在生成人类友好的知识库（Litho）…",
                    "Generating the human-friendly knowledge base (Litho)…",
                ),
            ));
            let result = crate::litho::run_litho_generation(
                paths,
                &project_slug,
                repo_path,
                acp,
                knowledge,
                LithoRunMode::Auto,
                &on_litho_progress,
            )
            .await?;
            human_doc_count = result.human_doc_count;
            human_docs_complete = result.human_docs_complete;
            litho_ran = true;
        } else {
            notes.push(
                lang.tr(
                    "人类友好的知识库：请先在设置中配置 ACP 代理",
                    "Human docs: please configure an ACP agent in Settings first",
                )
                .into(),
            );
        }
    }

    let agent_context_generated = run_agent_context_if_needed(
        paths,
        model_config,
        acp,
        knowledge,
        &project_slug,
        repo_path,
        litho_ran,
        litho_ran,
        &on_progress,
        &mut notes,
    )
    .await?;

    on_progress(ProgressEvent::new(
        terrain_core::ProgressKind::Done,
        "done",
        lang.tr("项目初始化完成", "Project initialization complete"),
    ));

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
