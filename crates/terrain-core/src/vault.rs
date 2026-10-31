//! Obsidian vault integration — persistent-memory installer used by the
//! env `user_env` integration and the fleet dashboard.
//!
//! Conventions match the community memory-setup flow: `VAULT_DIR` env override,
//! default `$HOME/vault`; autosave scripts under `~/scripts/`; SessionEnd hook
//! merged into `~/.claude/settings.json`; graphify graphs linked per project.

use crate::error::{CoreError, Result};
use crate::registry::load_registry;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const AUTOSAVE_SCRIPT: &str = "session_autosave.py";
pub const IMPORT_SCRIPT: &str = "claude_to_obsidian.py";
pub const SYNC_SCRIPT: &str = "sync_claude_obsidian.sh";
pub const VAULT_SUBDIRS: &[&str] = &[
    "permanent",
    "inbox",
    "fleeting",
    "templates",
    "logs",
    "references",
    "chats/code",
    "chats/web",
    "graphify",
];
const CRON_LINE: &str = "0 22 * * * $HOME/scripts/sync_claude_obsidian.sh";
const CATALOG_FILE: &str = "catalog.json";

/// Catalog root for script deployment — same discovery chain the env
/// catalog uses, without reaching into its private module.
fn find_catalog_root() -> PathBuf {
    if let Some(path) = std::env::var_os("TERRAIN_ENV_CATALOG") {
        return PathBuf::from(path);
    }
    let user = home_dir().join(".terrain").join("env-catalog");
    if user.is_dir() {
        return user;
    }
    let exe_relative = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("../Resources/env-catalog")))
        .map(PathBuf::from);
    if let Some(path) = exe_relative {
        if path.join(CATALOG_FILE).is_file() {
            return path;
        }
    }
    std::env::current_dir()
        .ok()
        .and_then(|cwd| cwd.join("../env-catalog").canonicalize().ok())
        .unwrap_or_else(|| home_dir().join("vault"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Vault root: `VAULT_DIR` override, else `~/vault`.
pub fn vault_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("VAULT_DIR") {
        return PathBuf::from(dir);
    }
    home_dir().join("vault")
}

pub fn vault_link_path(slug: &str) -> PathBuf {
    vault_root()
        .join("graphify")
        .join(slug)
        .join("graphify-out")
}

/// Detect the configured state: link exists and points at this project's graph.
pub fn vault_link_current(repo: &Path, slug: &str) -> Option<String> {
    let repo_out = repo.join(".terrain/graphify-out");
    if !repo_out.is_dir() || !vault_root().is_dir() {
        return None;
    }
    let link_path = vault_link_path(slug);
    std::fs::read_link(&link_path)
        .ok()
        .and_then(|target| target.canonicalize().ok())
        .is_some_and(|resolved| repo_out.canonicalize().is_ok_and(|real| resolved == real))
        .then(|| link_path.display().to_string())
}

/// Create or refresh the project's vault link (no-op without a vault).
pub fn sync_vault_link(repo: &Path, slug: &str) -> Result<Option<String>> {
    let repo_out = repo.join(".terrain/graphify-out");
    if !repo_out.is_dir() || !vault_root().is_dir() {
        return Ok(None);
    }
    let link_path = vault_link_path(slug);
    let Some(dest_dir) = link_path.parent() else {
        return Err(CoreError::InvalidDoc(
            "vault link path has no parent".to_owned(),
        ));
    };
    if vault_link_current(repo, slug).is_none() {
        std::fs::create_dir_all(dest_dir)?;
        if std::fs::symlink_metadata(&link_path).is_ok() {
            let _ = std::fs::remove_file(&link_path);
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&repo_out, &link_path)?;
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&repo_out, &link_path).is_err() {
                // Symlinks may need privileges; fall back to a pointer note.
                std::fs::write(
                    link_path.with_extension("vault-link.txt"),
                    format!("Graphify graph for {slug}: {}\n", repo_out.display()),
                )?;
            }
        }
    }
    Ok(Some(link_path.display().to_string()))
}

/// `~/scripts` dir for the autosave/sync scripts.
pub fn user_scripts_dir() -> PathBuf {
    home_dir().join("scripts")
}

