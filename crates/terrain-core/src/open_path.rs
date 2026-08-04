//! Open local files or folders in the system file manager.

use std::path::Path;

use crate::error::{CoreError, Result};
use crate::platform::user_home;
use crate::process::command as hidden_command;

/// Reveal a file or open a folder in the OS file manager.
pub fn open_path_in_file_manager(path: &str) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(CoreError::validation(format!(
            "path does not exist: {}",
            path.display()
        )));
    }
    if !is_path_allowed(path) {
        return Err(CoreError::validation(format!(
            "path is outside allowed locations: {}",
            path.display()
        )));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| CoreError::validation(format!("failed to resolve path: {e}")))?;

    #[cfg(target_os = "macos")]
    {
        let mut cmd = if canonical.is_file() {
            let mut c = hidden_command("open");
            c.args(["-R", &canonical.display().to_string()]);
            c
        } else {
            let mut c = hidden_command("open");
            c.arg(canonical.display().to_string());
            c
        };
        cmd.status()
            .map_err(|e| CoreError::Other(format!("failed to open path: {e}")))?;
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = if canonical.is_file() {
            let mut c = hidden_command("explorer");
            c.arg(format!("/select,{}", canonical.display()));
            c
        } else {
            let mut c = hidden_command("explorer");
            c.arg(canonical.display().to_string());
            c
        };
        cmd.status()
            .map_err(|e| CoreError::Other(format!("failed to open path: {e}")))?;
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let target = if canonical.is_file() {
            canonical
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(canonical)
        } else {
            canonical
        };
        hidden_command("xdg-open")
            .arg(target.display().to_string())
            .status()
            .map_err(|e| CoreError::Other(format!("failed to open path: {e}")))?;
    }

    Ok(())
}

fn is_path_allowed(path: &Path) -> bool {
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    if let Some(home) = user_home() {
        if let Ok(home_canonical) = home.canonicalize() {
            if canonical.starts_with(&home_canonical) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_paths_outside_home() {
        assert!(!is_path_allowed(Path::new("/etc/passwd")));
    }

    #[test]
    fn allows_paths_under_home() {
        let Some(home) = user_home() else {
            return;
        };
        let dir = home.join(".terrain");
        let _ = fs::create_dir_all(&dir);
        assert!(is_path_allowed(&dir));
    }
}
