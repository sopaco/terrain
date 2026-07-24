//! Shared helpers for Tauri command handlers.

use std::path::PathBuf;

use terrain_core::{validate_repo_path, Result as CoreResult};

/// Validate `repo_path` and return it as a `PathBuf` for downstream APIs.
pub fn validate_repo(path: &str) -> Result<PathBuf, String> {
    validate_repo_path(path).map_err(|e| e.to_string())?;
    Ok(PathBuf::from(path))
}

/// Map a `terrain_core` result into the `String` error form expected by Tauri IPC.
pub fn map_core_err<T>(result: CoreResult<T>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}
