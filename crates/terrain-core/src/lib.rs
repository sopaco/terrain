pub mod assets;
pub mod citations;
pub mod doc;
pub mod error;
pub mod freshness;
pub mod git_policy;
pub mod human;
pub mod ingest;
pub mod integrations;
pub mod ipc;
pub mod model_text;
pub mod path_portable;
pub mod platform;
pub mod preset_skills;
pub mod process;
pub mod paths;
pub mod progress;
pub mod project;
pub mod prompts;
pub mod registry;
pub mod repo;
pub mod repo_walk;
pub mod render;
pub mod schema;
pub mod search;
pub mod sessions;
pub mod settings;
pub mod agent_tools_deploy;
pub mod bundled_tools;
pub mod shell_path;
pub mod source;
pub mod usage;
#[macro_use]
mod ts_ipc;

pub use assets::{
    agent_context_fresh, agent_context_ready, agent_context_synced_with_head,
    build_agent_context_prompt, build_context_overview,
    build_generation_plan, collect_knowledge_dir_inputs, collect_project_meta,
    count_knowledge_markdown_files, count_markdown_in_dir, default_agent_arch_skill_dir,
    default_litho_skill_dir, default_sdd_skill_dir, discover_meta_files, enforce_context_max_size,
    extract_context_section, grep_file, grep_repomix_pack, grep_text, has_litho_research_artifacts,
    litho_human_complete, litho_human_complete_with_research, litho_research_ready, meta_inputs_ready,
    meta_inputs_status, persist_meta_inputs, read_agent_context_status, read_agent_pack_file,
    read_pack_text_cached, resolve_litho_skill_dir, resolve_sdd_skill_dir, sdd_phase_output_path,
    split_context_sections, write_agent_context, AgentPackFileContent, AssetGenerationPlan,
    ContextOverview, ContextSection, GrepMatch, META_FILENAME, AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS,
    AGENT_CONTEXT_SAVE_MAX_CHARS, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS, LITHO_CORE_RESEARCH_FILES,
    LITHO_REQUIRED_HUMAN_FILES,
};
#[cfg(feature = "repomix")]
pub use assets::{
    agent_pack_fresh, agent_pack_ready, agent_pack_synced_with_head, maybe_pack_agent_assets,
    pack_agent_assets, AgentPackReport,
};
#[cfg(not(feature = "repomix"))]
pub use assets::agent_pack_ready;

