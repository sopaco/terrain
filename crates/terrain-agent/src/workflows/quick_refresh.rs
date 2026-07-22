use std::sync::Arc;

use terrain_core::{
    agent_context_fresh, agent_context_ready, compute_freshness, KnowledgePaths, ProjectScanner,
    QuickRefreshResult,
};

use crate::acp::{agent_execution_ready, execution_pure_acp, execution_uses_native_llm};
use crate::agent_context::run_agent_context_generation;
use crate::chat::ChatEngine;
use crate::model::ModelConfig;
use crate::settings::AcpSettings;

/// Scan + repack + optional agent context regeneration (skips Litho).
///
/// Repomix packing runs once inside [`ProjectScanner::scan_repo`].
pub async fn run_quick_refresh(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    repo_path: &str,
    project_slug: &str,
) -> anyhow::Result<QuickRefreshResult> {
    let mut notes = Vec::new();

    let scanner = ProjectScanner::new(paths.clone());
    let scan = scanner
        .scan_repo(repo_path, Some(project_slug))
        .await?;

    let pack_tokens = scan.agent_pack.as_ref().map(|pack| pack.total_tokens);
    if scan.agent_pack.as_ref().is_some_and(|p| p.pack_skipped) {
        notes.push("源码索引：已与当前提交同步，已跳过".into());
    } else if pack_tokens.is_none() {
        notes.push(
            "源码索引：scan 未执行 repomix 打包（terrain-core 未启用 repomix feature）".into(),
        );
    }

    let mut agent_context_regenerated = false;
    if agent_execution_ready(acp, model_config).is_ok() {
        if !agent_context_fresh(paths, project_slug, repo_path) {
            let engine = if execution_uses_native_llm(acp) {
                Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?))
            } else {
                None
            };
            match run_agent_context_generation(paths, engine, acp, project_slug, repo_path).await {
                Ok(_) => agent_context_regenerated = true,
                Err(e) => notes.push(format!("Agent 知识资产：{e}")),
            }
        } else if agent_context_ready(paths, project_slug) {
            notes.push("Agent 友好的知识资产：已与当前提交同步，已跳过".into());
        }
    } else if execution_pure_acp(acp) {
        notes.push("Agent 友好的知识资产：请先在设置中配置 ACP 代理".into());
    } else {
        notes.push("Agent 友好的知识资产：请配置 ACP 代理与 LLM".into());
    }

    let freshness = compute_freshness(paths, project_slug, repo_path)?;

    Ok(QuickRefreshResult {
        project_slug: project_slug.to_string(),
        scan_files_written: scan.files_written,
        pack_tokens,
        agent_context_regenerated,
        notes,
        freshness,
    })
}
