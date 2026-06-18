pub mod assets;
pub mod citations;
pub mod doc;
pub mod error;
pub mod freshness;
pub mod human;
pub mod ingest;
pub mod model_text;
pub mod paths;
pub mod project;
pub mod registry;
pub mod render;
pub mod schema;
pub mod search;
pub mod agent_tools_deploy;
pub mod bundled_tools;
pub mod shell_path;
pub mod source;

pub use assets::{
    agent_context_ready, agent_pack_ready, build_agent_context_prompt, build_context_overview,
    build_generation_plan, build_litho_composition_prompt, build_litho_generation_prompt,
    build_sdd_llm_prompt, build_sdd_phase_prompt, collect_project_meta, create_sdd_session,
    default_agent_arch_skill_dir, default_litho_skill_dir, default_sdd_skill_dir, delete_sdd_session,
    discover_meta_files,
    enforce_context_max_size, extract_context_section, get_active_sdd_session, get_env_status,
    get_sdd_status, grep_file, grep_repomix_pack, grep_text, has_litho_research_artifacts,
    invalidate_env_status_cache, invalidate_env_status_cache_for_repo,
    litho_human_complete, litho_human_complete_with_research, litho_research_ready,
    apply_env_integration, collect_knowledge_dir_inputs, count_markdown_in_dir, meta_inputs_ready,
    meta_inputs_status, list_sdd_sessions, plan_env_integration, plan_litho_generation,
    plan_sdd_workflow, patch_agents_md, persist_meta_inputs, read_agent_context_status,
    read_agent_pack_file, resolve_litho_skill_dir, resolve_sdd_session_id, resolve_sdd_skill_dir,
    save_sdd_output, sdd_phase_output_path, set_active_sdd_session, split_context_sections,
    summarize_agent_env_light, write_agent_context,
    LITHO_CORE_RESEARCH_FILES, LITHO_REQUIRED_HUMAN_FILES,
    EnvApplyProgress, EnvApplyResult, EnvIntegrationStatus, EnvPlan, EnvPlanStep, EnvStatus,
    AgentPackFileContent, AssetGenerationPlan, ContextOverview, ContextSection, GrepMatch,
    META_FILENAME, AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS, AGENT_CONTEXT_SAVE_MAX_CHARS,
    AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
};
#[cfg(feature = "repomix")]
pub use assets::{pack_agent_assets, AgentPackReport};
pub use citations::{extract_source_citations, merge_citations};
pub use doc::{read_json, KnowledgeDoc, parse_markdown, parse_markdown_at, read_doc, render_markdown, write_doc};
pub use error::{CoreError, Result};
pub use freshness::{
    compute_freshness, format_freshness_trust_block, git_snapshot, read_freshness_ledger,
    write_freshness_ledger, FRESH_THRESHOLD, MACRO_PRELOAD_THRESHOLD, VERIFY_THRESHOLD,
};
pub use human::{count_human_docs, list_human_docs, read_human_doc};
pub use ingest::{ProjectScanner, ScanReport};
pub use model_text::{
    extract_markdown_body, prepare_chat_markdown, prepare_model_markdown, repair_flattened_markdown,
    strip_model_reasoning, unwrap_markdown_fence,
};
pub use paths::KnowledgePaths;
pub use project::{get_project_overview, merge_overview_freshness, resolve_project_repo_path, write_project_remark};
pub use registry::{
    knowledge_root_for_repo, list_stale_registry_projects, register_project, unregister_project,
    StaleProjectSummary,
};
pub use schema::{
    AgentContextMeta, AgentContextStatus, AgentEnvStatus, AgentPackMeta, AssetGenerator, AssetTrack,
    AssetTrackHealth, CitationKind, DocCounts, DocFrontmatter, DocType, EventMeta, FreshnessDriftFactor,
    FreshnessSummary,
    HumanDocEntry,
    InterfaceMeta, LithoPlan, LithoStatus, ProjectMeta, ProjectOverview, RouteMeta, SddPhase,
    SddPhaseInfo, SddPhaseResult, SddPlan, SddSessionInfo, SddStatus, SourceCitation, SourceSlice, SyncMeta,
    TokenHeavyFile,
};
pub use search::{
    KnowledgeSearch, ProjectSummary, SearchHit, SearchOptions, read_doc_at, read_doc_at_in_project,
};
pub use agent_tools_deploy::{
    agent_bin_dir, deploy_agent_toolchain, deploy_agent_toolchain_with_options,
    write_repo_agent_tools_manifest, AgentToolPaths, DeployOptions,
};
pub use bundled_tools::{
    bundled_mind_mesh_cli, bundled_tools, discover_bundled_tools_from_packages,
    ensure_bundled_tools_initialized, init_bundled_tools, packages_root,
    resolve_sidecar_next_to_exe, BundledTools,
};
pub use shell_path::{
    augment_path_from_login_shell, command_on_path, resolve_command, resolve_executable,
};
pub use source::{read_source_slice, resolve_source_citation};
