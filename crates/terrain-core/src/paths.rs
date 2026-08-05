use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};
use crate::registry;
use crate::schema::DocType;

/// Registry-backed resolver for per-repository knowledge at `{repo}/.terrain/`.
#[derive(Debug, Clone, Default)]
pub struct KnowledgePaths {
    /// When set, CLI and workspace tools default to this repository.
    workspace_repo: Option<PathBuf>,
}

impl KnowledgePaths {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_workspace_repo(repo: impl Into<PathBuf>) -> Self {
        Self {
            workspace_repo: Some(repo.into()),
        }
    }

    /// Knowledge root inside a repository (`.terrain/`).
    pub fn for_repo(repo_path: impl AsRef<Path>) -> Self {
        Self::with_workspace_repo(repo_path.as_ref().to_path_buf())
    }

    pub fn workspace_repo(&self) -> Option<&Path> {
        self.workspace_repo.as_deref()
    }

    /// `{workspace_repo}/.terrain` when a workspace repo is set.
    pub fn workspace_knowledge_root(&self) -> Option<PathBuf> {
        self.workspace_repo
            .as_ref()
            .map(|repo| registry::knowledge_root_for_repo(repo))
    }

    /// Resolve workspace repo from `TERRAIN_REPO_PATH` or the nearest Git root from cwd.
    pub fn resolve_workspace_repo() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("TERRAIN_REPO_PATH") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }
        let start = std::env::current_dir().ok()?;
        find_git_repo_root(&start)
    }

    /// Build paths scoped to the current workspace repository when discoverable.
    pub fn from_workspace() -> Self {
        Self::resolve_workspace_repo()
            .map(Self::with_workspace_repo)
            .unwrap_or_default()
    }

    /// Knowledge root for a registered project slug (`{repo}/.terrain/`).
    pub fn try_project_dir(&self, project_slug: &str) -> Result<PathBuf> {
        registry::knowledge_root_for_slug(project_slug).ok_or_else(|| {
            CoreError::ProjectNotFound(format!(
                "project '{project_slug}' is not registered; add the repository first"
            ))
        })
    }

    pub fn project_dir(&self, project_slug: &str) -> PathBuf {
        self.try_project_dir(project_slug)
            .unwrap_or_else(|e| panic!("{e}"))
    }

    /// All registered project knowledge roots that have an index file.
    pub fn indexed_project_roots(&self) -> HashMap<String, PathBuf> {
        registry::indexed_project_roots().unwrap_or_default()
    }

    /// `~/.terrain/debug` — scratch files for inspecting model output.
    pub fn debug_dir() -> PathBuf {
        dirs_home().join(".terrain").join("debug")
    }

    pub fn write_debug_file(&self, name: &str, content: &str) {
        let dir = Self::debug_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let _ = std::fs::write(dir.join(name), content);
    }

    pub fn project_index(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join("index.md")
    }

    pub fn doc_path(&self, project_slug: &str, doc_type: DocType, slug: &str) -> PathBuf {
        let base = self.project_dir(project_slug);
        match doc_type.subdir() {
            Some(sub) => base.join(sub).join(format!("{slug}.md")),
            None => base.join("index.md"),
        }
    }

    pub fn sync_meta_path(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".meta/sync.json")
    }

    pub fn freshness_meta_path(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".meta/freshness.json")
    }

    /// Sidecar for the Litho `human/` doc set — kept out of `human/` so it is not listed as a doc.
    pub fn human_docs_meta_path(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".meta/human.json")
    }

    /// Human-editable project remark (versioned with the repo).
    pub fn project_note_path(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join("project-note.md")
    }

    /// Human-editable project remark when scoped to a workspace repository.
    pub fn workspace_project_note_path(&self) -> Option<PathBuf> {
        self.workspace_knowledge_root()
            .map(|root| root.join("project-note.md"))
    }

    /// Human-facing Litho docs (`1.概述.md`, `2.架构.md`, …).
    pub fn human_docs_dir(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join("human")
    }

    /// Agent-facing Repomix pack and metadata.
    pub fn agent_pack_dir(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join("agent")
    }

    pub fn agent_pack_main(&self, project_slug: &str) -> PathBuf {
        self.agent_pack_dir(project_slug).join("repomix.md")
    }

    pub fn agent_pack_meta(&self, project_slug: &str) -> PathBuf {
        self.agent_pack_dir(project_slug).join("meta.json")
    }

    /// Agent-facing architecture narrative (`context.md`) — not source code.
    pub fn agent_context_main(&self, project_slug: &str) -> PathBuf {
        self.agent_pack_dir(project_slug).join("context.md")
    }

    pub fn agent_context_meta(&self, project_slug: &str) -> PathBuf {
        self.agent_pack_dir(project_slug).join("context-meta.json")
    }

    /// Litho skill intermediate workspace (`.litho-agent/` inside project knowledge).
    pub fn litho_workspace_dir(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".litho-agent")
    }

    /// Local SDD root (`~/.terrain/sdd/{slug}/`) — not versioned with the repo.
    pub fn sdd_local_root(&self, project_slug: &str) -> PathBuf {
        registry::registry_dir().join("sdd").join(project_slug)
    }

    pub fn sdd_sessions_dir(&self, project_slug: &str) -> PathBuf {
        self.sdd_local_root(project_slug).join("sessions")
    }

    pub fn sdd_active_session_path(&self, project_slug: &str) -> PathBuf {
        self.sdd_local_root(project_slug).join("active.json")
    }

    /// SDD workflow workspace for one parallel workstream (local, per session).
    pub fn sdd_workspace_dir(&self, project_slug: &str, session_id: &str) -> PathBuf {
        self.sdd_sessions_dir(project_slug).join(session_id)
    }

    /// SDD phase outputs (`1.requirements.md`, …) for a session.
    pub fn sdd_output_dir(&self, project_slug: &str, session_id: &str) -> PathBuf {
        self.sdd_workspace_dir(project_slug, session_id).join("outputs")
    }

    /// Legacy in-repo SDD path (migrated away; kept for gitignore / drift exclusion).
    pub fn sdd_legacy_workspace_dir(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".sdd-agent")
    }

    /// Local Ask history root (`~/.terrain/ask/{slug}/`) — not versioned with the repo.
    pub fn ask_local_root(&self, project_slug: &str) -> PathBuf {
        registry::registry_dir().join("ask").join(project_slug)
    }

    pub fn ask_sessions_dir(&self, project_slug: &str) -> PathBuf {
        self.ask_local_root(project_slug).join("sessions")
    }

    pub fn ask_active_session_path(&self, project_slug: &str) -> PathBuf {
        self.ask_local_root(project_slug).join("active.json")
    }

    pub fn ask_workspace_dir(&self, project_slug: &str, session_id: &str) -> PathBuf {
        self.ask_sessions_dir(project_slug).join(session_id)
    }

    /// True when `path` is under the local SDD store (`~/.terrain/sdd/`).
    pub fn is_sdd_local_path(&self, path: &Path) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let root = registry::registry_dir().join("sdd");
        let root = root.canonicalize().unwrap_or(root);
        canonical.starts_with(&root)
    }

    pub fn preset_skills_root() -> PathBuf {
        crate::preset_skills::preset_skills_root()
    }

    pub fn ensure_layout(&self) -> std::io::Result<()> {
        let registry_dir = registry::registry_dir();
        std::fs::create_dir_all(&registry_dir)?;
        std::fs::create_dir_all(Self::debug_dir())?;
        Ok(())
    }

    /// Knowledge root for Ask/ACP env vars — prefers an explicit project slug.
    pub fn knowledge_root_for(&self, project_slug: Option<&str>) -> Option<PathBuf> {
        if let Some(slug) = project_slug
            && let Ok(root) = self.try_project_dir(slug) {
                return Some(root);
            }
        self.workspace_knowledge_root()
    }

    pub fn ensure_project_layout(&self, project_slug: &str) -> std::io::Result<()> {
        let base = self.project_dir(project_slug);
        std::fs::create_dir_all(base.join(".meta"))?;
        std::fs::create_dir_all(base.join("modules"))?;
        std::fs::create_dir_all(base.join("interfaces"))?;
        std::fs::create_dir_all(base.join("routes"))?;
        std::fs::create_dir_all(base.join("events"))?;
        std::fs::create_dir_all(base.join("human"))?;
        std::fs::create_dir_all(base.join(".litho-agent"))?;
        std::fs::create_dir_all(base.join("agent"))?;
        std::fs::create_dir_all(base.join("knowledge"))?;
        std::fs::create_dir_all(base.join("env"))?;
        // Nested .gitignore / .gitattributes travel with the directory, so every
        // project gets the asset-tracking policy without running env integration.
        crate::git_policy::ensure_git_policy(&base)?;
        Ok(())
    }
}

