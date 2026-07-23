use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use terrain_agent::{ask_knowledge, validate_repo_path, Runtime};
use terrain_core::{list_ask_sessions, KnowledgePaths};

use crate::cli::AskCommands;
use crate::util::{print_json, workspace_project_slug};

pub async fn run(
    paths: &KnowledgePaths,
    cli_repo: Option<PathBuf>,
    command: AskCommands,
) -> Result<()> {
    match command {
        AskCommands::Query {
            query,
            project,
            stream,
        } => {
            let project = project.or_else(|| workspace_project_slug(paths));
            let repo_path = cli_repo
                .or_else(|| paths.workspace_repo().map(|p| p.to_path_buf()))
                .map(|p| p.display().to_string());
            if let Some(ref slug) = project {
                let repo = terrain_core::resolve_project_repo_path(paths, slug, repo_path.as_deref())?;
                validate_repo_path(&repo).map_err(|e| anyhow::anyhow!(e))?;
            }

            let runtime = Runtime::new(paths.clone());
            let session_id = format!("cli-ask-{}", query.len());

            if stream {
                let write_lock = Mutex::new(());
                ask_knowledge(
                    &runtime,
                    &session_id,
                    &query,
                    project.as_deref(),
                    repo_path.as_deref(),
                    |event| {
                        if let Ok(line) = serde_json::to_string(&event)
                            && let Ok(_guard) = write_lock.lock()
                        {
                            let mut stdout = io::stdout();
                            let _ = writeln!(stdout, "{line}");
                            let _ = stdout.flush();
                        }
                    },
                )
                .await?;
            } else {
                let reply = ask_knowledge(
                    &runtime,
                    &session_id,
                    &query,
                    project.as_deref(),
                    repo_path.as_deref(),
                    |_| {},
                )
                .await?;

                print_json(&serde_json::json!({
                    "answer": reply.answer,
                    "citations": reply.citations,
                    "tool_calls": reply.tool_calls,
                    "usage": reply.usage,
                    "completed_at": reply.completed_at,
                }))?;
            }
        }
        AskCommands::SessionsList { project } => {
            let sessions = list_ask_sessions(paths, &project);
            print_json(&sessions)?;
        }
    }
    Ok(())
}
