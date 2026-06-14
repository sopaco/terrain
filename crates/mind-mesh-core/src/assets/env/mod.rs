//! AI engineering environment optimization — catalog, status, plan, apply.

mod agents_md;
mod apply;
mod catalog;
mod status;

pub use agents_md::{patch_agents_md, agents_md_ready};
pub use apply::{apply_env_integration, EnvApplyProgress, EnvApplyResult};
pub use catalog::{env_catalog_root, load_catalog};
pub use status::{get_env_status, plan_env_integration, EnvIntegrationStatus, EnvPlan, EnvPlanStep, EnvStatus};
