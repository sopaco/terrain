//! Document frontmatter and metadata types.

use serde::{Deserialize, Serialize};

/// Top-level document kinds stored under `{repo}/.terrain/`.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "lowercase"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    Project,
    Module,
    Interface,
    Route,
    Event,
    /// Litho-generated narrative docs under `human/`.
    Human,
    /// Agent architecture narrative under `agent/context.md`.
    #[serde(rename = "agent_context")]
    AgentContext,
}

impl DocType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Module => "module",
            Self::Interface => "interface",
            Self::Route => "route",
            Self::Event => "event",
            Self::Human => "human",
            Self::AgentContext => "agent_context",
        }
    }

    pub fn subdir(self) -> Option<&'static str> {
        match self {
            Self::Project => None,
            Self::Module => Some("modules"),
            Self::Interface => Some("interfaces"),
            Self::Route => Some("routes"),
            Self::Event => Some("events"),
            Self::Human => Some("human"),
            Self::AgentContext => None,
        }
    }
}

/// Shared frontmatter fields across all knowledge documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFrontmatter {
    #[serde(rename = "type")]
    pub doc_type: DocType,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub repo_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tech_stack: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceMeta {
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMeta {
    pub uri: String,
    pub handler: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub middleware: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeta {
    pub event_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMeta {
    pub project: String,
    pub repo_path: String,
    pub synced_at: String,
    pub collectors: Vec<String>,
}

/// Sidecar for the Litho `human/` doc set — the baseline an incremental update diffs against.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDocsMeta {
    pub project: String,
    pub repo_path: String,
    pub generated_at: String,
    pub doc_count: usize,
    /// Git HEAD when the doc set was last written; `None` for non-Git repositories.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_git_head: Option<String>,
    /// `full` for a from-scratch pipeline run, `incremental` for a diff-driven update.
    pub last_run_mode: String,
    /// Language the doc set was generated in (`zh-CN` / `en`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}
