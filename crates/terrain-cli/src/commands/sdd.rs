use std::path::PathBuf;

use anyhow::Result;
use terrain_agent::{execution_pure_acp, run_sdd_phase, validate_repo_path, Runtime};
use terrain_core::{get_sdd_status, resolve_sdd_session_id, KnowledgePaths, SddPhase};

use crate::cli::SddCommands;
use crate::util::{print_json, require_repo_path};

pub async fn run(
    paths: &KnowledgePaths,
    cli_repo: Option<PathBuf>,
    command: SddCommands,
) -> Result<()> {
    match command {
        SddCommands::Status { project } => {
            let status = get_sdd_status(paths, &project);
            print_json(&status)
        }
        SddCommands::Run {
            project,
            repo_path,
            phase,
            session_id,
            input,
        } => {
            let repo_path = require_repo_path(cli_repo, repo_path)?;
            let repo = repo_path.display().to_string();
            validate_repo_path(&repo).map_err(|e| anyhow::anyhow!(e))?;
            let slug = project;
            let session_id = session_id
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| resolve_sdd_session_id(paths, &slug));
            let phase: SddPhase = phase.into();
            let user_input = input.unwrap_or_default();
            let runtime = Runtime::new(paths.clone());
            let acp = runtime.acp_settings();
            let engine = if execution_pure_acp(&acp) || phase == SddPhase::CodeGen {
                None
            } else {
                Some(runtime.chat_engine()?)
            };
            let result = run_sdd_phase(
                paths,
                engine,
                &slug,
                &repo,
                &session_id,
                phase,
                &user_input,
                &acp,
                |p| eprintln!("[sdd:{}] {}", p.stage, p.message),
            )
            .await?;
            print_json(&result)
        }
    }
}
