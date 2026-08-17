//! SDD workflow and session types.

use serde::{Deserialize, Serialize};

/// SDD standardized workflow phases.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SddPhase {
    Requirements,
    TechDesign,
    CodeGen,
    CodeReview,
}

impl SddPhase {
    pub fn all() -> [Self; 4] {
        [
            Self::Requirements,
            Self::TechDesign,
            Self::CodeGen,
            Self::CodeReview,
        ]
    }

    pub fn label(self) -> &'static str {
        let lang = crate::language::current_language();
        match self {
            Self::Requirements => lang.tr("需求澄清", "Requirements"),
            Self::TechDesign => lang.tr("技术方案", "Tech Design"),
            Self::CodeGen => lang.tr("代码生成", "Code Generation"),
            Self::CodeReview => lang.tr("代码审查", "Code Review"),
        }
    }

    pub fn output_filename(self) -> &'static str {
        match self {
            Self::Requirements => "1.requirements.md",
            Self::TechDesign => "2.tech-design.md",
            Self::CodeGen => "3.implementation.md",
            Self::CodeReview => "4.code-review.md",
        }
    }

    pub fn order(self) -> u8 {
        match self {
            Self::Requirements => 0,
            Self::TechDesign => 1,
            Self::CodeGen => 2,
            Self::CodeReview => 3,
        }
    }
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskSessionInfo {
    pub id: String,
    pub title: String,
    /// Last assistant reply date (`YYYY-MM-DD`).
    pub last_replied_at: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddSessionInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPlan {
    pub project_slug: String,
    pub session_id: String,
    pub repo_path: String,
    pub skill_dir: String,
    pub sdd_workspace_dir: String,
    pub sdd_output_dir: String,
    pub human_output_dir: String,
    pub agent_pack_path: String,
    pub skill_ready: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPhaseInfo {
    pub phase: SddPhase,
    pub label: String,
    pub output_path: String,
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddStatus {
    pub project_slug: String,
    pub skill_ready: bool,
    pub workspace_dir: String,
    pub output_dir: String,
    pub phases: Vec<SddPhaseInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_phase: Option<SddPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    pub sessions: Vec<SddSessionInfo>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SddPhaseResult {
    pub phase: SddPhase,
    pub output_path: String,
    pub response_excerpt: String,
}
