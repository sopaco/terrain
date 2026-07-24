use std::path::Path;

use crate::error::{Result, TerrainError};

pub fn validate_repo_path(repo_path: &str) -> Result<()> {
    let path = Path::new(repo_path);
    if !path.exists() {
        return Err(TerrainError::validation(format!(
            "repository path does not exist: {repo_path}"
        )));
    }
    if !path.is_dir() {
        return Err(TerrainError::validation(format!(
            "repository path is not a directory: {repo_path}"
        )));
    }
    Ok(())
}
