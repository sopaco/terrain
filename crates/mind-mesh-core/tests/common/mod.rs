use std::path::PathBuf;

use mind_mesh_core::{KnowledgePaths, registry};

pub struct TestKnowledgeSetup {
    pub paths: KnowledgePaths,
    pub slug: String,
    pub repo: PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
    _registry_dir: tempfile::TempDir,
}

impl TestKnowledgeSetup {
    pub fn new(slug: &str) -> Self {
        let lock = registry::registry_test_lock();
        let registry_dir = tempfile::tempdir().expect("temp registry dir");
        let registry_file = registry_dir.path().join("registry.json");
        unsafe {
            std::env::set_var("MIND_MESH_REGISTRY_FILE", &registry_file);
        }

        let repo_dir = registry_dir.path().join("repo");
        std::fs::create_dir_all(&repo_dir).expect("create test repo");

        registry::register_project(slug, &repo_dir.display().to_string()).expect("register");

        let paths = KnowledgePaths::new();
        paths
            .ensure_project_layout(slug)
            .expect("ensure project layout");

        Self {
            paths,
            slug: slug.to_string(),
            repo: repo_dir,
            _lock: lock,
            _registry_dir: registry_dir,
        }
    }
}
