//! External toolchain integration (env catalog, bundled tools, usage).

pub use crate::agent_tools_deploy::{
    agent_bin_dir, deploy_agent_toolchain, deploy_agent_toolchain_with_options,
    write_repo_agent_tools_manifest, AgentToolPaths, DeployOptions,
};
pub use crate::assets::{
    apply_env_integration, get_env_status, invalidate_env_status_cache,
    invalidate_env_status_cache_for_repo, plan_env_integration, summarize_agent_env_light,
    EnvApplyResult, EnvIntegrationStatus, EnvPlan, EnvPlanStep, EnvStatus,
};
pub use crate::bundled_tools::{
    bundled_terrain_cli, bundled_tools, discover_bundled_tools_from_packages,
    ensure_bundled_tools_initialized, find_codegraph_wrapper_under, init_bundled_tools,
    packages_root, resolve_sidecar_next_to_exe, BundledTools,
};
pub use crate::preset_skills::{
    default_ask_skill_dir, deploy_preset_skills_to_home, discover_preset_skills_runtime,
    ensure_preset_skills_initialized, init_preset_skills_root, preset_skill_dir,
    preset_skills_root, resolve_preset_skill_dir, user_preset_skills_dir,
};
pub use crate::usage::{
    load_usage_snapshot, probe_usage_sources, UsageDetailLevel, UsageModelBreakdown,
    UsagePeriodEntry, UsageProbeResult, UsageSnapshot, UsageSourceStatus, UsageTotals,
};
