mod acp;
mod agent_assets;
mod agent_context;
mod builder;
mod context_generator;
mod chat;
mod compat_tool;
mod env_optimize;
mod litho;
mod model;
mod project_init;
mod sdd;
mod settings;
mod throttle;
mod tool_schema;
mod tool_session_cache;
mod tools;

pub use acp::{
    acp_args, acp_available, acp_binary, acp_spawn_command, agent_execution_ready,
    build_acp_config, default_agent_arch_acp_skill_dir, default_ask_acp_skill_dir,
    execution_uses_acp, resolve_acp_settings,
};
pub use builder::{
    knowledge_paths_from_env, knowledge_root_from_env, AgentConfig, build_agent,
    opencode_available, validate_repo_path,
};
pub use context_generator::AgentContextGenerator;
pub use agent_assets::{ensure_agent_assets, AgentAssetsEnsureReport};
pub use agent_context::{
    agent_context_exists, run_agent_context_generation, AgentContextGenerationResult,
};
pub use chat::{ChatEngine, ChatPhase, ChatReply, ChatTokenUsage, ChatToolCallRecord, ChatToolCallStatus};
pub use env_optimize::{env_plan_for_repo, env_status_for_repo, run_env_integration};
pub use litho::{
    prepare_litho_generation, run_litho_generation, LithoGenerationJob, LithoGenerationResult,
    LithoProgress,
};
pub use project_init::{
    run_project_initialization, ProjectInitProgress, ProjectInitResult,
};
pub use sdd::{run_sdd_phase, SddProgress};
pub use model::{
    load_dotenv, llm_status, parse_provider, resolve_model_config, LlmProvider, LlmStatus,
    ModelConfig, build_llm, DEFAULT_LMSTUDIO_BASE_URL, DEFAULT_LMSTUDIO_MODEL,
    DEFAULT_OPENAI_BASE_URL, DEFAULT_OPENAI_MODEL,
};
pub use settings::{
    default_profile_for, load_model_settings, model_config_from_settings, model_settings_from_config,
    save_model_settings, AcpSettings, AgentExecution, AskExecution, ModelSettings, ProviderProfile,
    DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY,
};
