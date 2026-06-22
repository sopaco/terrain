use async_trait::async_trait;

use crate::agent_assets::AgentAssetsEnsureReport;

/// Ensures agent knowledge assets exist (repomix pack + agent/context.md).
#[async_trait]
pub trait AgentContextGenerator: Send + Sync {
    async fn ensure_ready(
        &self,
        project_slug: &str,
        repo_path: Option<&str>,
    ) -> anyhow::Result<AgentAssetsEnsureReport>;
}
