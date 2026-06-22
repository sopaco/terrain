use terrain_core::SourceCitation;
use serde::Serialize;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "lowercase"))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatToolCallStatus {
    Running,
    Ok,
    Error,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename = "ToolCallRecord"))]
#[derive(Debug, Clone, Serialize)]
pub struct ChatToolCallRecord {
    pub id: String,
    pub name: String,
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown>"))]
    pub arguments: serde_json::Value,
    #[cfg_attr(feature = "ts-export", ts(type = "unknown"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub status: ChatToolCallStatus,
    /// Unix epoch milliseconds when the tool call started.
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub started_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub completed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub duration_ms: Option<u64>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename = "TokenUsage"))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ChatTokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub estimated: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatPhase {
    Thinking,
    Tools,
    Generating,
    Streaming,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename = "AskKnowledgeReply"))]
#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub answer: String,
    pub citations: Vec<SourceCitation>,
    pub tool_calls: Vec<ChatToolCallRecord>,
    pub usage: ChatTokenUsage,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub completed_at: u64,
}
