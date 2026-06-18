use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{CoreError, Result};
use crate::paths::KnowledgePaths;

#[derive(Debug, Clone, Deserialize)]
pub struct EnvCatalog {
    pub version: u32,
    pub integrations: Vec<IntegrationDef>,
    #[serde(default)]
    pub agents_md_blocks: Vec<AgentsMdBlockDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationDef {
    pub id: String,
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub skill_dir: Option<String>,
    #[serde(default)]
    pub preset_skill: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub check: Option<Vec<String>>,
    #[serde(default)]
    pub install_steps: Vec<InstallStep>,
    #[serde(default)]
    pub skip_commands: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Provided by MindMesh app bundle; not user-toggleable when available.
    #[serde(default)]
    pub bundled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallStep {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentsMdBlockDef {
    pub id: String,
    pub version: u32,
    pub fragment: String,
}

pub fn env_catalog_root() -> PathBuf {
    if let Ok(root) = std::env::var("MIND_MESH_ENV_CATALOG") {
        return PathBuf::from(root);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../env-catalog")
}

pub fn load_catalog() -> Result<EnvCatalog> {
    let path = env_catalog_root().join("catalog.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot read env catalog {}: {e}", path.display()))
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        CoreError::InvalidDoc(format!("invalid env catalog {}: {e}", path.display()))
    })
}

pub fn resolve_skill_source(catalog_root: &Path, item: &IntegrationDef) -> PathBuf {
    if let Some(preset) = &item.preset_skill {
        return KnowledgePaths::preset_skills_root().join(preset);
    }
    if let Some(dir) = &item.skill_dir {
        let env_path = catalog_root.join("skills").join(dir);
        if env_path.is_dir() {
            return env_path;
        }
    }
    catalog_root.join("skills")
}

pub fn resolve_fragment_path(catalog_root: &Path, fragment: &str) -> PathBuf {
    catalog_root.join(fragment)
}
