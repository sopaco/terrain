mod assets;
mod ask;
mod env;
mod init;
mod knowledge;
mod project;
mod settings;
mod sdd;
mod source;
mod tools;
mod usage;

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
        Commands::Init { repo_path, slug } => init::run(paths, cli_repo, repo_path, slug).await,
        Commands::Refresh { repo_path, slug } => {
            init::refresh(paths, cli_repo, repo_path, slug).await
        }
        Commands::Search {
            query,
            project,
            limit,
        } => knowledge::search(&paths, &query, project, limit),
        Commands::Read { path } => knowledge::read(&paths, &path),
        Commands::Project { command } => project::run(&paths, command),
        Commands::Settings { command } => settings::run(command),
        Commands::Ask { command } => ask::run(&paths, cli_repo, command).await,
        Commands::Sdd { command } => sdd::run(&paths, cli_repo, command).await,
        Commands::Usage { command } => usage::run(command),
        Commands::Source { command } => source::run(cli_repo, command),
        Commands::Tools { command } => tools::run(&paths, command),
        Commands::Assets { command } => assets::run(&paths, command).await,
        Commands::Env { command } => env::run(cli_repo, command).await,
    }
}
