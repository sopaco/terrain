use std::path::{Path, PathBuf};

use crate::paths::KnowledgePaths;
use crate::schema::{SddPhase, SddPhaseInfo, SddPlan, SddStatus};

pub fn default_sdd_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MIND_MESH_SDD_SKILL") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../preset_skills/sdd-workflow-skill")
}

pub fn resolve_sdd_skill_dir() -> Option<PathBuf> {
    let dir = default_sdd_skill_dir();
    if dir.join("SKILL.md").is_file() {
        Some(dir)
    } else {
        None
    }
}

pub fn plan_sdd_workflow(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: impl AsRef<Path>,
) -> SddPlan {
    let skill_dir = resolve_sdd_skill_dir().unwrap_or_else(default_sdd_skill_dir);
    SddPlan {
        project_slug: project_slug.to_string(),
        repo_path: repo_path.as_ref().display().to_string(),
        skill_dir: skill_dir.display().to_string(),
        sdd_workspace_dir: paths.sdd_workspace_dir(project_slug).display().to_string(),
        sdd_output_dir: paths.sdd_output_dir(project_slug).display().to_string(),
        human_output_dir: paths.human_docs_dir(project_slug).display().to_string(),
        agent_pack_path: paths.agent_pack_main(project_slug).display().to_string(),
        skill_ready: resolve_sdd_skill_dir().is_some(),
    }
}

pub fn sdd_phase_output_path(output_dir: impl AsRef<Path>, phase: SddPhase) -> PathBuf {
    output_dir.as_ref().join(phase.output_filename())
}

pub fn get_sdd_status(paths: &KnowledgePaths, project_slug: &str) -> SddStatus {
    let output_dir = paths.sdd_output_dir(project_slug);
    let phases: Vec<SddPhaseInfo> = SddPhase::all()
        .into_iter()
        .map(|phase| {
            let path = sdd_phase_output_path(&output_dir, phase);
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
        .collect();

    let current_phase = phases
        .iter()
        .find(|p| !p.ready)
        .map(|p| p.phase)
        .or_else(|| phases.last().map(|p| p.phase));

    SddStatus {
        project_slug: project_slug.to_string(),
        skill_ready: resolve_sdd_skill_dir().is_some(),
        workspace_dir: paths.sdd_workspace_dir(project_slug).display().to_string(),
        output_dir: output_dir.display().to_string(),
        phases,
        current_phase,
    }
}

pub fn build_sdd_phase_prompt(plan: &SddPlan, phase: SddPhase, user_input: &str) -> String {
    let prior = read_prior_outputs(&plan.sdd_output_dir, phase);
    let prior_block = if prior.is_empty() {
        String::new()
    } else {
        format!("\n\n## Prior SDD artifacts\n\n{prior}")
    };

    let user_block = if user_input.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n## User input\n\n{user_input}")
    };

    match phase {
        SddPhase::Requirements => format!(
            "Run SDD Phase 1 — Requirement clarification for project \"{}\".\n\n\
             Skill directory (read SKILL.md): {}\n\
             Repository root: {}\n\
             Output file (write here with absolute path): {}/{}\n\
             Workspace: {}\n\n\
             Produce a structured requirements document covering goals, user stories, \
             acceptance criteria, constraints, and open questions.{prior_block}{user_block}\n\n\
             Write the final markdown to the output file path above.",
            plan.project_slug,
            plan.skill_dir,
            plan.repo_path,
            plan.sdd_output_dir,
            phase.output_filename(),
            plan.sdd_workspace_dir,
        ),
        SddPhase::TechDesign => format!(
            "Run SDD Phase 2 — Technical design for project \"{}\".\n\n\
             Skill directory: {}\n\
             Repository root: {}\n\
             Human docs directory: {}\n\
             Agent pack index: {}\n\
             Output file: {}/{}\n{prior_block}{user_block}\n\n\
             Produce a technical design with architecture decisions, module boundaries, \
             data flow, API changes, and implementation plan. Reference human docs and \
             agent pack when available.",
            plan.project_slug,
            plan.skill_dir,
            plan.repo_path,
            plan.human_output_dir,
            plan.agent_pack_path,
            plan.sdd_output_dir,
            phase.output_filename(),
        ),
        SddPhase::CodeGen => format!(
            "Run SDD Phase 3 — Code generation for project \"{}\".\n\n\
             Skill directory: {}\n\
             Repository root (implement changes here): {}\n\
             Output notes file: {}/{}\n{prior_block}{user_block}\n\n\
             Implement the approved technical design in the repository. \
             Write a brief implementation summary to the output notes file.",
            plan.project_slug,
            plan.skill_dir,
            plan.repo_path,
            plan.sdd_output_dir,
            phase.output_filename(),
        ),
        SddPhase::CodeReview => format!(
            "Run SDD Phase 4 — Intelligent code review for project \"{}\".\n\n\
             Skill directory: {}\n\
             Repository root: {}\n\
             Output file: {}/{}\n{prior_block}{user_block}\n\n\
             Review implementation against requirements and technical design. \
             Report findings by severity: critical, major, minor, suggestions.",
            plan.project_slug,
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