pub use citations::{extract_source_citations, merge_citations};
pub use doc::{read_json, KnowledgeDoc, parse_markdown, parse_markdown_at, read_doc, render_markdown, write_doc};
pub use error::{ipc_string, CoreError, Result, TerrainError, TerrainErrorBody};
pub use ipc::{
    AgentContextGenerationResult, AskStreamEvent, ChatPhase, ChatReply, ChatTokenUsage,
    ChatToolCallRecord, ChatToolCallStatus, LithoGenerationJob, LithoGenerationResult, LlmStatus,
    ProjectInitResult,
};
pub use integrations::{
    apply_env_integration, bundled_terrain_cli, bundled_tools, deploy_agent_toolchain,
    deploy_agent_toolchain_with_options, deploy_preset_skills_to_home, discover_bundled_tools_from_packages,
    discover_preset_skills_runtime, ensure_bundled_tools_initialized, ensure_preset_skills_initialized,
    find_codegraph_wrapper_under, get_env_status, init_bundled_tools, init_preset_skills_root, invalidate_env_status_cache,
    invalidate_env_status_cache_for_repo, load_usage_snapshot, packages_root, plan_env_integration,
    probe_usage_sources, resolve_sidecar_next_to_exe, summarize_agent_env_light, AgentToolPaths,
    BundledTools, DeployOptions, EnvApplyResult, EnvIntegrationStatus, EnvPlan, EnvPlanStep,
    EnvStatus, UsageDetailLevel, UsageModelBreakdown, UsagePeriodEntry, UsageProbeResult,
    UsageSnapshot, UsageSourceStatus, UsageTotals,
};
pub use progress::{
    EnvApplyProgress, LithoProgress, ProgressEvent, ProgressKind, ProjectInitProgress, SddProgress,
};
pub use prompts::{
    build_litho_composition_prompt, build_litho_generation_prompt, build_sdd_llm_prompt,
    build_sdd_phase_prompt, plan_litho_generation, plan_sdd_workflow,
};
pub use sessions::{
    clear_active_ask_session, create_ask_session, create_sdd_session, delete_ask_session,
    delete_sdd_session, discard_ask_session, get_active_ask_session, get_active_sdd_session,
    get_sdd_status, list_ask_sessions, list_sdd_sessions, load_ask_messages, resolve_ask_session_id,
    resolve_sdd_session_id, save_ask_messages, save_sdd_output, set_active_ask_session,
    set_active_sdd_session,
};
pub use freshness::{
    codegraph_drift, compute_freshness, format_freshness_trust_block, git_snapshot,
    read_freshness_ledger, resolve_freshness_summary, write_freshness_ledger, CodegraphDriftReport,
    FRESH_THRESHOLD, MACRO_PRELOAD_THRESHOLD, VERIFY_THRESHOLD,
};
pub use human::{count_human_docs, list_human_docs, read_human_doc};
pub use ingest::{AgentPackSummary, ProjectScanner, ScanReport};
pub use model_text::{
    extract_markdown_body, prepare_chat_markdown, prepare_model_markdown, repair_flattened_markdown,
    strip_model_reasoning, unwrap_markdown_fence,
};
pub use preset_skills::{
    default_ask_skill_dir, preset_skill_dir, preset_skills_root, resolve_preset_skill_dir,
    user_preset_skills_dir,
};
pub use paths::KnowledgePaths;
pub use path_portable::{
    path_in_repo, resolve_stored_repo_path, stored_repo_path, to_tilde_path,
    is_stored_repo_marker, normalize_repo_hint, REPO_AGENT_TOOLS_MANIFEST, REPO_ROOT_MARKER,
};
pub use project::{get_project_overview, merge_overview_freshness, resolve_project_repo_path, write_project_remark};
pub use registry::{
    knowledge_root_for_repo, list_stale_registry_projects, register_project, unregister_project,
    StaleProjectSummary,
};
pub use repo::validate_repo_path;
pub use settings::{
    default_profile_for, load_model_settings, normalize_model_settings, profile_for_provider,
    save_model_settings, settings_path, AcpSettings, AgentExecution, AskExecution, ModelSettings,
    ProviderProfile, DEFAULT_ACP_ARGS, DEFAULT_ACP_BINARY, DEFAULT_LMSTUDIO_API_KEY,
    DEFAULT_LMSTUDIO_BASE_URL, DEFAULT_LMSTUDIO_MODEL, DEFAULT_OLLAMA_HOST, DEFAULT_OLLAMA_MODEL,
    DEFAULT_OPENAI_BASE_URL, DEFAULT_OPENAI_MODEL,
};
pub use schema::{
    AgentContextMeta, AgentContextStatus, AgentEnvStatus, AgentPackMeta, AssetGenerator, AssetTrack,
    AssetTrackHealth, CitationKind, DocCounts, DocFrontmatter, DocType, EventMeta, FreshnessDriftFactor,
    FreshnessSummary,
    HumanDocEntry,
    InterfaceMeta, LithoPlan, LithoStatus, ProjectMeta, ProjectOverview, QuickRefreshResult,
    RouteMeta, SddPhase,
    SddPhaseInfo, SddPhaseResult, SddPlan, AskSessionInfo, SddSessionInfo, SddStatus, SourceCitation, SourceSlice, SyncMeta,
    TokenHeavyFile,
};
pub use search::{
    KnowledgeSearch, ProjectSummary, SearchHit, SearchOptions, read_doc_at, read_doc_at_in_project,
};
pub use shell_path::{
    augment_path_from_login_shell, command_on_path, resolve_command, resolve_executable,
};
pub use process::{async_command, command as process_command, hide_console, hide_console_async};
pub use source::{read_source_slice, resolve_source_citation};
