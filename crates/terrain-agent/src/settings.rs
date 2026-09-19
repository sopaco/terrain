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
        api_mode: config.openai_api_mode,
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
        // Preserve the persisted language preference when rewriting settings.
        language: load_model_settings()
            .map(|s| s.language)
            .unwrap_or_default(),
    }
}

pub fn model_config_from_settings(settings: &ModelSettings) -> ModelConfig {
    let mut normalized = settings.clone();
    normalize_model_settings(&mut normalized);

    let raw_provider = normalized
        .provider
        .clone()
        .unwrap_or_else(|| "openai".into());
    let provider = parse_provider(&raw_provider);

    // Profiles are stored under the raw provider id (e.g. "ollama-cloud"); aliased
    // ids that map to the same LlmProvider fall back to the canonical profile name.
    let profile_key = if normalized.profiles.contains_key(&raw_provider) {
        raw_provider
    } else {
        provider_name(provider).to_string()
    };

    let profile = profile_for_provider(&normalized, &profile_key);

    let model = profile
        .model
        .clone()
        .unwrap_or_else(|| default_profile_for(&profile_key).model.unwrap());

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
        openai_api_mode: profile.api_mode,
    }
}

pub fn provider_name(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::Ollama => "ollama",
        LlmProvider::Openai => "openai",
        LlmProvider::LmStudio => "lmstudio",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LlmProvider;

    #[test]
    fn ollama_cloud_profile_routes_to_openai_compatible() {
        let mut settings = ModelSettings::default();
        settings.provider = Some("ollama-cloud".into());
        settings.profiles.insert(
            "ollama-cloud".into(),
            ProviderProfile {
                model: Some("gpt-oss:120b-cloud".into()),
                api_key: Some("test-key".into()),
                base_url: Some("https://ollama.com/v1".into()),
                ..Default::default()
            },
        );

        let config = model_config_from_settings(&settings);
        assert_eq!(config.provider, LlmProvider::Openai);
        assert_eq!(
            config.openai_base_url.as_deref(),
            Some("https://ollama.com/v1")
        );
        assert_eq!(config.openai_api_key.as_deref(), Some("test-key"));
        assert_eq!(config.model, "gpt-oss:120b-cloud");
    }

    #[test]
    fn ollama_cloud_falls_back_to_defaults_without_profile() {
        let mut settings = ModelSettings::default();
        settings.provider = Some("ollama-cloud".into());

        let config = model_config_from_settings(&settings);
        assert_eq!(config.provider, LlmProvider::Openai);
        assert_eq!(
            config.openai_base_url.as_deref(),
            Some(crate::settings::DEFAULT_OLLAMA_CLOUD_BASE_URL)
        );
        assert_eq!(
            config.model,
            crate::settings::DEFAULT_OLLAMA_CLOUD_MODEL
        );
    }
}
