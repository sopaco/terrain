mod assets;
mod env;
mod knowledge;
mod payloads;
mod project;
mod sessions;
mod settings;
mod usage;
mod util;
mod workflows;

use terrain_agent::{
    knowledge_paths_from_env, load_model_settings, resolve_acp_settings, AcpSettings,
};
use terrain_core::KnowledgePaths;

pub(crate) fn resolved_acp_settings() -> AcpSettings {
    load_model_settings()
        .map(|s| s.acp)
        .unwrap_or_else(resolve_acp_settings)
}



pub(crate) use terrain_agent::resolve_knowledge_settings as resolved_knowledge_settings;

pub(crate) fn slugify_repo(repo_path: &str) -> String {
    slug::slugify(
        std::path::Path::new(repo_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project"),
    )
}

pub fn init_paths() -> KnowledgePaths {
    let paths = knowledge_paths_from_env();
    let _ = paths.ensure_layout();
    paths
}

pub use assets::*;
pub use env::*;
pub use knowledge::*;
pub use project::*;
pub use sessions::*;
pub use settings::*;
pub use usage::*;
pub use workflows::*;
