use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{
    DEFAULT_LMSTUDIO_API_KEY, DEFAULT_LMSTUDIO_BASE_URL, DEFAULT_LMSTUDIO_MODEL,
    DEFAULT_OPENAI_BASE_URL, DEFAULT_OPENAI_MODEL, LlmProvider, ModelConfig, parse_provider,
};

pub const DEFAULT_ACP_BINARY: &str = "opencode";
pub const DEFAULT_ACP_ARGS: &str = "acp";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecution {
    #[default]
    Native,
    Acp,
}

/// Deprecated alias — use [`AgentExecution`].
pub type AskExecution = AgentExecution;

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
            agent_execution: AgentExecution::Native,
            auto_approve: Some(true),
        }
    }
}

pub const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:9b";
pub const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";

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
}

pub fn settings_path() -> PathBuf {
    dirs_home().join(".mind-mesh/settings.json")
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
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut to_save = settings.clone();
    normalize_settings(&mut to_save);
    sync_active_flat_fields(&mut to_save);
    let json = serde_json::to_string_pretty(&to_save)?;
    fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn model_settings_from_config(config: &ModelConfig) -> ModelSettings {
    let provider = provider_name(config.provider).to_string();
    let profile = ProviderProfile {
        model: Some(config.model.clone()),
        api_key: config.openai_api_key.clone(),
        base_url: config.openai_base_url.clone(),
        ollama_host: Some(config.ollama_host.clone()),
    };

    let mut profiles = HashMap::new();
    for p in ["openai", "lmstudio", "ollama"] {
        profiles.insert(p.into(), default_profile_for(p));
    }
    profiles.insert(provider.clone(), profile.clone());

    ModelSettings {
        provider: Some(provider.clone()),
        model: profile.model.clone(),
        api_key: profile.api_key.clone(),
        base_url: profile.base_url.clone(),
        ollama_host: profile.ollama_host.clone(),
        profiles,
        acp: AcpSettings::default(),
    }
}

pub fn model_config_from_settings(settings: &ModelSettings) -> ModelConfig {
    let mut normalized = settings.clone();
    normalize_settings(&mut normalized);

    let provider = normalized
        .provider
        .as_deref()
        .map(parse_provider)
        .unwrap_or(LlmProvider::Openai);

    let profile = profile_for_provider(&normalized, provider_name(provider));

    let model = profile
        .model
        .clone()
        .unwrap_or_else(|| default_profile_for(provider_name(provider)).model.unwrap());

    let ollama_host = profile
        .ollama_host
        .clone()
        .unwrap_or_else(|| DEFAULT_OLLAMA_HOST.into());

    let openai_base_url = profile.base_url.clone().or_else(|| match provider {
        LlmProvider::Openai => Some(DEFAULT_OPENAI_BASE_URL.into()),
        LlmProvider::LmStudio => Some(DEFAULT_LMSTUDIO_BASE_URL.into()),
        LlmProvider::Ollama => None,
    });

    let mut openai_api_key = profile.api_key.clone();
    if provider == LlmProvider::LmStudio && openai_api_key.as_ref().is_none_or(|k| k.is_empty()) {
        openai_api_key = Some(DEFAULT_LMSTUDIO_API_KEY.into());
    }

    ModelConfig {
        provider,
        model,
        ollama_host,
        openai_api_key,
        openai_base_url,
    }
}

pub fn provider_name(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::Ollama => "ollama",
        LlmProvider::Openai => "openai",
        LlmProvider::LmStudio => "lmstudio",
    }
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

fn profile_for_provider(settings: &ModelSettings, provider: &str) -> ProviderProfile {
    settings
        .profiles
        .get(provider)
        .cloned()
        .unwrap_or_else(|| default_profile_for(provider))
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
