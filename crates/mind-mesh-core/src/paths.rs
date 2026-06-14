use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};
use crate::registry;
use crate::schema::DocType;

/// Registry-backed resolver for per-repository knowledge at `{repo}/.mind-mesh/`.
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

    /// Knowledge root inside a repository (`.mind-mesh/`).
    pub fn for_repo(repo_path: impl AsRef<Path>) -> Self {
        Self::with_workspace_repo(repo_path.as_ref().to_path_buf())
    }

    pub fn workspace_repo(&self) -> Option<&Path> {
        self.workspace_repo.as_deref()
    }

    /// `{workspace_repo}/.mind-mesh` when a workspace repo is set.
    pub fn workspace_knowledge_root(&self) -> Option<PathBuf> {
        self.workspace_repo
            .as_ref()
            .map(|repo| registry::knowledge_root_for_repo(repo))
    }

    /// Resolve workspace repo from `MIND_MESH_REPO_PATH` or the nearest Git root from cwd.
    pub fn resolve_workspace_repo() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("MIND_MESH_REPO_PATH") {
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

    /// Knowledge root for a registered project slug (`{repo}/.mind-mesh/`).
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

    /// `~/.mind-mesh/debug` — scratch files for inspecting model output.
    pub fn debug_dir() -> PathBuf {
        dirs_home().join(".mind-mesh").join("debug")
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

    /// SDD workflow workspace (`.sdd-agent/` inside project knowledge).
    pub fn sdd_workspace_dir(&self, project_slug: &str) -> PathBuf {
        self.project_dir(project_slug).join(".sdd-agent")
    }

    /// SDD phase outputs (`1.requirements.md`, …).
    pub fn sdd_output_dir(&self, project_slug: &str) -> PathBuf {
        self.sdd_workspace_dir(project_slug).join("outputs")
    }

    pub fn preset_skills_root() -> PathBuf {
        if let Ok(root) = std::env::var("MIND_MESH_PRESET_SKILLS") {
            return PathBuf::from(root);
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../preset_skills")
    }

    pub fn ensure_layout(&self) -> std::io::Result<()> {
        let registry_dir = registry::registry_dir();
        std::fs::create_dir_all(&registry_dir)?;
        std::fs::create_dir_all(Self::debug_dir())?;
        Ok(())
    }

    /// Knowledge root for Ask/ACP env vars — prefers an explicit project slug.
    pub fn knowledge_root_for(&self, project_slug: Option<&str>) -> Option<PathBuf> {
        if let Some(slug) = project_slug {
            if let Ok(root) = self.try_project_dir(slug) {
                return Some(root);
            }
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
        Ok(())
    }
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
