//! Citation and source slice types.

use serde::{Deserialize, Serialize};

/// Citation attached to a DeepWiki-style Q&A reply.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CitationKind {
    HumanDoc,
    StructuredDoc,
    SourceCode,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCitation {
    pub kind: CitationKind,
    pub title: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDocEntry {
    pub path: String,
    pub title: String,
    pub relative_path: String,
    /// Tree section: `human` (Litho docs) or `agent` (architecture context, etc.).
    #[serde(default = "default_human_section")]
    pub section: String,
}

fn default_human_section() -> String {
    "human".into()
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename = "IpcSourceSlice"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSlice {
    pub repo_path: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}
