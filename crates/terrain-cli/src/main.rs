mod cli;
mod commands;
mod util;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    terrain_agent::load_dotenv();
    terrain_core::ensure_bundled_tools_initialized();
    terrain_core::ensure_preset_skills_initialized();
    commands::run(Cli::parse()).await
}
