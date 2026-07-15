use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use terrain_agent::{
    agent_execution_ready, execution_uses_native_llm, resolve_acp_settings, resolve_model_config,
    run_agent_context_generation, run_litho_generation, ChatEngine,
};
use terrain_core::{
    build_generation_plan, grep_file, list_human_docs, pack_agent_assets, plan_litho_generation,
    register_project, resolve_project_repo_path, write_agent_context, KnowledgePaths,
};

use crate::cli::AssetCommands;
use crate::util::{print_json, slug_from};

pub async fn run(paths: &KnowledgePaths, command: AssetCommands) -> Result<()> {
    match command {
        AssetCommands::PackAgent { repo_path, slug } => {
            let slug = slug_from(&repo_path, slug);
            let report =
                pack_agent_assets(paths, &slug, &repo_path.display().to_string()).await?;
            print_json(&report)
        }
        AssetCommands::PlanLitho { repo_path, slug } => {
            let slug = slug_from(&repo_path, slug);
            let plan = plan_litho_generation(paths, &slug, &repo_path);
            print_json(&plan)
        }
        AssetCommands::Plan { repo_path, slug } => {
            let slug = slug_from(&repo_path, slug);
            let plan = build_generation_plan(paths, &slug, &repo_path.display().to_string());
            print_json(&plan)
        }
        AssetCommands::RunLitho { repo_path, slug } => {
            let slug = slug_from(&repo_path, slug);
            let repo = repo_path.display().to_string();
            let acp = resolve_acp_settings();
            let result = run_litho_generation(
                paths,
                &slug,
                &repo,
                &acp,
                false,
                |p| eprintln!("[{}] {}", p.stage, p.message),
            )
            .await?;
            print_json(&result)
        }
        AssetCommands::ListHuman { project } => {
            let docs = list_human_docs(paths, &project)?;
            print_json(&docs)
        }
        AssetCommands::GrepPack {
            project,
            pattern,
            context,
            limit,
        } => {
            let pack = paths.agent_pack_main(&project);
            let hits = grep_file(&pack, &pattern, context, limit)?;
            print_json(&hits)
        }
        AssetCommands::AgentContext {
            repo_path,
            slug,
            force,
        } => run_agent_context(paths, repo_path, slug, force).await,
        AssetCommands::Register { repo_path, slug } => register(paths, repo_path, slug),
        AssetCommands::RepairContext { slug, repo_path } => {
            repair_context(paths, slug, repo_path)
        }
    }
}

async fn run_agent_context(
    paths: &KnowledgePaths,
    repo_path: PathBuf,
    slug: Option<String>,
    force: bool,
) -> Result<()> {
    let slug = slug_from(&repo_path, slug);
    let repo = repo_path.display().to_string();
    register_project(&slug, &repo)?;
    paths.ensure_project_layout(&slug)?;

    let context_path = paths.agent_context_main(&slug);
    if force && context_path.is_file() {
        std::fs::remove_file(&context_path)?;
        let meta_path = paths.agent_context_meta(&slug);
        if meta_path.is_file() {
            std::fs::remove_file(meta_path)?;
        }
    }

    let model_config = resolve_model_config();
    let acp = resolve_acp_settings();
    agent_execution_ready(&acp, &model_config).map_err(|e| anyhow::anyhow!(e))?;
    let engine = if execution_uses_native_llm(&acp) {
        Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config)?))
    } else {
        None
    };
    let result = run_agent_context_generation(paths, engine, &acp, &slug, &repo).await?;
    print_json(&result)
}

fn register(paths: &KnowledgePaths, repo_path: PathBuf, slug: Option<String>) -> Result<()> {
    let slug = slug_from(&repo_path, slug);
    let repo = repo_path.display().to_string();
    register_project(&slug, &repo)?;
    paths.ensure_project_layout(&slug)?;
    let knowledge_root = terrain_core::knowledge_root_for_repo(&repo_path);
    println!(
        "{{\"slug\":\"{slug}\",\"repo_path\":\"{repo}\",\"knowledge_root\":\"{}\"}}",
        knowledge_root.display()
    );
    Ok(())
}

fn repair_context(
    paths: &KnowledgePaths,
    slug: String,
    repo_path: Option<PathBuf>,
) -> Result<()> {
    let repo_hint = repo_path.as_ref().map(|p| p.display().to_string());
    let repo = resolve_project_repo_path(paths, &slug, repo_hint.as_deref())?;
    let raw_path = KnowledgePaths::debug_dir().join("last-agent-context-raw.md");
    let raw = std::fs::read_to_string(&raw_path)
        .with_context(|| format!("read {}", raw_path.display()))?;
    let meta = write_agent_context(paths, &slug, &repo, &raw)?;
    print_json(&meta)
}
