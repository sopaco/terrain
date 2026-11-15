mod acp;
mod agent_assets;
mod agent_context;
mod builder;
mod chat;
mod compat_tool;
mod context_generator;
mod litho;
mod litho_native;
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
pub use agent_assets::{AgentAssetsEnsureReport, ensure_agent_assets};
pub use agent_context::{agent_context_exists, run_agent_context_generation};
pub use builder::{
    AgentConfig, build_agent, knowledge_paths_from_env, knowledge_root_from_env, opencode_available,
};
pub use chat::ChatEngine;
pub use context_generator::AgentContextGenerator;
pub use litho::{LithoRunMode, prepare_litho_generation, run_litho_generation};
pub use model::{
    LlmProvider, ModelConfig, build_llm, list_provider_models, llm_status, load_dotenv,
    parse_provider, probe_llm, resolve_model_config,
};
pub use runtime::Runtime;
pub use settings::{
    AcpSettings, AgentExecution, AskExecution, DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
    KnowledgeSettings, ModelSettings, ProviderProfile, default_profile_for, load_model_settings,
    model_config_from_settings, model_settings_from_config, resolve_knowledge_settings,
    save_model_settings,
};
pub use terrain_core::settings::{
    DEFAULT_LMSTUDIO_BASE_URL, DEFAULT_LMSTUDIO_MODEL, DEFAULT_OPENAI_BASE_URL,
    DEFAULT_OPENAI_MODEL,
};
pub use terrain_core::{
    AgentContextGenerationResult, AskStreamEvent, ChatPhase, ChatReply, ChatTokenUsage,
    ChatToolCallRecord, ChatToolCallStatus, KnowledgeRefreshMode, LithoGenerationJob,
    LithoGenerationResult, LlmStatus, validate_repo_path,
};
pub use workflows::{
    LithoProgress, ProgressEvent, ProjectInitProgress, ProjectInitResult, SddProgress,
    ask_knowledge, fallback_search_reply, llm_ready, run_project_initialization, run_quick_refresh,
    run_sdd_phase,
};
