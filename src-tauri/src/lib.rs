mod commands;

use mind_mesh_agent::{ChatEngine, ModelConfig, load_model_settings, resolve_acp_settings, resolve_model_config};
use mind_mesh_core::KnowledgePaths;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

pub struct AppState {
    pub paths: KnowledgePaths,
    model_config: RwLock<ModelConfig>,
    pub chat: Mutex<Option<Arc<ChatEngine>>>,
}

impl AppState {
    pub fn new(paths: KnowledgePaths) -> Self {
        Self {
            paths,
            model_config: RwLock::new(resolve_model_config()),
            chat: Mutex::new(None),
        }
    }

    pub fn get_model_config(&self) -> ModelConfig {
        self.model_config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_model_config(&self, config: ModelConfig) {
        *self.model_config
            .write()
            .unwrap_or_else(|e| e.into_inner()) = config;
    }

    pub async fn chat_engine(&self) -> Result<Arc<ChatEngine>, String> {
        let config = self.get_model_config();
        let acp = load_model_settings()
            .map(|s| s.acp)
            .unwrap_or_else(resolve_acp_settings);
        let mut guard = self.chat.lock().await;
        if let Some(engine) = guard.as_ref() {
            if engine.model_config() == &config && engine.acp_settings() == &acp {
                return Ok(engine.clone());
            }
            *guard = None;
        }
        let engine = Arc::new(
            ChatEngine::with_settings(self.paths.clone(), config, acp)
                .map_err(|e| e.to_string())?,
        );
        *guard = Some(engine.clone());
        Ok(engine)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    mind_mesh_agent::load_dotenv();

    tracing_subscriber::fmt()
        .with_env_filter("info,mind_mesh=debug,mind_mesh_core=debug")
        .init();

    let paths = commands::init_paths();
    let state = AppState::new(paths);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_knowledge_root,
            commands::list_projects,
            commands::scan_project,
            commands::search_knowledge,
            commands::read_document,
            commands::check_acp,
            commands::check_opencode,
            commands::acp_spawn_command_cmd,
            commands::check_llm,
            commands::get_model_settings,
            commands::save_model_settings_cmd,
            commands::copy_image_to_clipboard,
            commands::copy_text_to_clipboard,
            commands::pack_agent_assets_cmd,
            commands::plan_litho_cmd,
            commands::plan_assets_cmd,
            commands::list_human_docs_cmd,
            commands::read_source_slice_cmd,
            commands::resolve_source_citation_cmd,
            commands::open_repo_folder_cmd,
            commands::generate_human_docs_cmd,
            commands::run_litho_generation_cmd,
            commands::get_project_overview_cmd,
            commands::get_sdd_status_cmd,
            commands::initialize_project_cmd,
            commands::list_stale_projects_cmd,
            commands::run_sdd_phase_cmd,
            commands::run_agent_context_generation_cmd,
            commands::ask_knowledge_cmd,
            commands::get_env_status_cmd,
            commands::plan_env_integration_cmd,
            commands::run_env_integration_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MindMesh");
}
