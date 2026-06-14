use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::registry;
use crate::schema::DocType;

/// Root layout for the Markdown knowledge base.
#[derive(Debug, Clone)]
pub struct KnowledgePaths {
    root: PathBuf,
}

impl KnowledgePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_home() -> Self {
        Self::new(dirs_home().join(".mind-mesh/knowledge"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    pub fn meta_dir(&self) -> PathBuf {
        self.root.join(".meta")
    }

    /// `~/.mind-mesh/debug` — scratch files for inspecting model output.
    pub fn debug_dir(&self) -> PathBuf {
        self.root
            .parent()
            .map(|parent| parent.join("debug"))
            .unwrap_or_else(|| self.root.join(".debug"))
    }

    pub fn write_debug_file(&self, name: &str, content: &str) {
        let dir = self.debug_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let _ = std::fs::write(dir.join(name), content);
    }

    pub fn project_dir(&self, project_slug: &str) -> PathBuf {
        registry::knowledge_root_for_slug(project_slug)
            .unwrap_or_else(|| self.projects_dir().join(project_slug))
    }

    /// All indexed project knowledge roots: registry repos first, then legacy global dirs.
    pub fn indexed_project_roots(&self) -> HashMap<String, PathBuf> {
        let mut roots = registry::indexed_project_roots().unwrap_or_default();
        if let Ok(entries) = std::fs::read_dir(self.projects_dir()) {
            for entry in entries.flatten() {
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let slug = entry.file_name().to_string_lossy().into_owned();
                if roots.contains_key(&slug) {
                    continue;
                }
                let index = self.projects_dir().join(&slug).join("index.md");
                if index.is_file() {
                    roots.insert(slug, entry.path());
                }
            }
        }
        roots
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
        let local = self.project_dir(project_slug).join(".meta/sync.json");
        if local.is_file() {
            return local;
        }
        let legacy = self.meta_dir().join(format!("{project_slug}-sync.json"));
        if legacy.is_file() {
            return legacy;
        }
        local
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
        std::fs::create_dir_all(self.projects_dir())?;
        std::fs::create_dir_all(self.meta_dir())?;
        Ok(())
    }

    pub fn ensure_project_layout(&self, project_slug: &str) -> std::io::Result<()> {
        let base = self.project_dir(project_slug);
        std::fs::create_dir_all(base.join(".meta"))?;
        std::fs::create_dir_all(base.join("modules"))?;
        std::fs::create_dir_all(base.join("interfaces"))?;
        std::fs::create_dir_all(base.join("routes"))?;
        std::fs::create_dir_all(base.join("events"))?;
        std::fs::create_dir_all(base.join("human"))?;
        std::fs::create_dir_all(base.join("agent"))?;
        std::fs::create_dir_all(base.join("knowledge"))?;
        std::fs::create_dir_all(base.join("env"))?;
        Ok(())
    }

    /// Knowledge root inside a repository (`.mind-mesh/`).
    pub fn for_repo(repo_path: impl AsRef<Path>) -> Self {
        Self::new(registry::knowledge_root_for_repo(repo_path.as_ref()))
    }
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
