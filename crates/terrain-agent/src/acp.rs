use std::collections::HashMap;
use std::path::PathBuf;

use crate::builder::opencode_available;
use crate::model::{llm_status, ModelConfig};
use crate::settings::{
    load_model_settings, AcpSettings, AgentExecution, DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
};
use terrain_core::{default_agent_arch_skill_dir, default_ask_skill_dir, resolve_executable};

pub fn resolve_acp_settings() -> AcpSettings {
    load_model_settings()
        .map(|s| s.acp)
        .unwrap_or_default()
}

pub fn acp_binary(settings: &AcpSettings) -> String {
    settings
        .binary
        .clone()
        .or_else(|| std::env::var("TERRAIN_ACP_BINARY").ok())
        .unwrap_or_else(|| DEFAULT_ACP_BINARY.into())
}

pub fn acp_args(settings: &AcpSettings) -> String {
    settings
        .args
        .clone()
        .or_else(|| std::env::var("TERRAIN_ACP_ARGS").ok())
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
    if let Ok(cmd) = std::env::var("TERRAIN_ACP_COMMAND")
        && !cmd.trim().is_empty() {
            return cmd.trim().to_string();
        }
    format!("{} {}", acp_binary(settings), acp_args(settings))
}

pub fn acp_available(settings: &AcpSettings) -> bool {
    let cmd = acp_spawn_command(settings);
    let binary = cmd.split_whitespace().next().unwrap_or(DEFAULT_ACP_BINARY);
    opencode_available(binary)
}

pub fn default_ask_acp_skill_dir() -> PathBuf {
    default_ask_skill_dir()
}

pub fn default_agent_arch_acp_skill_dir() -> PathBuf {
    default_agent_arch_skill_dir()
}

/// True when Litho/SDD codegen (and all workloads in pure ACP mode) route through the configured ACP agent.
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

/// Resolve the (binary, args) pair for spawning the ACP agent.
///
/// Priority order matches `acp_spawn_command`:
///   1. Explicit `command` string (split on whitespace — avoid backslash issues by
///      preferring `binary` + `args` fields for Windows paths with spaces).
///   2. `TERRAIN_ACP_COMMAND` env var.
///   3. `binary` field (or `TERRAIN_ACP_BINARY`) + `args` field (or `TERRAIN_ACP_ARGS`).
///
/// We deliberately avoid `shell_words::split` because it treats `\` as a POSIX escape
/// character (corrupting Windows paths like `C:\Users\...`) and interprets `;` in
/// `PATH` as a command separator.
pub(crate) fn acp_command_parts(settings: &AcpSettings) -> (String, Vec<String>) {
    let split_cmd = |cmd: &str| -> (String, Vec<String>) {
        let trimmed = cmd.trim();
        let mut parts = trimmed.split_whitespace().map(str::to_string);
        let bin = parts.next().unwrap_or_else(|| DEFAULT_ACP_BINARY.into());
        let args: Vec<String> = parts.collect();
        (bin, args)
    };

    let (raw_binary, raw_args) =
        if let Some(cmd) = settings.command.as_ref().filter(|c| !c.trim().is_empty()) {
            split_cmd(cmd)
        } else if let Ok(cmd) = std::env::var("TERRAIN_ACP_COMMAND") {
            if !cmd.trim().is_empty() {
                split_cmd(&cmd)
            } else {
                let bin = acp_binary(settings);
                let args: Vec<String> = acp_args(settings)
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                (bin, args)
            }
        } else {
            let bin = acp_binary(settings);
            let args: Vec<String> = acp_args(settings)
                .split_whitespace()
                .map(str::to_string)
                .collect();
            (bin, args)
        };

    let resolved_binary = resolve_executable(&raw_binary)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or(raw_binary);
    (resolved_binary, raw_args)
}

/// Serialize an ACP stdio configuration as JSON for the adk-acp SDK.
///
/// The SDK's `AcpAgent::from_str` accepts either a shell-command string or a JSON
/// object. On Windows the shell-command path is broken because:
///
/// 1. `shell_words::split` treats `\` as a POSIX escape, stripping it from paths.
/// 2. `;` in `PATH` (and other env values) is interpreted as a shell command
///    separator, truncating the command.
///
/// Using the JSON form bypasses shell parsing entirely; `tokio::process::Command`
/// receives the binary path and args verbatim.
pub(crate) fn acp_config_json(
    binary: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> String {
    let name = std::path::Path::new(binary)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("acp-agent")
        .to_string();

    let env_arr: Vec<serde_json::Value> = env
        .iter()
        .map(|(k, v)| serde_json::json!({ "name": k, "value": v }))
        .collect();

    let config = serde_json::json!({
        "type": "stdio",
        "name": name,
        "command": binary,
        "args": args,
        "env": env_arr,
    });

    serde_json::to_string(&config).expect("ACP config JSON serialization should not fail")
}

#[cfg(feature = "opencode")]
pub fn build_acp_config(
    settings: &AcpSettings,
    working_dir: Option<&str>,
    extra_env: HashMap<String, String>,
) -> adk_acp::AcpAgentConfig {
    use adk_acp::AcpAgentConfig;

    let (binary, args) = acp_command_parts(settings);

    let mut env = HashMap::new();

    if let Ok(path) = std::env::var("PATH") {
        env.insert("PATH".to_string(), path);
    }
    for (k, v) in extra_env {
        env.insert(k, v);
    }

    let json = acp_config_json(&binary, &args, &env);

    let mut config = AcpAgentConfig::new(json);
    if settings.auto_approve.unwrap_or(true) {
        config = config.auto_approve(true);
    }
    if let Some(cwd) = working_dir.filter(|p| !p.is_empty()) {
        config = config.working_dir(cwd);
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
