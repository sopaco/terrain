use serde::Serialize;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressKind {
    ProjectInit,
    Scan,
    HumanDocs,
    AgentContext,
    Litho,
    Sdd,
    Env,
    QuickRefresh,
    Done,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub kind: ProgressKind,
    pub stage: String,
    pub message: String,
}

impl ProgressEvent {
    pub fn new(kind: ProgressKind, stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            stage: stage.into(),
            message: message.into(),
        }
    }

    pub fn project_init(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ProgressKind::ProjectInit, stage, message)
    }

    pub fn litho(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ProgressKind::Litho, stage, message)
    }

    pub fn sdd(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ProgressKind::Sdd, stage, message)
    }

    pub fn env(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ProgressKind::Env, stage, message)
    }
}

/// Backward-compatible aliases for workflow callbacks.
pub type ProjectInitProgress = ProgressEvent;
pub type LithoProgress = ProgressEvent;
pub type SddProgress = ProgressEvent;
pub type EnvApplyProgress = ProgressEvent;
