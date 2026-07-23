//! Env status cache keyed by repo fingerprint.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::agent_tools_deploy::agent_bin_dir;
use crate::error::Result;

use super::types::EnvStatus;

struct EnvStatusCacheEntry {
    fingerprint: u64,
    status: EnvStatus,
}

static ENV_STATUS_CACHE: Mutex<Option<HashMap<String, EnvStatusCacheEntry>>> = Mutex::new(None);

/// Drop cached env status (all repos). Call after env apply or global tool deploy.
pub fn invalidate_env_status_cache() {
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock() {
        *guard = None;
    }
}

/// Drop cached env status for one repository.
pub fn invalidate_env_status_cache_for_repo(repo: &Path) {
    let Ok(key) = env_cache_key(repo) else {
        return;
    };
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock()
        && let Some(map) = guard.as_mut() {
            map.remove(&key);
        }
}

pub(crate) fn env_cache_key(repo: &Path) -> Result<String> {
    Ok(repo
        .canonicalize()
        .unwrap_or_else(|_| repo.to_path_buf())
        .to_string_lossy()
        .into_owned())
}

pub(crate) fn env_cache_fingerprint(repo: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    for path in env_cache_watch_paths(repo) {
        path.to_string_lossy().hash(&mut hasher);
        if let Ok(meta) = std::fs::metadata(&path) {
            meta.len().hash(&mut hasher);
            meta.modified().ok().hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn env_cache_watch_paths(repo: &Path) -> Vec<PathBuf> {
    let bin = agent_bin_dir();
    vec![
        repo.join("AGENTS.md"),
        repo.join(".gitignore"),
        repo.join(".codegraph/codegraph.db"),
        repo.join(".terrain/env/manifest.json"),
        repo.join(".terrain/env/agent-tools.json"),
        repo.join(".agents/skills/terrain-knowledge-skill/SKILL.md"),
        repo.join(".agents/skills/codegraph-skill/SKILL.md"),
        repo.join(".agents/skills/rtk-skill/SKILL.md"),
        repo.join(".agents/skills/repomix-context-skill/SKILL.md"),
        repo.join(".claude/skills/terrain-knowledge-skill/SKILL.md"),
        repo.join(".claude/skills/codegraph-skill/SKILL.md"),
        repo.join(".claude/skills/rtk-skill/SKILL.md"),
        repo.join(".claude/skills/repomix-context-skill/SKILL.md"),
        bin.join("rtk"),
        bin.join("codegraph"),
    ]
}

pub(crate) fn env_cache_get(key: &str, fingerprint: u64) -> Option<EnvStatus> {
    let guard = ENV_STATUS_CACHE.lock().ok()?;
    let map = guard.as_ref()?;
    let entry = map.get(key)?;
    if entry.fingerprint == fingerprint {
        Some(entry.status.clone())
    } else {
        None
    }
}

pub(crate) fn env_cache_put(key: String, fingerprint: u64, status: EnvStatus) {
    if let Ok(mut guard) = ENV_STATUS_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(
            key,
            EnvStatusCacheEntry {
                fingerprint,
                status,
            },
        );
    }
}
