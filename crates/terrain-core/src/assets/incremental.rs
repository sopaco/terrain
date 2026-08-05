//! Incremental knowledge refresh — plan an update from `git diff` instead of regenerating.
//!
//! Full regeneration of `agent/context.md` or the Litho `human/` set costs one long LLM/ACP
//! session each. Most commits touch a handful of files and invalidate at most one or two
//! sections, so this module turns "baseline commit → HEAD" into evidence a model can apply
//! surgically on top of the existing document.
//!
//! The decision is deliberately conservative: anything that makes the diff an unreliable
//! summary of the change (missing baseline, unreachable commit, non-Git repo, too many changed
//! files) falls back to [`KnowledgeUpdateMode::Full`].

use crate::freshness::{git_change_set, git_snapshot, GitChangeSet, GitChangedFile};
use crate::settings::KnowledgeSettings;

/// How many changed paths to spell out in a prompt before switching to a count.
const MAX_LISTED_CHANGED_FILES: usize = 80;

/// Cap on the `git diff --stat` block carried into a prompt.
const MAX_DIFF_STAT_CHARS: usize = 4000;

/// Inputs for [`plan_incremental_update`], derived from user settings.
#[derive(Debug, Clone, Copy)]
pub struct IncrementalOptions {
    pub enabled: bool,
    pub max_changed_files: u32,
}

impl From<&KnowledgeSettings> for IncrementalOptions {
    fn from(settings: &KnowledgeSettings) -> Self {
        Self {
            enabled: settings.incremental_refresh,
            max_changed_files: settings.incremental_max_changed_files,
        }
    }
}

/// Evidence for an incremental update of one asset.
#[derive(Debug, Clone)]
pub struct IncrementalPlan {
    pub baseline_head: String,
    pub current_head: Option<String>,
    pub changed_files: Vec<GitChangedFile>,
    pub commit_log: Vec<String>,
    pub diff_stat: String,
    pub dirty_paths: Vec<String>,
}

/// What a refresh should actually do for a given asset.
#[derive(Debug, Clone)]
pub enum KnowledgeUpdateMode {
    /// Regenerate from scratch. `reason` is a stable slug for logs and UI notes.
    Full { reason: &'static str },
    /// Apply the diff on top of the existing asset.
    Incremental(Box<IncrementalPlan>),
    /// Nothing changed since the asset's baseline — skip the model call entirely.
    UpToDate,
}

impl KnowledgeUpdateMode {
    pub fn is_incremental(&self) -> bool {
        matches!(self, Self::Incremental(_))
    }

    pub fn plan(&self) -> Option<&IncrementalPlan> {
        match self {
            Self::Incremental(plan) => Some(plan),
            _ => None,
        }
    }
}

/// Decide how to refresh an asset whose recorded baseline is `baseline`.
///
/// `asset_exists` must be the caller's own readiness check — an incremental update needs a
/// document to edit, so a missing or malformed asset always means [`KnowledgeUpdateMode::Full`].
pub fn plan_incremental_update(
    repo_path: &str,
    baseline: Option<&str>,
    asset_exists: bool,
    opts: IncrementalOptions,
) -> KnowledgeUpdateMode {
    if !opts.enabled {
        return KnowledgeUpdateMode::Full {
            reason: "incremental_disabled",
        };
    }
    if !asset_exists {
        return KnowledgeUpdateMode::Full {
            reason: "asset_missing",
        };
    }

    let git = git_snapshot(repo_path);
    if !git.is_git_repo {
        return KnowledgeUpdateMode::Full {
            reason: "not_a_git_repo",
        };
    }

    let Some(baseline) = baseline.map(str::trim).filter(|b| !b.is_empty()) else {
        return KnowledgeUpdateMode::Full {
            reason: "no_baseline",
        };
    };

    // A rebased / squashed / shallow-cloned baseline cannot be diffed against; an empty diff
    // there would be indistinguishable from "no changes".
    let Some(GitChangeSet {
        changed,
        commit_log,
        diff_stat,
        dirty_paths,
    }) = git_change_set(repo_path, baseline)
    else {
        return KnowledgeUpdateMode::Full {
            reason: "baseline_unreachable",
        };
    };

    if changed.is_empty() && dirty_paths.is_empty() {
        return KnowledgeUpdateMode::UpToDate;
    }

    let touched = distinct_touched_paths(&changed, &dirty_paths);
    if touched > opts.max_changed_files as usize {
        return KnowledgeUpdateMode::Full {
            reason: "too_many_changed_files",
        };
    }

    KnowledgeUpdateMode::Incremental(Box::new(IncrementalPlan {
        baseline_head: baseline.to_string(),
        current_head: git.head,
        changed_files: changed,
        commit_log,
        diff_stat,
        dirty_paths,
    }))
}

/// Union of committed and uncommitted paths — a file edited in both must count once.
fn distinct_touched_paths(changed: &[GitChangedFile], dirty: &[String]) -> usize {
    let mut paths: Vec<&str> = changed
        .iter()
        .map(|c| c.path.as_str())
        .chain(dirty.iter().map(String::as_str))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths.len()
}

impl IncrementalPlan {
    pub fn touched_file_count(&self) -> usize {
        distinct_touched_paths(&self.changed_files, &self.dirty_paths)
    }

