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
    terrain_core::ensure_env_catalog_initialized();
    terrain_core::ensure_preset_skills_initialized();
    std::thread::spawn(|| {
        let _ = terrain_core::deploy_env_catalog_to_home();
    });
    commands::run(Cli::parse()).await
}
