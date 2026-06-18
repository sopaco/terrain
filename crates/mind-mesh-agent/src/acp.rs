use std::collections::HashMap;
use std::path::PathBuf;

use crate::builder::opencode_available;
use crate::model::{llm_status, ModelConfig};
use crate::settings::{
    load_model_settings, AcpSettings, AgentExecution, DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
};
use mind_mesh_core::{default_agent_arch_skill_dir, resolve_command};

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

pub fn default_agent_arch_acp_skill_dir() -> PathBuf {
    default_agent_arch_skill_dir()
}

/// True when Ask/Litho/SDD codegen should route through the configured ACP agent.
pub fn execution_uses_acp(settings: &AcpSettings) -> bool {
    matches!(
        settings.agent_execution,
        AgentExecution::Acp | AgentExecution::AcpNative
    )
}

/// Pure ACP — no native LLM workloads (all generation via external agent).
pub fn execution_pure_acp(settings: &AcpSettings) -> bool {
    settings.agent_execution == AgentExecution::Acp
}

/// Hybrid mode — native LLM supplements ACP for SDD doc phases and agent context.
pub fn execution_uses_native_llm(settings: &AcpSettings) -> bool {
    settings.agent_execution == AgentExecution::AcpNative
}

/// Whether the active execution mode has its backends configured.
pub fn agent_execution_ready(settings: &AcpSettings, config: &ModelConfig) -> Result<(), String> {
    let acp_err = if acp_available(settings) {
        None
    } else {
        Some(format!(
            "ACP agent not found on PATH: {}",
            acp_spawn_command(settings)
        ))
    };

    if execution_pure_acp(settings) {
        return acp_err.map_or(Ok(()), Err);
    }

    let llm_err = if llm_status(config).ready {
        None
    } else {
        Some(format!("LLM not ready: {}", llm_status(config).message))
    };

    match (acp_err, llm_err) {
        (None, None) => Ok(()),
        (Some(e), _) => Err(e),
        (_, Some(e)) => Err(e),
    }
}

#[cfg(feature = "opencode")]
pub fn build_acp_config(
    settings: &AcpSettings,
    working_dir: Option<&str>,
    extra_env: HashMap<String, String>,
) -> adk_acp::AcpAgentConfig {
    use adk_acp::AcpAgentConfig;

    let mut config = AcpAgentConfig::new(resolve_command(&acp_spawn_command(settings)));
    if settings.auto_approve.unwrap_or(true) {
        config = config.auto_approve(true);
    }
    if let Some(cwd) = working_dir.filter(|p| !p.is_empty()) {
        config = config.working_dir(cwd);
    }
    if let Ok(path) = std::env::var("PATH") {
        config = config.env("PATH", path);
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
