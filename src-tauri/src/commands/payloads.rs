use terrain_agent::{LithoGenerationResult, ProjectInitResult};
use terrain_core::{EnvApplyResult, SddPhase, SddPhaseResult};
use serde::Serialize;

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
