use std::path::Path;

use crate::paths::KnowledgePaths;
use crate::preset_skills::{default_litho_skill_dir, resolve_litho_skill_dir};
use crate::schema::LithoPlan;

/// Minimum research files before skipping the full C4 pipeline.
pub const LITHO_CORE_RESEARCH_FILES: &[&str] = &[
    "preprocessing.md",
    "c1-system-context.md",
    "c2-domain-modules.md",
    "architecture.md",
    "workflow.md",
    "boundary.md",
];

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
         Persist research artifacts under the intermediate workspace (TERRAIN_LITHO_WORKSPACE).\n\
         Write final Markdown files to the output directory (TERRAIN_HUMAN_OUTPUT_DIR): \
         1.概述.md through 6.数据库概览.md plus 4.Deep-Exploration/*.md.\n\
         Include source file paths (and line numbers when possible) in the docs for DeepWiki citations.\n\n\
         IMPORTANT: Use the absolute paths from TERRAIN_LITHO_WORKSPACE and TERRAIN_HUMAN_OUTPUT_DIR. \
         Do not finish until the full Litho doc set exists under the output directory.",
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
         1. Read all markdown files under the intermediate workspace (TERRAIN_LITHO_WORKSPACE).\n\
         2. Compose final human-facing docs per the skill templates.\n\
         3. Write missing files to the output directory (TERRAIN_HUMAN_OUTPUT_DIR) using absolute paths:\n\
            - 1.概述.md\n\
            - 2.架构.md\n\
            - 3.工作流.md\n\
            - 4.Deep-Exploration/{{module}}.md (one per module under workspace/modules/)\n\
            - 5.边界接口.md\n\
            - 6.数据库概览.md\n\
         If some files already exist in the output directory, keep them unless incomplete — fill gaps only.\n\
         Do not repeat preprocessing or C4 research. \
         Do not stop until the output directory contains the full Litho doc set.",
        plan.project_slug,
        plan.litho_workspace_dir,
        plan.skill_dir,
        plan.human_output_dir,
        plan.repo_path
    )
}

/// Prompt for an **incremental** update of the Litho `human/` doc set from a Git change set.
///
/// Skips phases 1–2 (preprocessing and C4 research) entirely: the existing docs are the
/// research output, so a normal commit only needs the affected files touched up. Deep-dive
/// files are per-module, which makes the changed-path list a direct routing signal.
pub fn build_litho_update_prompt(
    plan: &LithoPlan,
    incremental: &crate::assets::IncrementalPlan,
    existing_docs: &[String],
) -> String {
    let doc_list = if existing_docs.is_empty() {
        "(none listed)".to_string()
    } else {
        existing_docs
            .iter()
            .map(|d| format!("- {d}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Incrementally update the existing human-facing documentation for project \"{project}\".\n\n\
         Skill directory (read references/phase3-composition.md for the doc templates): {skill}\n\
         Repository root: {repo}\n\
         Documentation directory (edit files in place here): {out}\n\n\
         {rules}\
         {evidence}\
         ## Existing documents\n{doc_list}\n\n\
         ## How to work\n\
         1. Read the change list above and build a docs-impact plan: for each changed path, decide \
            which existing file (if any) makes a claim that is now wrong.\n\
         2. Read only those files, and only the changed source paths needed to correct them.\n\
         3. Edit those files in place with the smallest change that makes them accurate. Keep the \
            existing structure, heading levels, tone and citation style.\n\
         4. Add a `4.Deep-Exploration/{{module}}.md` file only when the changes introduce a genuinely \
            new module; delete one only when its module was removed from the repository.\n\
         5. Keep source file paths (and line numbers where practical) in the text for DeepWiki \
            citations, and correct any citation whose path was moved or deleted.\n\n\
         ## Hard constraints\n\
         - Do NOT run preprocessing or C4 research; do NOT rebuild the intermediate workspace.\n\
         - Do NOT delete or truncate existing files, and do NOT rewrite files end to end.\n\
         - Do NOT touch a file that the change list does not affect — an untouched file is the \
           correct outcome, not a missed step.\n\
         - Edit at most 3 files unless the change list clearly justifies more.\n\
         - If nothing in the change list invalidates any document, change nothing and say so.\n",
        project = plan.project_slug,
        skill = plan.skill_dir,
        repo = plan.repo_path,
        out = plan.human_output_dir,
        rules = incremental
            .update_rules_block(crate::assets::IncrementalOutputMode::EditFilesInPlace),
        evidence = incremental.evidence_block(),
        doc_list = doc_list,
    )
}

/// Markdown file names under `human/`, relative to the doc root (sorted, for prompt listings).
pub fn list_human_doc_names(human_dir: impl AsRef<Path>) -> Vec<String> {
    let dir = human_dir.as_ref();
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|e| {
            e.path()
                .strip_prefix(dir)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    names.sort();
    names
}

/// Recorded baseline commit for the `human/` doc set, if a previous run wrote one.
pub fn human_docs_baseline_head(
    paths: &KnowledgePaths,
    project_slug: &str,
) -> Option<String> {
    crate::doc::read_json::<crate::schema::HumanDocsMeta>(paths.human_docs_meta_path(project_slug))
        .ok()
        .and_then(|meta| meta.baseline_git_head)
}

/// Persist the `human/` doc-set sidecar so the next refresh has a diff baseline.
pub fn write_human_docs_meta(
    paths: &KnowledgePaths,
    project_slug: &str,
    repo_path: &str,
    run_mode: &str,
) -> crate::error::Result<crate::schema::HumanDocsMeta> {
    let meta = crate::schema::HumanDocsMeta {
        project: project_slug.to_string(),
        repo_path: crate::path_portable::stored_repo_path(Path::new(repo_path)),
        generated_at: chrono::Utc::now().to_rfc3339(),
        doc_count: count_markdown_in_dir(paths.human_docs_dir(project_slug)),
        baseline_git_head: crate::freshness::git_snapshot(repo_path).head,
        last_run_mode: run_mode.to_string(),
    };
    crate::doc::write_json(paths.human_docs_meta_path(project_slug), &meta)?;
    Ok(meta)
}

/// Any markdown research artifact exists (for UI hints).
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

/// Research is complete enough to run composition-only (core reports + module deep dives).
pub fn litho_research_ready(litho_workspace_dir: impl AsRef<Path>) -> bool {
    let dir = litho_workspace_dir.as_ref();
    if !dir.is_dir() {
        return false;
    }
    for name in LITHO_CORE_RESEARCH_FILES {
        if !dir.join(name).is_file() {
            return false;
        }
    }
    count_litho_research_modules(dir) > 0
}

pub fn count_litho_research_modules(litho_workspace_dir: impl AsRef<Path>) -> usize {
    count_markdown_in_dir(litho_workspace_dir.as_ref().join("modules"))
}

pub fn count_deep_exploration_modules(human_dir: impl AsRef<Path>) -> usize {
    count_markdown_in_dir(human_dir.as_ref().join("4.Deep-Exploration"))
}

/// Core Litho deliverables that must exist before treating generation as complete.
pub const LITHO_REQUIRED_HUMAN_FILES: &[&str] = &[
    "1.概述.md",
    "2.架构.md",
    "3.工作流.md",
    "5.边界接口.md",
    "6.数据库概览.md",
];

/// Whether the human output directory contains the full Litho doc set.
pub fn litho_human_complete(human_dir: impl AsRef<Path>) -> bool {
    litho_human_complete_with_research(human_dir, None)
}

/// Like [`litho_human_complete`], but when a research workspace is given, requires
/// `4.Deep-Exploration/` to cover every module report under `modules/`.
pub fn litho_human_complete_with_research(
    human_dir: impl AsRef<Path>,
    litho_workspace: Option<&Path>,
) -> bool {
    let dir = human_dir.as_ref();
    if !dir.is_dir() {
        return false;
    }
    for name in LITHO_REQUIRED_HUMAN_FILES {
        if !dir.join(name).is_file() {
            return false;
        }
    }
    let deep_count = count_deep_exploration_modules(dir);
    if deep_count == 0 {
        return false;
    }
    if let Some(workspace) = litho_workspace {
        let expected = count_litho_research_modules(workspace);
        if expected > 0 && deep_count < expected {
            return false;
        }
    }
    true
}

pub fn count_markdown_in_dir(dir: impl AsRef<Path>) -> usize {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return 0;
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .count()
}
