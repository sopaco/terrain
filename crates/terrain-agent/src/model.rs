use std::sync::Arc;

use adk_core::Llm;
use adk_model::ollama::{OllamaConfig, OllamaModel};
use adk_model::openai::{OpenAIClient, OpenAIConfig};
use anyhow::{Context, Result, bail};
pub use terrain_core::LlmStatus;
pub use terrain_core::settings::{
    DEFAULT_LMSTUDIO_API_KEY, DEFAULT_LMSTUDIO_BASE_URL,
    DEFAULT_OLLAMA_HOST, DEFAULT_OPENAI_BASE_URL, DEFAULT_OPENAI_MODEL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    Ollama,
    Openai,
    LmStudio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelConfig {
    pub provider: LlmProvider,
    pub model: String,
    pub ollama_host: String,
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
}

use crate::settings::model_config_from_settings;
use terrain_core::settings::load_model_settings;

pub fn parse_provider(raw: &str) -> LlmProvider {
    match raw.to_lowercase().as_str() {
        "ollama" => LlmProvider::Ollama,
        "lmstudio" | "lm-studio" | "lm_studio" => LlmProvider::LmStudio,
        "openai" | "openai-compatible" | "nvidia" => LlmProvider::Openai,
        _ => LlmProvider::Openai,
    }
}

/// `OPENAI_API_KEY` or `TERRAIN_API_KEY`.
fn read_api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY")
        .ok()
        .or_else(|| std::env::var("TERRAIN_API_KEY").ok())
        .filter(|k| !k.is_empty())
}

/// Load effective LLM config.
///
/// When `~/.terrain/settings.json` exists, UI-saved provider/profile values win.
/// Environment variables only apply as fallbacks when no settings file exists, or to
/// supply a missing OpenAI API key for the `openai` provider.
pub fn resolve_model_config() -> ModelConfig {
    if let Some(settings) = load_model_settings() {
        let mut config = model_config_from_settings(&settings);
        apply_env_secret_fallback(&mut config);
        return finalize_config(config);
    }

    let mut config = ModelConfig {
        provider: LlmProvider::Openai,
        model: DEFAULT_OPENAI_MODEL.into(),
        ollama_host: DEFAULT_OLLAMA_HOST.into(),
        openai_api_key: None,
        openai_base_url: None,
    };
    apply_env_overrides(&mut config);
    finalize_config(config)
}

fn apply_env_overrides(config: &mut ModelConfig) {
    if let Ok(p) = std::env::var("TERRAIN_LLM_PROVIDER") {
        config.provider = parse_provider(&p);
    }
    if let Ok(m) = std::env::var("TERRAIN_MODEL") {
        config.model = m;
    }
    if let Ok(h) = std::env::var("OLLAMA_HOST") {
        config.ollama_host = h;
    }
    if let Ok(url) = std::env::var("OPENAI_BASE_URL") {
        config.openai_base_url = Some(url);
    }
    if let Some(key) = read_api_key() {
        config.openai_api_key = Some(key);
    }
}

fn apply_env_secret_fallback(config: &mut ModelConfig) {
    if config.provider != LlmProvider::Openai {
        return;
    }
    if config.openai_api_key.as_ref().is_some_and(|k| !k.is_empty()) {
        return;
    }
    if let Some(key) = read_api_key() {
        config.openai_api_key = Some(key);
    }
}

fn finalize_config(mut config: ModelConfig) -> ModelConfig {
    if config.openai_base_url.is_none() {
        config.openai_base_url = match config.provider {
            LlmProvider::Openai => Some(DEFAULT_OPENAI_BASE_URL.into()),
            LlmProvider::LmStudio => Some(DEFAULT_LMSTUDIO_BASE_URL.into()),
            LlmProvider::Ollama => None,
        };
    }

    if config.provider == LlmProvider::LmStudio
        && config.openai_api_key.as_ref().is_none_or(|k| k.is_empty())
    {
        config.openai_api_key = Some(DEFAULT_LMSTUDIO_API_KEY.into());
    }

    config
}

