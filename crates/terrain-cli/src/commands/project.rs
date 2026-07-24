use anyhow::Result;
use terrain_core::{
    get_project_overview, list_stale_registry_projects, read_freshness_ledger, unregister_project,
    write_project_remark, KnowledgePaths,
};

use crate::cli::ProjectCommands;
use crate::util::print_json;

pub fn run(paths: &KnowledgePaths, command: ProjectCommands) -> Result<()> {
    match command {
        ProjectCommands::Overview { project } => {
            let overview = get_project_overview(paths, &project)?;
            print_json(&overview)
        }
        ProjectCommands::Remark { project, remark } => {
            write_project_remark(paths, &project, &remark)?;
            let overview = get_project_overview(paths, &project)?;
            print_json(&overview)
        }
        ProjectCommands::Remove { project } => {
            unregister_project(&project)?;
            print_json(&serde_json::json!({ "removed": project }))
        }
        ProjectCommands::ListStale => {
            let stale = list_stale_registry_projects()?;
            print_json(&stale)
        }
        ProjectCommands::FreshnessCached { project } => {
            let summary = read_freshness_ledger(paths, &project).map(|l| l.summary);
            print_json(&summary)
        }
    }
}
