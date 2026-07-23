//! Gitignore-aware repository traversal aligned with each repo's `.gitignore`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::gitignore::Gitignore;
use ignore::WalkBuilder;
use ignore::Match;

use std::sync::LazyLock;

type GitignoreCache = Mutex<HashMap<PathBuf, (Option<std::time::SystemTime>, Gitignore)>>;

static GITIGNORE_CACHE: LazyLock<GitignoreCache> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub const META_DISCOVER_MAX_DEPTH: usize = 6;

/// Walk repository entries for `terrain-meta.json` discovery, honoring `.gitignore`.
pub fn discover_repo_walk(repo: &Path) -> ignore::Walk {
    WalkBuilder::new(repo)
        .max_depth(Some(META_DISCOVER_MAX_DEPTH))
        .hidden(true)
        .require_git(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .filter_entry(|entry| allow_meta_discover_entry(entry.path()))
        .build()
}

fn allow_meta_discover_entry(path: &Path) -> bool {
    !is_under_git_dir(path) && !is_terrain_knowledge(path)
}

/// Build a matcher from `{repo}/.gitignore` (cached by repo path + `.gitignore` mtime).
pub fn build_repo_gitignore(repo: &Path) -> Gitignore {
    let gi = repo.join(".gitignore");
    let mtime = gi.metadata().ok().and_then(|m| m.modified().ok());
    let key = repo.to_path_buf();
    if let Ok(guard) = GITIGNORE_CACHE.lock()
        && let Some((cached_mtime, ig)) = guard.get(&key)
            && cached_mtime == &mtime {
                return ig.clone();
            }

    let ig = if gi.is_file() {
        Gitignore::new(gi).0
    } else {
        Gitignore::empty()
    };

    if let Ok(mut guard) = GITIGNORE_CACHE.lock() {
        guard.insert(key, (mtime, ig.clone()));
    }
    ig
}

/// Whether `path` should be excluded when resolving meta inputs (gitignore + knowledge dir).
pub fn is_path_gitignored(repo: &Path, path: &Path) -> bool {
    if is_terrain_knowledge(path) {
        return true;
    }
    let Ok(rel) = path.strip_prefix(repo) else {
        return false;
    };
    let ig = build_repo_gitignore(repo);
    matches!(
        ig.matched_path_or_any_parents(rel, false),
        Match::Ignore(_)
    )
}

pub fn is_terrain_knowledge(path: &Path) -> bool {
    let parts: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    parts
        .windows(2)
        .any(|w| w[0] == ".terrain" && w[1] == "knowledge")
}

fn is_under_git_dir(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn gitignore_skips_target_during_meta_discover() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        fs::write(repo.join(".gitignore"), "/target/\n").unwrap();
        fs::create_dir_all(repo.join("target/deep")).unwrap();
        fs::write(
            repo.join("target/deep/terrain-meta.json"),
            r#"{"version":1,"inputs":[]}"#,
        )
        .unwrap();
        fs::write(
            repo.join("terrain-meta.json"),
            r#"{"version":1,"inputs":[]}"#,
        )
        .unwrap();

        let names: Vec<_> = discover_repo_walk(repo)
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .filter_map(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
            })
            .collect();

        assert!(names.contains(&META_FILENAME.to_string()));
        assert_eq!(
            names.iter().filter(|n| n.as_str() == META_FILENAME).count(),
            1,
            "target/ meta file must be gitignored: {names:?}"
        );
    }

    const META_FILENAME: &str = "terrain-meta.json";

    #[test]
    fn is_path_gitignored_respects_repo_gitignore() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        fs::write(repo.join(".gitignore"), "node_modules/\n").unwrap();
        let ignored = repo.join("node_modules/pkg/index.js");
        fs::create_dir_all(ignored.parent().unwrap()).unwrap();
        fs::write(&ignored, "x").unwrap();
        let kept = repo.join("src/main.rs");
        fs::create_dir_all(kept.parent().unwrap()).unwrap();
        fs::write(&kept, "fn main() {}").unwrap();

        let ig = build_repo_gitignore(repo);
        assert!(!ig.is_empty(), "gitignore matcher should load patterns");
        assert!(
            matches!(
                ig.matched_path_or_any_parents("node_modules/pkg/index.js", false),
                Match::Ignore(_)
            ),
            "node_modules/ should ignore nested files"
        );

        assert!(is_path_gitignored(repo, &ignored));
        assert!(!is_path_gitignored(repo, &kept));
    }
}
