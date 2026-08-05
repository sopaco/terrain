//! Model settings persistence lives in [`terrain_core::settings`].
//! This module adds LLM config conversion for `terrain-agent`.

pub use terrain_core::settings::*;

use crate::model::{LlmProvider, ModelConfig, parse_provider};

/// Knowledge-refresh preferences from `~/.terrain/settings.json`, or defaults when absent.
pub fn resolve_knowledge_settings() -> KnowledgeSettings {
    load_model_settings()
        .map(|s| s.knowledge)
        .unwrap_or_default()
}

pub fn model_settings_from_config(config: &ModelConfig) -> ModelSettings {
    let provider = provider_name(config.provider).to_string();
    let profile = ProviderProfile {
        model: Some(config.model.clone()),
        api_key: config.openai_api_key.clone(),
        base_url: config.openai_base_url.clone(),
        ollama_host: Some(config.ollama_host.clone()),
    };

    let mut profiles = std::collections::HashMap::new();
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
        knowledge: KnowledgeSettings::default(),
    }
}

pub fn model_config_from_settings(settings: &ModelSettings) -> ModelConfig {
    let mut normalized = settings.clone();
    normalize_model_settings(&mut normalized);

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
