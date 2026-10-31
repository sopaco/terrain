//! Fleet dashboard CLI: summary, batch updates, multi-project registration.
//!
//! Output is JSON (`--json`-style by design) so it composes with pipelines and
//! agent loops.

use crate::cli::DashboardCommands;
use anyhow::{Context, Result};
use serde_json;
use terrain_core::paths::KnowledgePaths;
use terrain_dashboard::{BatchProgressEvent, add_projects, batch_update_graphs, scan_dashboard};

use std::sync::Arc;

pub async fn run(paths: &KnowledgePaths, command: DashboardCommands) -> Result<()> {
    match command {
        DashboardCommands::Summary => {
            let summary = scan_dashboard(paths, None).context("scan dashboard")?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
            Ok(())
        }
        DashboardCommands::Batch {
            projects,
            concurrency,
        } => {
            let on_progress: Arc<dyn Fn(BatchProgressEvent) + Send + Sync> = Arc::new(|event| {
                println!(
                    "[{}] {}: {}",
                    event.stage, event.project_slug, event.message
                );
            });
            let result = batch_update_graphs(projects, Some(concurrency), on_progress)
                .await
                .context("batch update")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
        DashboardCommands::Add { repo_paths } => {
            let result = add_projects(paths, repo_paths)
                .await
                .context("add projects")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }
    }
}
