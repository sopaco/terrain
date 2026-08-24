//! AI engineering environment optimization — catalog, status, plan, apply.

mod agents_md;
mod apply;
mod catalog;
mod status;

pub use agents_md::{patch_agents_md, agents_md_ready};
pub use apply::{apply_env_integration, EnvApplyResult};
pub use crate::progress::EnvApplyProgress;
pub use catalog::{
    deploy_env_catalog_to_home, discover_env_catalog_runtime, ensure_env_catalog_initialized,
    env_catalog_root, init_env_catalog_root, load_catalog, user_env_catalog_dir,
};
pub use status::{
    get_env_status, invalidate_env_status_cache, invalidate_env_status_cache_for_repo,
    plan_env_integration, summarize_agent_env_light, EnvIntegrationStatus, EnvPlan, EnvPlanStep,
    EnvStatus,
};
