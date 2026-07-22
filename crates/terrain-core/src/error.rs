use serde::Serialize;
use thiserror::Error;

/// Unified error type for terrain-core (and IPC surfaces).
#[derive(Debug, Error)]
pub enum TerrainError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("document not found: {0}")]
    DocNotFound(String),

    #[error("invalid document: {0}")]
    InvalidDoc(String),

    #[error("pack failed: {0}")]
    Pack(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("{0}")]
    Other(String),
}

/// Backward-compatible alias used throughout terrain-core.
pub type CoreError = TerrainError;

pub type Result<T> = std::result::Result<T, TerrainError>;

#[derive(Debug, Clone, Serialize)]
pub struct TerrainErrorBody {
    pub code: String,
    pub message: String,
}

impl TerrainError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Yaml(_) => "yaml",
            Self::Json(_) => "json",
            Self::ProjectNotFound(_) => "project_not_found",
            Self::DocNotFound(_) => "doc_not_found",
            Self::InvalidDoc(_) => "invalid_doc",
            Self::Pack(_) => "pack",
            Self::Validation(_) => "validation",
            Self::Agent(_) => "agent",
            Self::Other(_) => "other",
        }
    }

    pub fn body(&self) -> TerrainErrorBody {
        TerrainErrorBody {
            code: self.code().into(),
            message: self.to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.body()).unwrap_or_else(|_| self.to_string())
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
}

/// Format any error for Tauri IPC (`String` command results).
pub fn ipc_string(err: impl std::fmt::Display) -> String {
    err.to_string()
}
