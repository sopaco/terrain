use std::path::{Path, PathBuf};

use crate::paths::KnowledgePaths;
use crate::schema::LithoPlan;

/// Default preset skill bundled with MindMesh.
pub fn default_litho_skill_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MIND_MESH_LITHO_SKILL") {
        return PathBuf::from(path);
    }

    // Workspace layout: mind-mesh/preset_skills/litho-documents-skill
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../preset_skills/litho-documents-skill")
}

pub fn resolve_litho_skill_dir() -> Option<PathBuf> {
    let dir = default_litho_skill_dir();
    if dir.join("SKILL.md").is_file() {
        Some(dir)
    } else {
        None
    }
}

pub fn plan_litho_generation(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: impl AsRef<Path>,
) -> LithoPlan {
    let skill_dir = resolve_litho_skill_dir().unwrap_or_else(default_litho_skill_dir);
    LithoPlan {
        project_slug: project_slug.to_string(),
        repo_path: repo_path.as_ref().display().to_string(),
        skill_dir: skill_dir.display().to_string(),
        human_output_dir: paths.human_docs_dir(project_slug).display().to_string(),
        litho_workspace_dir: paths.litho_workspace_dir(project_slug).display().to_string(),
        skill_ready: resolve_litho_skill_dir().is_some(),
    }
}

/// Prompt for ACP / OpenCode to run Litho document generation.
pub fn build_litho_generation_prompt(plan: &LithoPlan) -> String {
    format!(
        "Generate human-facing project documentation using the Litho document skill.\n\n\
         Skill directory (read SKILL.md first): {}\n\
         Repository root: {}\n\
         Output directory (write final docs here): {}\n\
         Intermediate workspace: {}\n\n\
         Follow the four-phase pipeline in the skill: preprocessing → C4 research → composition → output.\n\
         Persist research artifacts under the intermediate workspace.\n\
         Write final Markdown files to the output directory (1.概述.md through 6.数据库概览.md).\n\
         Include source file paths (and line numbers when possible) in the docs for DeepWiki citations.\n\n\
         IMPORTANT: Do not finish until all required files exist under the output directory. \
         Use absolute paths when writing outside the repository root.",
        plan.skill_dir, plan.repo_path, plan.human_output_dir, plan.litho_workspace_dir
    )
}

/// Follow-up prompt when research artifacts exist but final human docs were not written.
pub fn build_litho_composition_prompt(plan: &LithoPlan) -> String {
    format!(
        "Continue Litho document generation for project \"{}\".\n\n\
         Research artifacts are already persisted under: {}\n\
         Skill directory (read references/phase3-composition.md and phase4-output.md): {}\n\
         Final output directory (write here): {}\n\
         Repository root (for code citations): {}\n\n\
         Execute ONLY phase 3 (composition) and phase 4 (output validation):\n\
         1. Read all markdown files under the intermediate workspace.\n\
         2. Compose final human-facing docs per the skill templates.\n\
         3. Write these files to the output directory using absolute paths:\n\
            - 1.概述.md\n\
            - 2.架构.md\n\
            - 3.工作流.md\n\
            - 4.Deep-Exploration/{{module}}.md (one per researched module)\n\
            - 5.边界接口.md\n\
            - 6.数据库概览.md\n\
         Do not repeat preprocessing or C4 research. \
         Do not stop until the output directory contains the required markdown files.",
        plan.project_slug,
        plan.litho_workspace_dir,
        plan.skill_dir,
        plan.human_output_dir,
        plan.repo_path
    )
}

pub fn has_litho_research_artifacts(litho_workspace_dir: impl AsRef<Path>) -> bool {
    let dir = litho_workspace_dir.as_ref();
    if !dir.is_dir() {
        return false;
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().is_some_and(|ext| ext == "md"))
}
