//! Portable path strings for knowledge assets and agent manifests.
//!
//! Knowledge files committed to Git should use repo-relative paths; machine-local
//! manifests should use `~` paths instead of absolute `/Users/...` paths.

use std::path::{Path, PathBuf};

/// Sentinel stored in `.terrain/` meta when the repo root is implicit.
pub const REPO_ROOT_MARKER: &str = ".";

/// Relative path to the per-repo agent tools manifest (portable across clones).
pub const REPO_AGENT_TOOLS_MANIFEST: &str = ".terrain/env/agent-tools.json";

/// Convert an absolute path under `$HOME` to `~/…` for agent manifests.
pub fn to_tilde_path(path: &Path) -> String {
    if let Some(home) = user_home() {
        if let Ok(rest) = path.strip_prefix(&home) {
            let rest = rest.to_string_lossy();
            if rest.is_empty() {
                return "~".into();
            }
            return format!("~/{rest}");
        }
    }
    normalize_slashes(&path.display().to_string())
}

/// Repo-relative path for frontmatter / `source` fields (portable across clones).
pub fn path_in_repo(repo: &Path, path: &Path) -> String {
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let abs = if path.is_absolute() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        repo.join(path)
    };
    if let Ok(rel) = abs.strip_prefix(&repo) {
        let s = rel.display().to_string();
        if s.is_empty() {
            return REPO_ROOT_MARKER.into();
        }
        return normalize_slashes(&s);
    }
    if let Ok(rel) = path.strip_prefix(repo) {
        let s = rel.display().to_string();
        if s.is_empty() {
            return REPO_ROOT_MARKER.into();
        }
        return normalize_slashes(&s);
    }
    normalize_slashes(&path.display().to_string())
}

/// Value for `repo_path` fields persisted under `.terrain/` (not machine-specific).
pub fn stored_repo_path(repo: &Path) -> String {
    let _ = repo;
    REPO_ROOT_MARKER.into()
}

/// Resolve a stored repo pointer (`.`, empty, legacy absolute) to an absolute path.
pub fn resolve_stored_repo_path(stored: &str, project_slug: &str) -> Option<String> {
    if stored.is_empty() || stored == REPO_ROOT_MARKER {
        return crate::registry::repo_path_for_slug(project_slug);
    }
    Some(stored.to_string())
}

fn normalize_slashes(s: &str) -> String {
    s.replace('\\', "/")
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn to_tilde_path_under_home() {
        if let Some(home) = user_home() {
            let p = home.join(".terrain/bin/rtk");
            assert_eq!(to_tilde_path(&p), "~/.terrain/bin/rtk");
        }
    }

    #[test]
    fn path_in_repo_relative() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let spec = repo.join("examples/openapi.yaml");
        fs::create_dir_all(spec.parent().unwrap()).unwrap();
        fs::write(&spec, "openapi: 3").unwrap();
        assert_eq!(
            path_in_repo(&repo, &spec),
            "examples/openapi.yaml"
        );
        assert_eq!(path_in_repo(&repo, &repo), ".");
    }

    #[test]
    fn stored_repo_path_is_marker() {
        assert_eq!(stored_repo_path(Path::new("/any/path")), ".");
    }
}
