mod git;
mod openapi;

pub use git::GitScanner;
pub use openapi::OpenApiImporter;

use chrono::Utc;
use std::path::Path;

use crate::doc::write_json;
use crate::error::Result;
use crate::path_portable::stored_repo_path;
use crate::paths::KnowledgePaths;
use crate::registry;
use crate::schema::SyncMeta;

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanReport {
    pub project_slug: String,
    pub files_written: usize,
    pub collectors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_pack: Option<AgentPackSummary>,
}

    #[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
    #[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentPackSummary {
    pub total_files: usize,
    pub total_tokens: usize,
    pub output_path: String,
    #[serde(default)]
    pub pack_skipped: bool,
}

pub struct ProjectScanner {
    paths: KnowledgePaths,
}

impl ProjectScanner {
    pub fn new(paths: KnowledgePaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &KnowledgePaths {
        &self.paths
    }

    pub async fn scan_repo(&self, repo_path: &str, project_slug: Option<&str>) -> Result<ScanReport> {
        let slug = project_slug
            .map(str::to_string)
            .unwrap_or_else(|| slug::slugify(repo_path.rsplit('/').next().unwrap_or("project")));

        registry::register_project(&slug, repo_path)?;

        self.paths.ensure_layout()?;
        self.paths.ensure_project_layout(&slug)?;

        let mut files_written = 0;
        let mut collectors = Vec::new();

        let git = GitScanner::new(&self.paths, &slug);
        files_written += git.scan(repo_path)?;
        collectors.push("git".into());

        if let Some(count) = OpenApiImporter::new(&self.paths, &slug).import_repo(repo_path)? {
            if count > 0 {
                files_written += count;
                collectors.push("openapi".into());
            }
        }

        #[cfg(feature = "repomix")]
        let agent_pack = {
            let pack = crate::assets::maybe_pack_agent_assets(&self.paths, &slug, repo_path).await?;
            if !pack.skipped {
                collectors.push("repomix".into());
            }
            Some(AgentPackSummary {
                total_files: pack.total_files,
                total_tokens: pack.total_tokens,
                output_path: pack.output_path,
                pack_skipped: pack.skipped,
            })
        };

        #[cfg(not(feature = "repomix"))]
        let agent_pack = None;

        let sync = SyncMeta {
            project: slug.clone(),
            repo_path: stored_repo_path(Path::new(repo_path)),
            synced_at: Utc::now().to_rfc3339(),
            collectors: collectors.clone(),
        };
        let sync_path = self.paths.sync_meta_path(&slug);
        if let Some(parent) = sync_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_json(sync_path, &sync)?;

        Ok(ScanReport {
            project_slug: slug,
            files_written,
            collectors,
            agent_pack,
        })
    }
}
