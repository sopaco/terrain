use std::sync::Arc;

use terrain_core::{
    agent_context_ready, agent_context_synced_with_head, compute_freshness,
    litho_human_complete_with_research, KnowledgePaths, KnowledgeRefreshMode, KnowledgeSettings,
    LithoProgress, ProjectScanner, QuickRefreshResult,
};

use crate::acp::{acp_available, agent_execution_ready, execution_pure_acp, execution_uses_native_llm};
use crate::agent_context::run_agent_context_generation;
use crate::chat::ChatEngine;
use crate::litho::LithoRunMode;
use crate::model::ModelConfig;
use crate::settings::AcpSettings;

/// Scan + repack + agent context refresh; Litho only when explicitly opted into.
///
/// Repomix packing runs once inside [`ProjectScanner::scan_repo`]. The context refresh is
/// incremental when [`KnowledgeSettings::incremental_refresh`] is on, so a small commit costs a
/// short diff-driven turn instead of a full architecture pass.
#[allow(clippy::too_many_arguments)]
pub async fn run_quick_refresh(
    paths: &KnowledgePaths,
    model_config: &ModelConfig,
    acp: &AcpSettings,
    knowledge: &KnowledgeSettings,
    repo_path: &str,
    project_slug: &str,
    on_litho_progress: impl Fn(LithoProgress),
) -> anyhow::Result<QuickRefreshResult> {
    let lang = terrain_core::current_language();
    let mut notes = Vec::new();

    let scanner = ProjectScanner::new(paths.clone());
    let scan = scanner
        .scan_repo(repo_path, Some(project_slug))
        .await?;

    let pack_tokens = scan.agent_pack.as_ref().map(|pack| pack.total_tokens);
    if scan.agent_pack.as_ref().is_some_and(|p| p.pack_skipped) {
        notes.push(
            lang.tr(
                "源码索引：已与当前提交同步，已跳过",
                "Source index: in sync with the current commit, skipped",
            )
            .into(),
        );
    } else if pack_tokens.is_none() {
        notes.push(
            lang.tr(
                "源码索引：scan 未执行 repomix 打包（terrain-core 未启用 repomix feature）",
                "Source index: scan did not run repomix packing (terrain-core built without the repomix feature)",
            )
            .into(),
        );
    }

    let mut agent_context_regenerated = false;
    if agent_execution_ready(acp, model_config).is_ok() {
        if !agent_context_synced_with_head(paths, project_slug, repo_path) {
            let engine = if execution_uses_native_llm(acp) {
                Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config.clone())?))
            } else {
                None
            };
            match run_agent_context_generation(
                paths,
                engine,
                acp,
                project_slug,
                repo_path,
                knowledge,
                false,
            )
            .await
            {
                Ok(result) => {
                    agent_context_regenerated =
                        result.refresh_mode != KnowledgeRefreshMode::Skipped;
                    let reason = result.refresh_reason.as_deref();
                    notes.push(match result.refresh_mode {
                        KnowledgeRefreshMode::Incremental => match reason {
                            // The reply was unusable but the agent's own in-place edit was sound.
                            Some(r) if r.starts_with("recovered_from_disk") => lang
                                .tr(
                                    "Agent 友好的知识资产：已增量更新（回复不完整，已采用 Agent 就地修改的文件）",
                                    "Agent knowledge assets: incrementally updated (reply incomplete; adopted the agent's in-place edits)",
                                )
                                .to_string(),
                            _ => lang
                                .tr(
                                    "Agent 友好的知识资产：已按 git diff 增量更新",
                                    "Agent knowledge assets: incrementally updated from git diff",
                                )
                                .to_string(),
                        },
                        // A full run that was *asked for* reads differently from one that had to
                        // rescue a rejected incremental attempt — say which happened.
                        KnowledgeRefreshMode::Full => match reason {
                            Some(r) if r.starts_with("full_after_incremental_") => lang
                                .tr(
                                    &format!(
                                        "Agent 友好的知识资产：增量更新结果不可信（{}），已自动改为完整重新生成",
                                        r.trim_start_matches("full_after_incremental_")
                                    ),
                                    &format!(
                                        "Agent knowledge assets: incremental update untrustworthy ({}); auto-switched to full regeneration",
                                        r.trim_start_matches("full_after_incremental_")
                                    ),
                                )
                                .to_string(),
                            _ => lang
                                .tr(
                                    "Agent 友好的知识资产：已重新生成",
                                    "Agent knowledge assets: regenerated",
                                )
                                .to_string(),
                        },
                        KnowledgeRefreshMode::Skipped => lang
                            .tr(
                                "Agent 友好的知识资产：源码无变更，仅更新基线",
                                "Agent knowledge assets: no source changes; only the baseline was updated",
                            )
                            .to_string(),
                    });
                }
                Err(e) => notes.push(
                    lang.tr(
                        &format!("Agent 知识资产：{e}"),
                        &format!("Agent knowledge assets: {e}"),
                    )
                    .to_string(),
                ),
            }
        } else if agent_context_ready(paths, project_slug) {
            notes.push(
                lang.tr(
                    "Agent 友好的知识资产：已与当前提交同步，已跳过",
                    "Agent knowledge assets: in sync with the current commit, skipped",
                )
                .into(),
            );
        }
    } else if execution_pure_acp(acp) {
        notes.push(
            lang.tr(
                "Agent 友好的知识资产：请先在设置中配置 ACP 代理",
                "Agent knowledge assets: please configure an ACP agent in Settings first",
            )
            .into(),
        );
    } else {
        notes.push(
            lang.tr(
                "Agent 友好的知识资产：请配置 ACP 代理与 LLM",
                "Agent knowledge assets: please configure an ACP agent and an LLM",
            )
            .into(),
        );
    }

    // Litho is the slowest stage, so quick refresh only touches it when the user opted in —
    // and only to *update* an existing doc set. Generating one from scratch is a 数十分钟
    // pipeline and belongs to initialization, not to 快速保鲜.
    if knowledge.incremental_refresh && knowledge.incremental_human_docs {
        let human_dir = paths.human_docs_dir(project_slug);
        let litho_workspace = paths.litho_workspace_dir(project_slug);
        if !litho_human_complete_with_research(&human_dir, Some(&litho_workspace)) {
            notes.push(
                lang.tr(
                    "人类友好的知识库：尚未完整，需先完整生成，已跳过",
                    "Human docs: not complete yet, skipped",
                )
                .into(),
            );
        } else if acp_available(acp) {
            match crate::litho::run_litho_generation(
                paths,
                project_slug,
                repo_path,
                acp,
                knowledge,
                LithoRunMode::Auto,
                &on_litho_progress,
            )
            .await
            {
                Ok(result) => notes.push(match result.refresh_mode {
                    KnowledgeRefreshMode::Incremental => lang
                        .tr(
                            "人类友好的知识库：已按 git diff 增量更新",
                            "Human docs: incrementally updated from git diff",
                        )
                        .into(),
                    KnowledgeRefreshMode::Full => lang
                        .tr("人类友好的知识库：已生成", "Human docs: generated")
                        .to_string(),
                    KnowledgeRefreshMode::Skipped => match result.refresh_reason.as_deref() {
                        Some("too_many_changed_files") => lang
                            .tr(
                                "人类友好的知识库：变更范围过大，需「重新生成」，已跳过",
                                "Human docs: change scope too large, needs \"Regenerate\"; skipped",
                            )
                            .to_string(),
                        Some("no_baseline") => lang
                            .tr(
                                "人类友好的知识库：本次仅记录基线，下次起可增量更新",
                                "Human docs: baseline recorded this run; incremental updates start next time",
                            )
                            .to_string(),
                        _ => lang
                            .tr(
                                "人类友好的知识库：无需更新，已跳过",
                                "Human docs: no update needed, skipped",
                            )
                            .to_string(),
                    },
                }),
                Err(e) => notes.push(
                    lang.tr(
                        &format!("人类友好的知识库：{e}"),
                        &format!("Human docs: {e}"),
                    )
                    .to_string(),
                ),
            }
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