/// Normalize a citation path to knowledge-root-relative form (no `.terrain/` prefix).
pub fn normalize_knowledge_ref(file_path: &str) -> String {
    let p = file_path.trim().trim_start_matches("./").trim_start_matches('/');
    if p == "context.md" {
        return "agent/context.md".into();
    }
    if let Some(rest) = p.strip_prefix(".terrain/") {
        return rest.to_string();
    }
    if let Some(idx) = p.find("/.terrain/") {
        return p[idx + "/.terrain/".len()..].to_string();
    }
    p.to_string()
}

/// Non-markdown assets under `.terrain/` that should be read from the knowledge root,
/// not from the live repo or Repomix agent pack (e.g. `.meta/freshness.json`).
pub fn is_terrain_knowledge_asset_path(file_path: &str) -> bool {
    let p = normalize_knowledge_ref(file_path);
    if p == "agent/repomix.md" || p.ends_with("/agent/repomix.md") {
        return false;
    }
    p.starts_with(".meta/")
        || p.starts_with("env/")
        || (p.starts_with("agent/") && !p.ends_with(".md"))
}

/// Git/repo paths that are Terrain-generated knowledge outputs.
/// Excluded from source-drift signals (dirty tree, baseline file diff).
pub fn is_knowledge_output_path(path: &str) -> bool {
    let p = path.trim().trim_start_matches("./");
    if matches!(p, ".terrain" | ".litho-agent" | ".sdd-agent") {
        return true;
    }
    p.starts_with(".terrain/")
        || p.contains("/.terrain/")
        || p.starts_with(".litho-agent/")
        || p.contains("/.litho-agent/")
        || p.starts_with(".sdd-agent/")
        || p.contains("/.sdd-agent/")
}

fn find_git_repo_root(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
