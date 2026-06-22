use terrain_agent::{
    ChatPhase, ChatTokenUsage, ChatToolCallRecord, LithoGenerationResult, ProjectInitResult,
};
use terrain_core::{EnvApplyResult, SddPhase, SddPhaseResult, SourceCitation};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) struct ChatChunkPayload {
    pub session_id: String,
    pub text: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatToolCallsPayload {
    pub session_id: String,
    pub tool_calls: Vec<ChatToolCallRecord>,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatPhasePayload {
    pub session_id: String,
    pub phase: ChatPhase,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatUsagePayload {
    pub session_id: String,
    pub usage: ChatTokenUsage,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatDonePayload {
    pub session_id: String,
    pub answer: String,
    pub citations: Vec<SourceCitation>,
    pub tool_calls: Vec<ChatToolCallRecord>,
    pub usage: ChatTokenUsage,
    pub completed_at: u64,
}

#[derive(Clone, Serialize)]
pub(crate) struct LithoProgressPayload {
    pub project_slug: String,
    pub stage: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct LithoDonePayload {
    pub project_slug: String,
    pub result: LithoGenerationResult,
}

#[derive(Clone, Serialize)]
pub(crate) struct SddProgressPayload {
    pub project_slug: String,
    pub phase: SddPhase,
    pub stage: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct SddDonePayload {
    pub project_slug: String,
    pub result: SddPhaseResult,
}

#[derive(Clone, Serialize)]
pub(crate) struct ProjectInitProgressPayload {
    pub project_slug: String,
    pub stage: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct ProjectInitDonePayload {
    pub project_slug: String,
    pub result: ProjectInitResult,
}

#[derive(Clone, Serialize)]
pub(crate) struct EnvOptProgressPayload {
    pub repo_path: String,
    pub stage: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct EnvOptDonePayload {
    pub repo_path: String,
    pub result: EnvApplyResult,
}