    pub fn short_baseline(&self) -> &str {
        let end = self.baseline_head.len().min(7);
        &self.baseline_head[..end]
    }

    /// Render the change evidence as a prompt section.
    ///
    /// Paths carry their status letter so the model can tell a deleted module from a renamed
    /// one without reading the repository.
    pub fn evidence_block(&self) -> String {
        let mut out = String::new();
        out.push_str("## Changes since the last knowledge refresh (authoritative)\n");
        out.push_str(&format!(
            "Baseline commit: {}\nCurrent HEAD: {}\nTouched source files: {}\n\n",
            self.baseline_head,
            self.current_head.as_deref().unwrap_or("(unknown)"),
            self.touched_file_count(),
        ));

        if !self.commit_log.is_empty() {
            out.push_str("### Commits (newest first)\n");
            for line in &self.commit_log {
                out.push_str(&format!("- {line}\n"));
            }
            out.push('\n');
        }

        out.push_str("### Changed files (`git diff --name-status`)\n");
        if self.changed_files.is_empty() {
            out.push_str("(no committed changes — only uncommitted edits below)\n");
        } else {
            for file in self.changed_files.iter().take(MAX_LISTED_CHANGED_FILES) {
                out.push_str(&format!("- {} {}\n", file.status, file.path));
            }
            if self.changed_files.len() > MAX_LISTED_CHANGED_FILES {
                out.push_str(&format!(
                    "- …and {} more\n",
                    self.changed_files.len() - MAX_LISTED_CHANGED_FILES
                ));
            }
        }
        out.push('\n');

        if !self.dirty_paths.is_empty() {
            out.push_str("### Uncommitted working-tree changes\n");
            for path in self.dirty_paths.iter().take(MAX_LISTED_CHANGED_FILES) {
                out.push_str(&format!("- W {path}\n"));
            }
            if self.dirty_paths.len() > MAX_LISTED_CHANGED_FILES {
                out.push_str(&format!(
                    "- …and {} more\n",
                    self.dirty_paths.len() - MAX_LISTED_CHANGED_FILES
                ));
            }
            out.push('\n');
        }

        if !self.diff_stat.trim().is_empty() {
            let stat: String = self.diff_stat.chars().take(MAX_DIFF_STAT_CHARS).collect();
            out.push_str("### Diff stat\n```\n");
            out.push_str(stat.trim_end());
            out.push_str("\n```\n\n");
        }

        out
    }

