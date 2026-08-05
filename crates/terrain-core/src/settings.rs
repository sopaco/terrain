use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub const DEFAULT_ACP_BINARY: &str = "opencode";
pub const DEFAULT_ACP_ARGS: &str = "acp";
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:9b";
pub const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";
pub const DEFAULT_OPENAI_MODEL: &str = "stepfun-ai/step-3.7-flash";
pub const DEFAULT_LMSTUDIO_BASE_URL: &str = "http://localhost:1234/v1";
pub const DEFAULT_LMSTUDIO_MODEL: &str = "qwen/qwen3.5-9b";
pub const DEFAULT_LMSTUDIO_API_KEY: &str = "lm-studio";

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecution {
    #[default]
    Acp,
    #[serde(alias = "native")]
    AcpNative,
}

/// Deprecated alias — use [`AgentExecution`].
pub type AskExecution = AgentExecution;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub agent_execution: AgentExecution,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_approve: Option<bool>,
}

impl Default for AcpSettings {
    fn default() -> Self {
        Self {
            binary: None,
            args: None,
            command: None,
            agent_execution: AgentExecution::Acp,
            auto_approve: Some(true),
        }
    }
}

/// Above this many changed source files, an incremental update is no longer cheaper or safer
/// than regenerating from scratch — the diff stops being a summary of the change.
pub const DEFAULT_INCREMENTAL_MAX_CHANGED_FILES: u32 = 60;

/// How knowledge assets are refreshed when they have drifted behind the repository.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeSettings {
    /// Update existing assets from `git diff` instead of regenerating them from scratch.
    #[serde(default = "default_true")]
    pub incremental_refresh: bool,
    /// Fall back to a full regeneration when more source files than this changed.
    #[serde(default = "default_incremental_max_changed_files")]
    pub incremental_max_changed_files: u32,
    /// Also update the human-facing Litho docs during quick refresh (off by default — the
    /// Litho ACP pass is the slowest stage, so quick refresh stays agent-assets-only).
    #[serde(default)]
    pub incremental_human_docs: bool,
}

impl Default for KnowledgeSettings {
    fn default() -> Self {
        Self {
            incremental_refresh: true,
            incremental_max_changed_files: DEFAULT_INCREMENTAL_MAX_CHANGED_FILES,
            incremental_human_docs: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_incremental_max_changed_files() -> u32 {
    DEFAULT_INCREMENTAL_MAX_CHANGED_FILES
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama_host: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelSettings {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub ollama_host: Option<String>,
    #[serde(default)]
    pub profiles: HashMap<String, ProviderProfile>,
    #[serde(default)]
    pub acp: AcpSettings,
    #[serde(default)]
    pub knowledge: KnowledgeSettings,
}

pub fn settings_path() -> PathBuf {
    dirs_home().join(".terrain/settings.json")
}

pub fn load_model_settings() -> Option<ModelSettings> {
    let path = settings_path();
    let raw = fs::read_to_string(path).ok()?;
    let mut settings: ModelSettings = serde_json::from_str(&raw).ok()?;
    normalize_settings(&mut settings);
    Some(settings)
}

pub fn save_model_settings(settings: &ModelSettings) -> Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut to_save = settings.clone();
    normalize_settings(&mut to_save);
    sync_active_flat_fields(&mut to_save);
    let json = serde_json::to_string_pretty(&to_save)?;
    fs::write(&path, json)?;
    Ok(())
}

pub fn default_profile_for(provider: &str) -> ProviderProfile {
    match provider {
        "lmstudio" => ProviderProfile {
            model: Some(DEFAULT_LMSTUDIO_MODEL.into()),
            api_key: Some(DEFAULT_LMSTUDIO_API_KEY.into()),
            base_url: Some(DEFAULT_LMSTUDIO_BASE_URL.into()),
            ollama_host: Some(DEFAULT_OLLAMA_HOST.into()),
        },
        "ollama" => ProviderProfile {
            model: Some(DEFAULT_OLLAMA_MODEL.into()),
            api_key: None,
            base_url: None,
            ollama_host: Some(DEFAULT_OLLAMA_HOST.into()),
        },
        _ => ProviderProfile {
            model: Some(DEFAULT_OPENAI_MODEL.into()),
            api_key: None,
            base_url: Some(DEFAULT_OPENAI_BASE_URL.into()),
            ollama_host: Some(DEFAULT_OLLAMA_HOST.into()),
        },
    }
}

pub fn profile_for_provider(settings: &ModelSettings, provider: &str) -> ProviderProfile {
    settings
        .profiles
        .get(provider)
        .cloned()
        .unwrap_or_else(|| default_profile_for(provider))
}

/// Normalize profiles and active provider fields (same rules as load).
pub fn normalize_model_settings(settings: &mut ModelSettings) {
    normalize_settings(settings);
}

fn normalize_settings(settings: &mut ModelSettings) {
    for p in ["openai", "lmstudio", "ollama"] {
        settings
            .profiles
            .entry(p.into())
            .or_insert_with(|| default_profile_for(p));
    }

    let active = settings.provider.as_deref().unwrap_or("openai").to_string();

    if settings.model.is_some()
        || settings.api_key.is_some()
        || settings.base_url.is_some()
        || settings.ollama_host.is_some()
    {
        let legacy = ProviderProfile {
            model: settings.model.clone(),
            api_key: settings.api_key.clone(),
            base_url: settings.base_url.clone(),
            ollama_host: settings.ollama_host.clone(),
        };
        merge_profile(
            settings.profiles.entry(active.clone()).or_default(),
            &legacy,
        );
    }

    settings.provider = Some(active);

    // A 0 budget would silently disable incremental refresh; treat it as "unset".
    if settings.knowledge.incremental_max_changed_files == 0 {
        settings.knowledge.incremental_max_changed_files = DEFAULT_INCREMENTAL_MAX_CHANGED_FILES;
    }
}

fn merge_profile(target: &mut ProviderProfile, source: &ProviderProfile) {
    if source.model.is_some() {
        target.model = source.model.clone();
    }
    if source.api_key.is_some() {
        target.api_key = source.api_key.clone();
    }
    if source.base_url.is_some() {
        target.base_url = source.base_url.clone();
    }
    if source.ollama_host.is_some() {
        target.ollama_host = source.ollama_host.clone();
    }
}

fn sync_active_flat_fields(settings: &mut ModelSettings) {
    let active = settings.provider.as_deref().unwrap_or("openai");
    let profile = profile_for_provider(settings, active);
    settings.model = profile.model.clone();
    settings.api_key = profile.api_key.clone();
    settings.base_url = profile.base_url.clone();
    settings.ollama_host = profile.ollama_host.clone();
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_execution_defaults_to_acp() {
        assert_eq!(AcpSettings::default().agent_execution, AgentExecution::Acp);
    }

    #[test]
    fn agent_execution_deserializes_legacy_native_as_hybrid() {
        let json = r#"{"agent_execution":"native"}"#;
        let settings: AcpSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.agent_execution, AgentExecution::AcpNative);
    }
}
