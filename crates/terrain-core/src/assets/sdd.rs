use std::path::{Path, PathBuf};

use crate::paths::KnowledgePaths;
use crate::preset_skills::{default_sdd_skill_dir, resolve_sdd_skill_dir};
use crate::schema::{SddPhase, SddPhaseInfo, SddPlan, SddSessionInfo, SddStatus};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ActiveSessionFile {
    session_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SessionMetaFile {
    id: String,
    title: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

fn session_meta_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir.join(session_id).join("meta.json")
}

pub fn new_session_id(title: &str) -> String {
    let base = slug::slugify(title.trim());
    let base = if base.is_empty() { "sdd".to_string() } else { base };
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    format!("{base}-{ts}")
}

pub fn list_sdd_sessions(paths: &KnowledgePaths, project_slug: &str) -> Vec<SddSessionInfo> {
    let sessions_dir = paths.sdd_sessions_dir(project_slug);
    if !sessions_dir.is_dir() {
        return Vec::new();
    }
    let mut sessions = Vec::new();
    let Ok(entries) = std::fs::read_dir(&sessions_dir) else {
        return sessions;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let meta_path = session_meta_path(&sessions_dir, &id);
        if let Ok(raw) = std::fs::read_to_string(&meta_path)
            && let Ok(meta) = serde_json::from_str::<SessionMetaFile>(&raw) {
                sessions.push(SddSessionInfo {
                    id: meta.id,
                    title: meta.title,
                    created_at: meta.created_at,
                    updated_at: meta.updated_at,
                });
                continue;
            }
        sessions.push(SddSessionInfo {
            id: id.clone(),
            title: id,
            created_at: String::new(),
            updated_at: None,
        });
    }
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sessions
}

pub fn get_active_sdd_session(paths: &KnowledgePaths, project_slug: &str) -> Option<String> {
    let path = paths.sdd_active_session_path(project_slug);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<ActiveSessionFile>(&raw)
        .ok()
        .map(|f| f.session_id)
}

pub fn set_active_sdd_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<()> {
    let session_dir = paths.sdd_workspace_dir(project_slug, session_id);
    if !session_dir.is_dir() {
        return Err(crate::error::CoreError::InvalidDoc(format!(
            "SDD session not found: {session_id}"
        )));
    }
    let root = paths.sdd_local_root(project_slug);
    std::fs::create_dir_all(&root)?;
    let active = ActiveSessionFile {
        session_id: session_id.to_string(),
    };
    let raw = serde_json::to_string_pretty(&active)?;
    std::fs::write(paths.sdd_active_session_path(project_slug), raw)?;
    Ok(())
}

pub fn create_sdd_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    title: &str,
) -> crate::error::Result<SddSessionInfo> {
    let title = title.trim();
    let title = if title.is_empty() {
        "新需求"
    } else {
        title
    };
    let id = new_session_id(title);
    let session_dir = paths.sdd_workspace_dir(project_slug, &id);
    std::fs::create_dir_all(session_dir.join("outputs"))?;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let meta = SessionMetaFile {
        id: id.clone(),
        title: title.to_string(),
        created_at: now.clone(),
        updated_at: None,
    };
    let meta_path = session_meta_path(&paths.sdd_sessions_dir(project_slug), &id);
    std::fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;
    set_active_sdd_session(paths, project_slug, &id)?;
    Ok(SddSessionInfo {
        id,
        title: title.to_string(),
        created_at: now,
        updated_at: None,
    })
}

/// Delete an SDD session and all associated local files under `~/.terrain/sdd/`.
pub fn delete_sdd_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<()> {
    let session_dir = paths.sdd_workspace_dir(project_slug, session_id);
    if !session_dir.is_dir() {
        return Err(crate::error::CoreError::InvalidDoc(format!(
            "SDD session not found: {session_id}"
        )));
    }
    std::fs::remove_dir_all(&session_dir)?;

    if get_active_sdd_session(paths, project_slug).as_deref() == Some(session_id) {
        let active_path = paths.sdd_active_session_path(project_slug);
        let _ = std::fs::remove_file(&active_path);
        let remaining = list_sdd_sessions(paths, project_slug);
        if let Some(next) = remaining.first() {
            set_active_sdd_session(paths, project_slug, &next.id)?;
        }
    }
    Ok(())
}

