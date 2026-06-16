mod agent_context;
mod context_layers;
mod env;
mod litho;
mod pack_read;
mod project_meta;
mod query;
mod sdd;

#[cfg(feature = "repomix")]
mod repomix;

pub use agent_context::{
    agent_context_ready, build_agent_context_prompt, default_agent_arch_skill_dir,
    read_agent_context_status, write_agent_context,
};
pub use env::{
    agents_md_ready, apply_env_integration, env_catalog_root, get_env_status, load_catalog,
    patch_agents_md, plan_env_integration, EnvApplyProgress, EnvApplyResult, EnvIntegrationStatus,
    EnvPlan, EnvPlanStep, EnvStatus,
};
pub use context_layers::{
    build_context_overview, enforce_context_max_size, extract_context_section,
    split_context_sections, ContextOverview, ContextSection,
    AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS, AGENT_CONTEXT_SAVE_MAX_CHARS,
    AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
};
pub use litho::{
    build_litho_composition_prompt, build_litho_generation_prompt, count_deep_exploration_modules,
    count_litho_research_modules, count_markdown_in_dir, default_litho_skill_dir,
    has_litho_research_artifacts, litho_human_complete, litho_human_complete_with_research,
    litho_research_ready, plan_litho_generation, resolve_litho_skill_dir,
    LITHO_CORE_RESEARCH_FILES, LITHO_REQUIRED_HUMAN_FILES,
};
pub use pack_read::{agent_pack_ready, read_agent_pack_file, AgentPackFileContent};
pub use project_meta::{
    collect_knowledge_dir_inputs, collect_project_meta, discover_meta_files,
    format_meta_bundle_for_prompt, meta_inputs_ready, meta_inputs_status, persist_meta_inputs,
    CollectedMetaInput, MetaInputSpec, MetaInputsManifest, ProjectMetaBundle, ProjectMetaFile,
    META_FILENAME,
};
pub use query::{grep_file, grep_repomix_pack, grep_text, GrepMatch};
pub use sdd::{
    build_sdd_llm_prompt, build_sdd_phase_prompt, create_sdd_session, default_sdd_skill_dir,
    delete_sdd_session, get_active_sdd_session, get_sdd_status, list_sdd_sessions, new_session_id,
    plan_sdd_workflow, resolve_sdd_session_id, resolve_sdd_skill_dir, save_sdd_output,
    sdd_phase_output_path, set_active_sdd_session,
};

#[cfg(feature = "repomix")]
pub use repomix::{pack_agent_assets, AgentPackReport};

use crate::paths::KnowledgePaths;
use crate::schema::LithoPlan;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetGenerationPlan {
    pub litho: LithoPlan,
    pub agent_pack_command: String,
}

pub fn build_generation_plan(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
) -> AssetGenerationPlan {
    AssetGenerationPlan {
        litho: plan_litho_generation(paths, project_slug, repo_path),
        agent_pack_command: format!("mind-mesh assets pack-agent {repo_path} --slug {project_slug}"),
    }
}
