use std::collections::HashMap;
use std::path::PathBuf;

use crate::builder::opencode_available;
use crate::settings::{
    load_model_settings, AcpSettings, DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
};

pub fn resolve_acp_settings() -> AcpSettings {
    load_model_settings()
        .map(|s| s.acp)
        .unwrap_or_default()
}

pub fn acp_binary(settings: &AcpSettings) -> String {
    settings
        .binary
        .clone()
        .or_else(|| std::env::var("MIND_MESH_ACP_BINARY").ok())
        .unwrap_or_else(|| DEFAULT_ACP_BINARY.into())
}

pub fn acp_args(settings: &AcpSettings) -> String {
    settings
        .args
        .clone()
        .or_else(|| std::env::var("MIND_MESH_ACP_ARGS").ok())
        .unwrap_or_else(|| DEFAULT_ACP_ARGS.into())
}

pub fn acp_spawn_command(settings: &AcpSettings) -> String {
    if let Some(cmd) = settings
        .command
        .as_ref()
        .filter(|c| !c.trim().is_empty())
    {
        return cmd.trim().to_string();
    }
    if let Ok(cmd) = std::env::var("MIND_MESH_ACP_COMMAND") {
        if !cmd.trim().is_empty() {
            return cmd.trim().to_string();
        }
    }
    format!("{} {}", acp_binary(settings), acp_args(settings))
}

pub fn acp_available(settings: &AcpSettings) -> bool {
    let cmd = acp_spawn_command(settings);
    let binary = cmd.split_whitespace().next().unwrap_or(DEFAULT_ACP_BINARY);
    opencode_available(binary)
}

pub fn default_ask_acp_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MIND_MESH_ASK_SKILL") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../preset_skills/mind-mesh-ask-skill")
}

#[cfg(feature = "opencode")]
pub fn build_acp_config(
    settings: &AcpSettings,
    working_dir: Option<&str>,
    extra_env: HashMap<String, String>,
) -> adk_acp::AcpAgentConfig {
    use adk_acp::AcpAgentConfig;

    let mut config = AcpAgentConfig::new(acp_spawn_command(settings));
    if settings.auto_approve.unwrap_or(true) {
        config = config.auto_approve(true);
    }
    if let Some(cwd) = working_dir.filter(|p| !p.is_empty()) {
        config = config.working_dir(cwd);
    }
    for (k, v) in extra_env {
        config = config.env(k, v);
    }
    config
}

#[cfg(not(feature = "opencode"))]
pub fn build_acp_config(
    _settings: &AcpSettings,
    _working_dir: Option<&str>,
    _extra_env: HashMap<String, String>,
) -> ! {
    panic!("ACP support not enabled (rebuild with opencode feature)")
}
