mod assets;
mod env;
mod knowledge;
mod tools;

use anyhow::{Context, Result};

use crate::cli::{Cli, Commands};

pub async fn run(cli: Cli) -> Result<()> {
    let paths = crate::util::paths(&cli);
    let cli_repo = cli.repo_path.clone();
    paths.ensure_layout().context("create knowledge layout")?;

    match cli.command {
        Commands::List => knowledge::list(&paths).await,
        Commands::Scan { repo_path, slug } => {
            knowledge::scan(paths, cli_repo, repo_path, slug).await
        }
        Commands::Search {
            query,
            project,
            limit,
        } => knowledge::search(&paths, &query, project, limit),
        Commands::Read { path } => knowledge::read(&paths, &path),
        Commands::Tools { command } => tools::run(&paths, command),
        Commands::Assets { command } => assets::run(&paths, command).await,
        Commands::Env { command } => env::run(cli_repo, command).await,
    }
}