    /// Shared editing contract for every incremental knowledge update.
    ///
    /// The critical rule is preservation: the model sees only a diff, so anything it cannot
    /// tie to a changed path must survive byte-for-byte rather than be rewritten from memory.
    pub fn update_rules_block(&self) -> String {
        format!(
            "## Incremental update rules (MANDATORY)\n\
             You are UPDATING existing documentation, not writing it again.\n\
             - The existing document below is the baseline and is presumed CORRECT for everything \
               the change list does not touch.\n\
             - Reproduce untouched sections **verbatim** — same wording, same ordering, same tables. \
               Do NOT rephrase, re-summarize, reformat, or \"improve\" them.\n\
             - Revise only what the {touched} changed file(s) actually invalidate: new/removed \
               modules, moved paths, changed flows, changed dependencies or boundaries.\n\
             - Deleted paths (status `D`) must be removed from module maps and path indexes; \
               renamed paths (status `R…`) must be updated to the new path.\n\
             - Investigate only the changed paths. Do not re-explore the whole repository.\n\
             - If a change is cosmetic (formatting, comments, tests, lockfiles, docs) and affects \
               no architectural claim, make NO edit for it.\n\
             - If nothing in the change list invalidates the document, return it completely \
               unchanged.\n\n",
            touched = self.touched_file_count(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    fn opts(enabled: bool, max: u32) -> IncrementalOptions {
        IncrementalOptions {
            enabled,
            max_changed_files: max,
        }
    }

    fn git(repo: &Path, args: &[&str]) {
        Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git");
    }

    fn head(repo: &Path) -> String {
        String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(repo)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string()
    }

    fn init_repo(repo: &Path) -> String {
        git(repo, &["init"]);
        git(repo, &["config", "user.email", "t@test.com"]);
        git(repo, &["config", "user.name", "t"]);
        fs::write(repo.join("main.rs"), "fn main() {}\n").unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-m", "init"]);
        head(repo)
    }

    #[test]
    fn disabled_and_missing_asset_force_full_regeneration() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().display().to_string();
        assert!(matches!(
            plan_incremental_update(&repo, Some("abc"), true, opts(false, 60)),
            KnowledgeUpdateMode::Full {
                reason: "incremental_disabled"
            }
        ));
        assert!(matches!(
            plan_incremental_update(&repo, Some("abc"), false, opts(true, 60)),
            KnowledgeUpdateMode::Full {
                reason: "asset_missing"
            }
        ));
        // Not a git repo at all.
        assert!(matches!(
            plan_incremental_update(&repo, Some("abc"), true, opts(true, 60)),
            KnowledgeUpdateMode::Full {
                reason: "not_a_git_repo"
            }
        ));
    }

    #[test]
    fn unreachable_baseline_is_not_mistaken_for_no_changes() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        init_repo(repo);
        let repo_s = repo.display().to_string();

        let bogus = "0".repeat(40);
        assert!(matches!(
            plan_incremental_update(&repo_s, Some(&bogus), true, opts(true, 60)),
            KnowledgeUpdateMode::Full {
                reason: "baseline_unreachable"
            }
        ));
        assert!(matches!(
            plan_incremental_update(&repo_s, None, true, opts(true, 60)),
            KnowledgeUpdateMode::Full {
                reason: "no_baseline"
            }
        ));
    }

    #[test]
    fn clean_repo_at_baseline_is_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let baseline = init_repo(repo);
        let repo_s = repo.display().to_string();

        assert!(matches!(
            plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 60)),
            KnowledgeUpdateMode::UpToDate
        ));

        // Knowledge-only churn must not trigger an update of the knowledge itself.
        fs::create_dir_all(repo.join(".terrain/agent")).unwrap();
        fs::write(repo.join(".terrain/agent/context.md"), "generated\n").unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-m", "update terrain assets"]);
        assert!(matches!(
            plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 60)),
            KnowledgeUpdateMode::UpToDate
        ));
    }

    #[test]
    fn source_changes_produce_a_plan_with_status_letters() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let baseline = init_repo(repo);
        let repo_s = repo.display().to_string();

        fs::write(repo.join("added.rs"), "pub fn added() {}\n").unwrap();
        fs::remove_file(repo.join("main.rs")).unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-m", "swap entrypoint"]);
        // Plus an uncommitted edit.
        fs::write(repo.join("dirty.rs"), "// wip\n").unwrap();

        let mode = plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 60));
        let plan = mode.plan().expect("incremental plan");
        assert_eq!(plan.baseline_head, baseline);
        assert_eq!(plan.dirty_paths, vec!["dirty.rs".to_string()]);
        assert_eq!(plan.touched_file_count(), 3);

        let statuses: Vec<(&str, &str)> = plan
            .changed_files
            .iter()
            .map(|c| (c.status.as_str(), c.path.as_str()))
            .collect();
        assert!(statuses.contains(&("A", "added.rs")), "{statuses:?}");
        assert!(statuses.contains(&("D", "main.rs")), "{statuses:?}");

        let evidence = plan.evidence_block();
        assert!(evidence.contains("- D main.rs"));
        assert!(evidence.contains("- W dirty.rs"));
        assert!(evidence.contains("swap entrypoint"));
        assert!(plan.update_rules_block().contains("verbatim"));
    }

    #[test]
    fn large_change_sets_fall_back_to_full_regeneration() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let baseline = init_repo(repo);
        let repo_s = repo.display().to_string();

        for i in 0..5 {
            fs::write(repo.join(format!("f{i}.rs")), "// x\n").unwrap();
        }
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-m", "bulk"]);

        assert!(matches!(
            plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 3)),
            KnowledgeUpdateMode::Full {
                reason: "too_many_changed_files"
            }
        ));
        assert!(plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 5))
            .is_incremental());
    }
}