pub fn resolve_sdd_session_id(paths: &KnowledgePaths, project_slug: &str) -> String {
    if let Some(id) = get_active_sdd_session(paths, project_slug) {
        let dir = paths.sdd_workspace_dir(project_slug, &id);
        if dir.is_dir() {
            return id;
        }
    }
    let sessions = list_sdd_sessions(paths, project_slug);
    if let Some(first) = sessions.first() {
        let _ = set_active_sdd_session(paths, project_slug, &first.id);
        return first.id.clone();
    }
    create_sdd_session(paths, project_slug, "默认需求")
        .map(|s| s.id)
        .unwrap_or_else(|_| "default".into())
}

pub fn save_sdd_output(
    paths: &KnowledgePaths,
    output_path: &str,
    content: &str,
) -> crate::error::Result<()> {
    let path = Path::new(output_path);
    if !paths.is_sdd_local_path(path) {
        return Err(crate::error::CoreError::InvalidDoc(
            "SDD output path must be under ~/.terrain/sdd/".into(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

pub fn plan_sdd_workflow(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> SddPlan {
    let skill_dir = resolve_sdd_skill_dir().unwrap_or_else(default_sdd_skill_dir);
    SddPlan {
        project_slug: project_slug.to_string(),
        session_id: session_id.to_string(),
        repo_path: repo_path.as_ref().display().to_string(),
        skill_dir: skill_dir.display().to_string(),
        sdd_workspace_dir: paths
            .sdd_workspace_dir(project_slug, session_id)
            .display()
            .to_string(),
        sdd_output_dir: paths
            .sdd_output_dir(project_slug, session_id)
            .display()
            .to_string(),
        human_output_dir: paths.human_docs_dir(project_slug).display().to_string(),
        agent_pack_path: paths.agent_pack_main(project_slug).display().to_string(),
        skill_ready: resolve_sdd_skill_dir().is_some(),
    }
}

pub fn sdd_phase_output_path(output_dir: impl AsRef<Path>, phase: SddPhase) -> PathBuf {
    output_dir.as_ref().join(phase.output_filename())
}

pub fn get_sdd_status(paths: &KnowledgePaths, project_slug: &str) -> SddStatus {
    let sessions = list_sdd_sessions(paths, project_slug);
    let active_session_id = get_active_sdd_session(paths, project_slug)
        .filter(|id| paths.sdd_workspace_dir(project_slug, id).is_dir())
        .or_else(|| {
            sessions.first().map(|s| {
                let _ = set_active_sdd_session(paths, project_slug, &s.id);
                s.id.clone()
            })
        });

    let (workspace_dir, output_dir, phases) = if let Some(ref session_id) = active_session_id {
        let out = paths.sdd_output_dir(project_slug, session_id);
        let phases = build_phase_infos(&out);
        (
            paths
                .sdd_workspace_dir(project_slug, session_id)
                .display()
                .to_string(),
            out.display().to_string(),
            phases,
        )
    } else {
        let empty_phases = SddPhase::all()
            .into_iter()
            .map(|phase| SddPhaseInfo {
                phase,
                label: phase.label().to_string(),
                output_path: String::new(),
                ready: false,
                updated_at: None,
            })
            .collect();
        (
            paths.sdd_local_root(project_slug).display().to_string(),
            String::new(),
            empty_phases,
        )
    };

    let current_phase = phases
        .iter()
        .find(|p| !p.ready)
        .map(|p| p.phase)
        .or_else(|| phases.last().map(|p| p.phase));

    SddStatus {
        project_slug: project_slug.to_string(),
        skill_ready: resolve_sdd_skill_dir().is_some(),
        workspace_dir,
        output_dir,
        phases,
        current_phase,
        active_session_id,
        sessions,
    }
}

fn build_phase_infos(output_dir: &Path) -> Vec<SddPhaseInfo> {
    SddPhase::all()
        .into_iter()
        .map(|phase| {
            let path = sdd_phase_output_path(output_dir, phase);
            let ready = path.is_file();
            let updated_at = ready
                .then(|| {
                    std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            chrono::DateTime::<chrono::Utc>::from(t)
                                .format("%Y-%m-%d %H:%M UTC")
                                .to_string()
                        })
                })
                .flatten();
            SddPhaseInfo {
                phase,
                label: phase.label().to_string(),
                output_path: path.display().to_string(),
                ready,
                updated_at,
            }
        })
        .collect()
}

pub fn build_sdd_phase_prompt(plan: &SddPlan, phase: SddPhase, user_input: &str) -> String {
    let prior = read_prior_outputs(&plan.sdd_output_dir, phase);
    let prior_block = if prior.is_empty() {
        String::new()
    } else {
        format!("\n\n## Prior SDD artifacts\n\n{prior}")
    };

    let current_draft = read_current_phase_draft(&plan.sdd_output_dir, phase);
    let draft_block = if current_draft.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n## Current draft (human may have edited — revise per feedback)\n\n{current_draft}"
        )
    };

    let user_block = if user_input.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n## Human feedback / instructions\n\n{user_input}")
    };

    match phase {
        SddPhase::Requirements => format!(
            "Run SDD Phase 1 — Requirement clarification for project \"{}\" (session \"{}\").\n\n\
             Skill directory (read SKILL.md): {}\n\
             Repository root: {}\n\
             Output file (write here with absolute path): {}/{}\n\
             Workspace: {}\n\n\
             Produce a structured requirements document covering goals, user stories, \
             acceptance criteria, constraints, and open questions.{prior_block}{draft_block}{user_block}\n\n\
             Write the final markdown to the output file path above.",
            plan.project_slug,
            plan.session_id,
            plan.skill_dir,
            plan.repo_path,
            plan.sdd_output_dir,
            phase.output_filename(),
            plan.sdd_workspace_dir,
        ),
        SddPhase::TechDesign => format!(
            "Run SDD Phase 2 — Technical design for project \"{}\" (session \"{}\").\n\n\
             Skill directory: {}\n\
             Repository root: {}\n\
             Human docs directory: {}\n\
             Agent pack index: {}\n\
             Output file: {}/{}\n{prior_block}{draft_block}{user_block}\n\n\
             Produce a technical design with architecture decisions, module boundaries, \
             data flow, API changes, and implementation plan. Reference human docs and \
             agent pack when available.",
            plan.project_slug,
            plan.session_id,
            plan.skill_dir,
            plan.repo_path,
            plan.human_output_dir,
            plan.agent_pack_path,
            plan.sdd_output_dir,
            phase.output_filename(),
        ),
        SddPhase::CodeGen => format!(
            "Run SDD Phase 3 — Code generation for project \"{}\" (session \"{}\").\n\n\
             Skill directory: {}\n\
             Repository root (implement changes here): {}\n\
             Output notes file: {}/{}\n{prior_block}{draft_block}{user_block}\n\n\
             Implement the approved technical design in the repository. \
             Write a brief implementation summary to the output notes file.",
            plan.project_slug,
            plan.session_id,
            plan.skill_dir,
            plan.repo_path,
            plan.sdd_output_dir,
            phase.output_filename(),
        ),
        SddPhase::CodeReview => format!(
            "Run SDD Phase 4 — Intelligent code review for project \"{}\" (session \"{}\").\n\n\
             Skill directory: {}\n\
             Repository root: {}\n\
             Output file: {}/{}\n{prior_block}{draft_block}{user_block}\n\n\
             Review implementation against requirements and technical design. \
             Report findings by severity: critical, major, minor, suggestions.",
            plan.project_slug,
            plan.session_id,
            plan.skill_dir,
            plan.repo_path,
            plan.sdd_output_dir,
            phase.output_filename(),
        ),
    }
}

pub fn build_sdd_llm_prompt(plan: &SddPlan, phase: SddPhase, user_input: &str) -> String {
    let base = build_sdd_phase_prompt(plan, phase, user_input);
    format!(
        "{base}\n\n\
         Respond with the complete markdown document only — no preamble. \
         The document will be saved directly to disk."
    )
}

fn read_prior_outputs(output_dir: &str, up_to: SddPhase) -> String {
    let dir = Path::new(output_dir);
    if !dir.is_dir() {
        return String::new();
    }
    let mut parts = Vec::new();
    for phase in SddPhase::all() {
        if phase.order() >= up_to.order() {
            break;
        }
        let path = sdd_phase_output_path(dir, phase);
        if let Ok(body) = std::fs::read_to_string(&path) {
            parts.push(format!("### {} ({})\n\n{}", phase.label(), path.display(), body));
        }
    }
    parts.join("\n\n")
}

fn read_current_phase_draft(output_dir: &str, phase: SddPhase) -> String {
    let path = sdd_phase_output_path(Path::new(output_dir), phase);
    std::fs::read_to_string(path).unwrap_or_default()
}