fn install_scripts(catalog_root: &Path) -> Result<usize> {
    let src_dir = catalog_root.join("scripts");
    let dest_dir = user_scripts_dir();
    std::fs::create_dir_all(&dest_dir)?;
    let mut installed = 0;
    for name in [AUTOSAVE_SCRIPT, IMPORT_SCRIPT, SYNC_SCRIPT] {
        let src = src_dir.join(name);
        if src.is_file() {
            std::fs::copy(&src, dest_dir.join(name))?;
            installed += 1;
        }
    }
    Ok(installed)
}

/// Merge the SessionEnd autosave hook into `~/.claude/settings.json`
/// (read-modify-write; never removes other hooks; idempotent).
fn register_session_end_hook() -> Result<bool> {
    const COMMAND: &str = "VAULT_DIR=$HOME/vault python3 $HOME/scripts/session_autosave.py";
    let path = home_dir().join(".claude").join("settings.json");
    let mut root: Value = if path.is_file() {
        serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|e| CoreError::InvalidDoc(format!("invalid ~/.claude/settings.json: {e}")))?
    } else {
        serde_json::json!({})
    };
    let existing = root
        .pointer("/hooks/SessionEnd")
        .map(Value::to_string)
        .unwrap_or_default();
    if existing.contains("session_autosave") {
        return Ok(false);
    }
    let obj = root
        .as_object_mut()
        .ok_or_else(|| CoreError::InvalidDoc("settings.json root is not an object".to_owned()))?;
    if !obj.contains_key("hooks") {
        obj.insert("hooks".to_owned(), serde_json::json!({}));
    }
    let hooks = obj
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| CoreError::InvalidDoc("hooks is not an object".to_owned()))?;
    if !hooks.contains_key("SessionEnd") {
        hooks.insert("SessionEnd".to_owned(), serde_json::json!([]));
    }
    let session = hooks
        .get_mut("SessionEnd")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| CoreError::InvalidDoc("SessionEnd is not an array".to_owned()))?;
    session.push(serde_json::json!({
        "hooks": [{"type": "command", "command": COMMAND}]
    }));
    std::fs::create_dir_all(path.parent().expect("settings path has parent"))?;
    if path.is_file() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak-terrain"));
    }
    let tmp = path.with_extension("json.tmp-terrain");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&root)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(true)
}

/// Merge the daily sync cron line (unix only; skipped if already present).
#[cfg(unix)]
fn register_cron() -> Result<bool> {
    use crate::process;
    let shell = process::command("sh")
        .arg("-c")
        .arg(format!(
            "crontab -l 2>/dev/null | grep -qF 'sync_claude_obsidian' || (crontab -l 2>/dev/null; echo '{CRON_LINE}') | crontab -"
        ))
        .output();
    let output = shell.map_err(|e| CoreError::InvalidDoc(format!("failed to run crontab: {e}")))?;
    Ok(output.status.success())
}

#[cfg(windows)]
fn register_cron() -> Result<bool> {
    Ok(false) // scheduled-tasks wiring deferred; autosave hook covers Windows
}

/// One-shot installer for the persistent-memory environment: vault layout,
/// scripts, SessionEnd hook, cron, and graphify links for all projects.
pub fn apply_vault_memory_setup() -> Result<String> {
    let vault = vault_root();
    std::fs::create_dir_all(&vault)?;
    for sub in VAULT_SUBDIRS {
        std::fs::create_dir_all(vault.join(sub))?;
    }
    let catalog_root = find_catalog_root();
    let scripts = install_scripts(&catalog_root)?;
    let hook_added = register_session_end_hook()?;
    let cron_added = register_cron()?;
    let mut linked = 0;
    for entry in load_registry()?.into_iter() {
        if sync_vault_link(Path::new(&entry.repo_path), &entry.slug)
            .unwrap_or(None)
            .is_some()
        {
            linked += 1;
        }
    }
    let mut summary = format!(
        "vault ready at {}; scripts installed: {scripts}; SessionEnd hook: {}; cron: {}; graphify links: {linked}",
        vault.display(),
        if hook_added {
            "added"
        } else {
            "already present"
        },
        if cron_added {
            "daily 22:00"
        } else {
            "skipped/unavailable"
        }
    );
    if scripts == 0 {
        summary.push_str("; note: memory scripts not found in env catalog");
    }
    Ok(summary)
}

/// Whether the hook is already registered (used by the env check probe).
pub fn session_autosave_registered() -> bool {
    let path = home_dir().join(".claude").join("settings.json");
    std::fs::read_to_string(&path)
        .map(|t| t.contains("session_autosave"))
        .unwrap_or(false)
}
