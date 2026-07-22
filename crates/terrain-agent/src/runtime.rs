use std::sync::{Arc, Mutex, RwLock};

use terrain_core::KnowledgePaths;

use crate::chat::ChatEngine;
use crate::model::{ModelConfig, resolve_model_config};
use crate::acp::resolve_acp_settings;
use crate::settings::{load_model_settings, AcpSettings};

/// Shared runtime for GUI and CLI — caches [`ChatEngine`] and model settings.
pub struct Runtime {
    pub paths: KnowledgePaths,
    model_config: RwLock<ModelConfig>,
    chat: Mutex<Option<Arc<ChatEngine>>>,
}

impl Runtime {
    pub fn new(paths: KnowledgePaths) -> Self {
        Self {
            paths,
            model_config: RwLock::new(resolve_model_config()),
            chat: Mutex::new(None),
        }
    }

    pub fn model_config(&self) -> ModelConfig {
        self.model_config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_model_config(&self, config: ModelConfig) {
        *self.model_config
            .write()
            .unwrap_or_else(|e| e.into_inner()) = config;
        *self.chat.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn reload_model_config(&self) {
        self.set_model_config(resolve_model_config());
    }

    pub fn acp_settings(&self) -> AcpSettings {
        load_model_settings()
            .map(|s| s.acp)
            .unwrap_or_else(resolve_acp_settings)
    }

    pub fn chat_engine(&self) -> anyhow::Result<Arc<ChatEngine>> {
        let config = self.model_config();
        let acp = self.acp_settings();
        let mut guard = self.chat.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(engine) = guard.as_ref() {
            if engine.model_config() == &config && engine.acp_settings() == &acp {
                return Ok(engine.clone());
            }
            *guard = None;
        }
        let engine = Arc::new(ChatEngine::with_settings(self.paths.clone(), config, acp)?);
        *guard = Some(engine.clone());
        Ok(engine)
    }

    pub fn chat_engine_native(&self) -> anyhow::Result<Arc<ChatEngine>> {
        Ok(Arc::new(ChatEngine::new_native(
            self.paths.clone(),
            self.model_config(),
        )?))
    }
}
