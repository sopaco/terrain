//! Environment integration status, planning, and caching.

mod cache;
mod plan;
mod probe;
mod types;

pub use cache::{invalidate_env_status_cache, invalidate_env_status_cache_for_repo};
pub use plan::{get_env_status, plan_env_integration, summarize_agent_env_light};
pub use probe::gitignore_has_patterns;
pub use types::{EnvIntegrationStatus, EnvPlan, EnvPlanStep, EnvStatus};

pub(crate) use plan::dependencies_satisfied;
pub(crate) use probe::{
    dependency_ready, rtk_runtime_ready,
    tool_check_passes,
};

use std::path::{Path, PathBuf};

use super::catalog::{env_catalog_root, resolve_skill_source, IntegrationDef};

pub(crate) fn skill_source_path(def: &IntegrationDef) -> PathBuf {
    resolve_skill_source(&env_catalog_root(), def)
}

pub(crate) fn substitute_install_args(
    _repo: &Path,
    _def: &IntegrationDef,
    args: &[String],
) -> Vec<String> {
    args.to_vec()
}
