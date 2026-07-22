mod bundled_tools;
mod commands;
mod preset_skills;
mod tray;

use terrain_agent::{ModelConfig, Runtime};
use terrain_core::KnowledgePaths;

pub struct AppState {
    pub runtime: Runtime,
}

impl AppState {
    pub fn new(paths: KnowledgePaths) -> Self {
        Self {
            runtime: Runtime::new(paths),
        }
    }

    pub fn paths(&self) -> &KnowledgePaths {
        &self.runtime.paths
    }

    pub fn model_config(&self) -> ModelConfig {
        self.runtime.model_config()
    }

    pub fn set_model_config(&self, config: ModelConfig) {
        self.runtime.set_model_config(config);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    terrain_agent::load_dotenv();

    tracing_subscriber::fmt()
        .with_env_filter("info,terrain=debug,terrain_core=debug")
        .init();

    let paths = commands::init_paths();
    let state = AppState::new(paths);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            preset_skills::init_app_preset_skills(app.handle());
            bundled_tools::init_app_bundled_tools(app.handle());
            tray::init(app)?;
            Ok(())
        })
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
            commands::compute_freshness_cmd,
            commands::read_project_freshness_cached_cmd,
            commands::run_quick_refresh_cmd,
            commands::plan_litho_cmd,
            commands::plan_assets_cmd,
            commands::list_human_docs_cmd,
            commands::read_source_slice_cmd,
            commands::resolve_source_citation_cmd,
            commands::open_repo_folder_cmd,
            commands::generate_human_docs_cmd,
            commands::run_litho_generation_cmd,
            commands::get_project_overview_cmd,
            commands::save_project_remark_cmd,
            commands::get_sdd_status_cmd,
            commands::create_sdd_session_cmd,
            commands::set_active_sdd_session_cmd,
            commands::delete_sdd_session_cmd,
            commands::save_sdd_output_cmd,
            commands::remove_project_cmd,
            commands::initialize_project_cmd,
            commands::list_stale_projects_cmd,
            commands::run_sdd_phase_cmd,
            commands::run_agent_context_generation_cmd,
            commands::ask_knowledge_cmd,
            commands::list_ask_sessions_cmd,
            commands::load_ask_messages_cmd,
            commands::save_ask_messages_cmd,
            commands::create_ask_session_cmd,
            commands::set_active_ask_session_cmd,
            commands::delete_ask_session_cmd,
            commands::discard_ask_session_cmd,
            commands::clear_active_ask_session_cmd,
            commands::get_active_ask_session_cmd,
            commands::get_env_status_cmd,
            commands::plan_env_integration_cmd,
            commands::run_env_integration_cmd,
            commands::usage_probe_cmd,
            commands::usage_snapshot_cmd,
        ])
        .build(tauri::generate_context!())
        .expect("error while running Terrain")
        .run(|app_handle, event| {
            tray::handle_run_event(app_handle, &event);
        });
}
