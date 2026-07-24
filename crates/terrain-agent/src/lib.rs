mod acp;
mod agent_assets;
mod agent_context;
mod builder;
mod context_generator;
mod chat;
mod compat_tool;
mod litho;
mod model;
mod runtime;
mod sdd;
mod settings;
mod throttle;
mod tool_schema;
mod tool_session_cache;
mod tools;
mod workflows;

pub use acp::{
    acp_args, acp_available, acp_binary, acp_spawn_command, agent_execution_ready,
    build_acp_config, default_agent_arch_acp_skill_dir, default_ask_acp_skill_dir,
    execution_pure_acp, execution_uses_acp, execution_uses_native_llm, resolve_acp_settings,
};
pub use builder::{
    knowledge_paths_from_env, knowledge_root_from_env, AgentConfig, build_agent,
    opencode_available,
};
pub use context_generator::AgentContextGenerator;
pub use agent_assets::{ensure_agent_assets, AgentAssetsEnsureReport};
pub use agent_context::{agent_context_exists, run_agent_context_generation};
pub use chat::ChatEngine;
pub use litho::{prepare_litho_generation, run_litho_generation};
pub use runtime::Runtime;
pub use workflows::{
    ask_knowledge, fallback_search_reply, llm_ready, run_project_initialization, run_quick_refresh,
    run_sdd_phase, LithoProgress, ProgressEvent, ProjectInitProgress, ProjectInitResult,
    SddProgress,
};
pub use model::{
    load_dotenv, llm_status, parse_provider, resolve_model_config, LlmProvider, ModelConfig,
    build_llm,
};
pub use settings::{
    default_profile_for, load_model_settings, model_config_from_settings, model_settings_from_config,
    save_model_settings, AcpSettings, AgentExecution, AskExecution, ModelSettings, ProviderProfile,
    DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
};
pub use terrain_core::{
    validate_repo_path, AgentContextGenerationResult, AskStreamEvent, ChatPhase, ChatReply,
    ChatTokenUsage, ChatToolCallRecord, ChatToolCallStatus, LithoGenerationJob,
    LithoGenerationResult, LlmStatus,
};
pub use terrain_core::settings::{
    DEFAULT_LMSTUDIO_BASE_URL, DEFAULT_LMSTUDIO_MODEL, DEFAULT_OPENAI_BASE_URL, DEFAULT_OPENAI_MODEL,
};
