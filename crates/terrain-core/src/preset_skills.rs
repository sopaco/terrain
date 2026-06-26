//! Resolve and deploy Terrain preset skills (Litho, SDD, Ask, …).
//!
//! Compile-time `CARGO_MANIFEST_DIR` only works in the Terrain source tree. Packaged
//! apps and downstream crates must resolve skills at runtime from app resources or
//! `~/.terrain/preset_skills/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::error::{CoreError, Result};

static ROOT: OnceLock<PathBuf> = OnceLock::new();

const LITHO_SKILL: &str = "litho-documents-skill";
const SDD_SKILL: &str = "sdd-workflow-skill";
const ASK_SKILL: &str = "terrain-ask-skill";
const AGENT_ARCH_SKILL: &str = "agent-architecture-skill";

/// Inject preset skills root (Tauri resource dir or dev discovery). Call once at app startup.
pub fn init_preset_skills_root(path: PathBuf) {
    if path.join(LITHO_SKILL).join("SKILL.md").is_file() {
        let _ = ROOT.set(path);
    }
}

/// Ensure a root is selected when the app has not called [`init_preset_skills_root`].
pub fn ensure_preset_skills_initialized() {
    if ROOT.get().is_some() {
        return;
    }
    if let Ok(raw) = std::env::var("TERRAIN_PRESET_SKILLS") {
        let path = PathBuf::from(raw);
        if path.join(LITHO_SKILL).join("SKILL.md").is_file() {
            let _ = ROOT.set(path);
            return;
        }
    }
    let home = user_preset_skills_dir();
    if home.join(LITHO_SKILL).join("SKILL.md").is_file() {
        let _ = ROOT.set(home);
        return;
    }
    if let Some(path) = discover_preset_skills_runtime() {
        let _ = ROOT.set(path);
    }
}

pub fn preset_skills_root() -> PathBuf {
    ensure_preset_skills_initialized();
    ROOT.get()
        .cloned()
        .or_else(discover_preset_skills_runtime)
        .unwrap_or_else(fallback_dev_preset_skills_root)
}

pub fn preset_skill_dir(name: &str) -> PathBuf {
    preset_skills_root().join(name)
}

pub fn resolve_preset_skill_dir(name: &str) -> Option<PathBuf> {
    let dir = preset_skill_dir(name);
    if dir.join("SKILL.md").is_file() {
        Some(dir)
    } else {
        None
    }
}

pub fn default_litho_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("TERRAIN_LITHO_SKILL") {
        return PathBuf::from(path);
    }
    resolve_preset_skill_dir(LITHO_SKILL).unwrap_or_else(|| preset_skill_dir(LITHO_SKILL))
}

pub fn resolve_litho_skill_dir() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("TERRAIN_LITHO_SKILL") {
        let p = PathBuf::from(path);
        if p.join("SKILL.md").is_file() {
            return Some(p);
        }
    }
    resolve_preset_skill_dir(LITHO_SKILL)
}

pub fn default_sdd_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("TERRAIN_SDD_SKILL") {
        return PathBuf::from(path);
    }
    resolve_preset_skill_dir(SDD_SKILL).unwrap_or_else(|| preset_skill_dir(SDD_SKILL))
}

pub fn resolve_sdd_skill_dir() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("TERRAIN_SDD_SKILL") {
        let p = PathBuf::from(path);
        if p.join("SKILL.md").is_file() {
            return Some(p);
        }
    }
    resolve_preset_skill_dir(SDD_SKILL)
}

pub fn default_agent_arch_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("TERRAIN_AGENT_ARCH_SKILL") {
        return PathBuf::from(path);
    }
    resolve_preset_skill_dir(AGENT_ARCH_SKILL).unwrap_or_else(|| preset_skill_dir(AGENT_ARCH_SKILL))
}

pub fn default_ask_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("TERRAIN_ASK_SKILL") {
        return PathBuf::from(path);
    }
    resolve_preset_skill_dir(ASK_SKILL).unwrap_or_else(|| preset_skill_dir(ASK_SKILL))
}

pub fn user_preset_skills_dir() -> PathBuf {
    user_home()
        .map(|h| h.join(".terrain/preset_skills"))
        .unwrap_or_else(|| PathBuf::from(".terrain/preset_skills"))
}

/// Symlink bundled preset skills into `~/.terrain/preset_skills/` for CLI / external agents.
pub fn deploy_preset_skills_to_home() -> Result<PathBuf> {
    ensure_preset_skills_initialized();
    let src = ROOT
        .get()
        .cloned()
        .or_else(discover_preset_skills_runtime)
        .ok_or_else(|| {
            CoreError::InvalidDoc(
                "Terrain preset skills not found (bundle or dev tree)".into(),
            )
        })?;

    let dest = user_preset_skills_dir();
    fs::create_dir_all(&dest)?;

    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let from = entry.path();
        if !from.join("SKILL.md").is_file() {
            continue;
        }
        let link = dest.join(name);
        symlink_replace(&link, &from)?;
    }

    if !dest.join(LITHO_SKILL).join("SKILL.md").is_file() {
        return Err(CoreError::InvalidDoc(
            "failed to deploy Litho preset skill to ~/.terrain/preset_skills/".into(),
        ));
    }

    Ok(dest)
}

/// Discover preset skills next to the running executable (Tauri `.app` resources).
pub fn discover_preset_skills_runtime() -> Option<PathBuf> {
    discover_next_to_exe().or_else(discover_dev_preset_skills)
}

fn discover_next_to_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let parent = exe.parent()?;
    let candidates = [
        parent.join("../Resources/preset_skills"),
        parent.join("Resources/preset_skills"),
        parent.join("resources/preset_skills"),
        parent.join("../resources/preset_skills"),
        parent.join("preset_skills"),
        parent.join("../preset_skills"),
    ];
    for candidate in candidates {
        if candidate.join(LITHO_SKILL).join("SKILL.md").is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn discover_dev_preset_skills() -> Option<PathBuf> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "../../preset_skills",
        "../../../preset_skills",
        "../preset_skills",
        "preset_skills",
    ] {
        let candidate = base.join(rel);
        if candidate.join(LITHO_SKILL).join("SKILL.md").is_file() {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn fallback_dev_preset_skills_root() -> PathBuf {
    discover_dev_preset_skills().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../preset_skills")
    })
}

fn symlink_replace(link: &Path, target: &Path) -> Result<()> {
    if link.symlink_metadata().is_ok() {
        fs::remove_file(link).or_else(|_| fs::remove_dir_all(link))?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        CoreError::InvalidDoc(format!(
            "symlink {} -> {}: {e}",
            link.display(),
            target.display()
        ))
    })?;
    #[cfg(not(unix))]
    {
        if target.is_dir() {
            copy_dir_recursive(target, link)?;
        } else {
            fs::copy(target, link)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_discovery_finds_terrain_preset_skills() {
        let found = discover_dev_preset_skills();
        assert!(
            found.is_some(),
            "expected preset_skills under Terrain workspace"
        );
        let root = found.unwrap();
        assert!(root.join(LITHO_SKILL).join("SKILL.md").is_file());
    }
}