pub fn build_llm(config: &ModelConfig) -> Result<Arc<dyn Llm>> {
    let inner = match config.provider {
        LlmProvider::Ollama => {
            let mut ollama_cfg = OllamaConfig::with_host(&config.ollama_host, &config.model);
            if let Ok(ctx) = std::env::var("TERRAIN_OLLAMA_CTX") {
                ollama_cfg.num_ctx = ctx.parse().ok();
            }
            let model = OllamaModel::new(ollama_cfg).context("failed to create Ollama model")?;
            Arc::new(model) as Arc<dyn Llm>
        }
        LlmProvider::Openai | LlmProvider::LmStudio => build_openai_compatible(config)?,
    };
    Ok(crate::throttle::wrap_llm(inner, crate::throttle::call_cooldown_from_env()))
}

fn build_openai_compatible(config: &ModelConfig) -> Result<Arc<dyn Llm>> {
    let api_key = config
        .openai_api_key
        .clone()
        .filter(|k| !k.is_empty())
        .context("OPENAI_API_KEY or TERRAIN_API_KEY is required")?;
    let base_url = config
        .openai_base_url
        .clone()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| match config.provider {
            LlmProvider::LmStudio => DEFAULT_LMSTUDIO_BASE_URL.into(),
            _ => DEFAULT_OPENAI_BASE_URL.into(),
        });
    let openai_cfg = OpenAIConfig::compatible(api_key, base_url, &config.model);
    let model = OpenAIClient::new(openai_cfg).map_err(|e| anyhow::anyhow!("openai model: {e}"))?;
    Ok(Arc::new(model))
}

pub fn llm_status(config: &ModelConfig) -> LlmStatus {
    let provider = match config.provider {
        LlmProvider::Ollama => "ollama",
        LlmProvider::Openai => "openai-compatible",
        LlmProvider::LmStudio => "lmstudio",
    };

    let (ready, message) = match config.provider {
        LlmProvider::Ollama => (
            true,
            format!("Ollama @ {} (model: {})", config.ollama_host, config.model),
        ),
        LlmProvider::Openai => {
            let ok = config
                .openai_api_key
                .as_ref()
                .is_some_and(|k| !k.is_empty());
            let base = config
                .openai_base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.into());
            (
                ok,
                if ok {
                    format!("{base} (model: {})", config.model)
                } else {
                    "Set OPENAI_API_KEY or TERRAIN_API_KEY".into()
                },
            )
        }
        LlmProvider::LmStudio => {
            let base = config
                .openai_base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_LMSTUDIO_BASE_URL.into());
            (
                true,
                format!("LM Studio @ {base} (model: {})", config.model),
            )
        }
    };

    LlmStatus {
        provider: provider.into(),
        model: config.model.clone(),
        ready,
        message,
        base_url: config.openai_base_url.clone(),
    }
}

pub fn ensure_llm(config: &ModelConfig) -> Result<()> {
    let status = llm_status(config);
    if !status.ready {
        bail!("LLM not ready: {}", status.message);
    }
    Ok(())
}

pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

#[cfg(test)]
mod tests {
    use super::*;
    use terrain_core::settings::{ModelSettings, ProviderProfile, save_model_settings};
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn settings_file_wins_over_env_provider_and_base_url() {
        let _guard = env_lock().lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().to_path_buf();

        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("TERRAIN_LLM_PROVIDER", "openai");
            std::env::set_var("OPENAI_BASE_URL", "https://integrate.api.nvidia.com/v1");
            std::env::set_var("TERRAIN_MODEL", "stepfun-ai/step-3.7-flash");
        }

        let mut profiles = HashMap::new();
        profiles.insert(
            "lmstudio".into(),
            ProviderProfile {
                model: Some("qwen/qwen3.5-9b".into()),
                api_key: Some("lm-studio".into()),
                base_url: Some("http://localhost:1234/v1".into()),
                ollama_host: None,
            },
        );
        save_model_settings(&ModelSettings {
            provider: Some("lmstudio".into()),
            model: Some("qwen/qwen3.5-9b".into()),
            api_key: Some("lm-studio".into()),
            base_url: Some("http://localhost:1234/v1".into()),
            ollama_host: None,
            profiles,
            acp: Default::default(),
        })
        .unwrap();

        let config = resolve_model_config();
        assert_eq!(config.provider, LlmProvider::LmStudio);
        assert_eq!(
            config.openai_base_url.as_deref(),
            Some("http://localhost:1234/v1")
        );
        assert_eq!(config.model, "qwen/qwen3.5-9b");

        unsafe {
            std::env::remove_var("TERRAIN_LLM_PROVIDER");
            std::env::remove_var("OPENAI_BASE_URL");
            std::env::remove_var("TERRAIN_MODEL");
        }
    }
}
