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
pub mod source;

pub use assets::{
    agent_context_ready, agent_pack_ready, build_agent_context_prompt, build_context_overview,
    build_generation_plan, build_litho_composition_prompt, build_litho_generation_prompt,
    build_sdd_llm_prompt, build_sdd_phase_prompt, collect_project_meta, default_agent_arch_skill_dir,
    default_litho_skill_dir, default_sdd_skill_dir, discover_meta_files, enforce_context_max_size,
    extract_context_section, get_env_status, get_sdd_status, grep_file, grep_repomix_pack, grep_text,
    has_litho_research_artifacts, litho_human_complete, litho_human_complete_with_research,
    litho_research_ready, apply_env_integration, collect_knowledge_dir_inputs, count_markdown_in_dir, meta_inputs_ready, meta_inputs_status,
    plan_env_integration, plan_litho_generation, plan_sdd_workflow, patch_agents_md,
    persist_meta_inputs, read_agent_context_status, read_agent_pack_file, resolve_litho_skill_dir,
    resolve_sdd_skill_dir, sdd_phase_output_path, split_context_sections, write_agent_context,
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
pub use project::{get_project_overview, resolve_project_repo_path};
pub use registry::{knowledge_root_for_repo, list_stale_registry_projects, register_project, StaleProjectSummary};
pub use schema::{
    AgentContextMeta, AgentContextStatus, AgentEnvStatus, AgentPackMeta, AssetGenerator, AssetTrack,
    AssetTrackHealth, CitationKind, DocCounts, DocFrontmatter, DocType, EventMeta, FreshnessSummary,
    HumanDocEntry,
    InterfaceMeta, LithoPlan, LithoStatus, ProjectMeta, ProjectOverview, RouteMeta, SddPhase,
    SddPhaseInfo, SddPhaseResult, SddPlan, SddStatus, SourceCitation, SourceSlice, SyncMeta,
    TokenHeavyFile,
};
pub use search::{
    KnowledgeSearch, ProjectSummary, SearchHit, SearchOptions, read_doc_at, read_doc_at_in_project,
};
pub use source::{read_source_slice, resolve_source_citation};
