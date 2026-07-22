use std::fs;

use anyhow::{Context, Result};
use terrain_agent::{
    acp_available, acp_spawn_command, llm_status, load_model_settings, model_settings_from_config,
    resolve_model_config, save_model_settings, ModelSettings,
};
use terrain_core::settings::settings_path;

use crate::cli::SettingsCommands;
use crate::util::print_json;

pub fn run(command: SettingsCommands) -> Result<()> {
    match command {
        SettingsCommands::Get => {
            let settings = load_model_settings()
                .unwrap_or_else(|| model_settings_from_config(&resolve_model_config()));
            print_json(&settings)
        }
        SettingsCommands::Set { file } => {
            let raw = fs::read_to_string(&file)
                .with_context(|| format!("read {}", file.display()))?;
            let settings: ModelSettings = serde_json::from_str(&raw)?;
            save_model_settings(&settings)?;
            let status = llm_status(&resolve_model_config());
            print_json(&serde_json::json!({
                "saved_to": settings_path().display().to_string(),
                "llm": status,
            }))
        }
        SettingsCommands::CheckLlm => {
            let status = llm_status(&resolve_model_config());
            print_json(&status)
        }
        SettingsCommands::CheckAcp => {
            let acp = terrain_agent::resolve_acp_settings();
            print_json(&serde_json::json!({
                "available": acp_available(&acp),
                "spawn_command": acp_spawn_command(&acp),
            }))
        }
    }
}
