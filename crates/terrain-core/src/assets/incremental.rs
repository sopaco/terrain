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
    ///
    /// `output` decides how "nothing changed" is expressed, and that difference matters: with
    /// [`IncrementalOutputMode::WholeDocumentReply`] the reply *is* the file, so "leave it alone"
    /// must still mean "emit every byte of it". Telling such a caller it may reply with nothing
    /// invites a short confirmation message that would then be persisted as the whole document.
    pub fn update_rules_block(&self, output: IncrementalOutputMode) -> String {
        let no_op_rule = match output {
            IncrementalOutputMode::WholeDocumentReply => {
                "- If nothing in the change list invalidates the document, still output the \
                 **entire document verbatim** — an empty reply, a summary of what you checked, or \
                 a note saying \"unchanged\" DELETES the document.\n"
            }
            IncrementalOutputMode::EditFilesInPlace => {
                "- If nothing in the change list invalidates the documents, change no file and \
                 say so.\n"
            }
        };
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
             {no_op_rule}\n",
            touched = self.touched_file_count(),
            no_op_rule = no_op_rule,
        )
    }
}

/// Fraction of the baseline's length an incremental result must retain to be believable.
///
/// Deliberately loose: a legitimate update can delete a removed module's rows, but it cannot
/// lose a third of the document and still be "the same document with edits applied".
const MIN_RETAINED_LENGTH_RATIO: f64 = 0.6;

/// Reject an incremental result that cannot plausibly be `baseline` with edits applied.
///
/// Assets persisted from reply text have no diff to apply and no way to notice a partial answer:
/// whatever comes back replaces the file. A model that edited the document with its own tool and
/// replied "updated 模块地图", or took the no-op escape hatch literally, produces a short
/// non-empty string that a bare `is_empty()` check waves through — and the real document is gone.
///
/// Returns `Some(reason)` when the result must NOT be persisted; the reason is a stable slug for
/// logs and UI notes.
pub fn reject_incremental_document(updated: &str, baseline: &str) -> Option<&'static str> {
    let updated = updated.trim();
    if updated.is_empty() {
        return Some("empty_reply");
    }

    let baseline = baseline.trim();
    if baseline.is_empty() {
        // No prior document to protect — the caller's own readiness check governs.
        return None;
    }

    if count_h2_sections(updated) < count_h2_sections(baseline) {
        return Some("sections_lost");
    }

    let updated_len = updated.chars().count() as f64;
    let baseline_len = baseline.chars().count() as f64;
    if updated_len < baseline_len * MIN_RETAINED_LENGTH_RATIO {
        return Some("document_shrank");
    }

    None
}

/// `##`-level section count, counting a document that opens directly on a heading.
fn count_h2_sections(body: &str) -> usize {
    body.matches("\n## ").count() + usize::from(body.starts_with("## "))
}

/// How the model is expected to hand an incremental update back to Terrain.
///
/// This is a correctness constraint, not a style preference: `agent/context.md` is persisted
/// from the reply text (the native execution path has read-only tools and *cannot* write files),
/// while the Litho `human/` set is edited on disk by an ACP agent and never overwritten from a
/// reply. Prompts must state the matching contract or the two disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalOutputMode {
    /// The reply text becomes the entire file. Omitting anything deletes it.
    WholeDocumentReply,
    /// The model edits files on disk; its reply is only commentary.
    EditFilesInPlace,
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
        assert!(plan
            .update_rules_block(IncrementalOutputMode::WholeDocumentReply)
            .contains("verbatim"));
    }

    /// A realistic 7-section document, as the baseline to protect.
    fn baseline_doc() -> String {
        [
            "项目概览",
            "架构设计",
            "模块地图",
            "核心流程",
            "技术选型",
            "系统边界",
            "代码映射索引",
        ]
        .iter()
        .map(|t| format!("## {t}\n\n{}\n", "内容".repeat(200)))
        .collect::<Vec<_>>()
        .join("\n")
    }

    #[test]
    fn rejects_the_replies_a_bare_empty_check_waves_through() {
        let baseline = baseline_doc();

        // Agent edited the file with its own tool and only summarized what it did.
        assert_eq!(
            reject_incremental_document(
                "I've updated the 模块地图 section and corrected the path index. \
                 No other sections needed changes.",
                &baseline,
            ),
            Some("sections_lost")
        );

        // The no-op escape hatch taken literally.
        assert_eq!(
            reject_incremental_document(
                "Nothing in the change list invalidates the document. Returned unchanged.",
                &baseline,
            ),
            Some("sections_lost")
        );

        // Only the sections it touched.
        assert_eq!(
            reject_incremental_document(
                "## 模块地图\n\n| Module | Responsibility |\n|---|---|\n\n## 代码映射索引\n\n| C | L |",
                &baseline,
            ),
            Some("sections_lost")
        );

        assert_eq!(
            reject_incremental_document("   \n  ", &baseline),
            Some("empty_reply")
        );
    }

    #[test]
    fn rejects_a_document_that_kept_its_headings_but_lost_its_body() {
        let baseline = baseline_doc();
        // All seven headings present, bodies gutted — section count alone would pass this.
        let gutted = [
            "项目概览",
            "架构设计",
            "模块地图",
            "核心流程",
            "技术选型",
            "系统边界",
            "代码映射索引",
        ]
        .iter()
        .map(|t| format!("## {t}\n\nTODO\n"))
        .collect::<Vec<_>>()
        .join("\n");

        assert_eq!(
            reject_incremental_document(&gutted, &baseline),
            Some("document_shrank")
        );
    }

    #[test]
    fn accepts_a_genuine_incremental_edit() {
        let baseline = baseline_doc();

        // Unchanged is fine.
        assert_eq!(reject_incremental_document(&baseline, &baseline), None);

        // A real edit: one module row rewritten, everything else reproduced.
        let edited = baseline.replace("## 模块地图", "## 模块地图\n\n新增 incremental 模块");
        assert_eq!(reject_incremental_document(&edited, &baseline), None);

        // An added section is fine too.
        let grown = format!("{baseline}\n## 附录\n\n{}", "内容".repeat(50));
        assert_eq!(reject_incremental_document(&grown, &baseline), None);

        // No baseline to protect — nothing to reject against.
        assert_eq!(reject_incremental_document("## 新文档\n\n内容", ""), None);
    }

    #[test]
    fn no_op_rule_matches_the_persistence_model() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        let baseline = init_repo(repo);
        let repo_s = repo.display().to_string();
        fs::write(repo.join("added.rs"), "pub fn added() {}\n").unwrap();

        let mode = plan_incremental_update(&repo_s, Some(&baseline), true, opts(true, 60));
        let plan = mode.plan().expect("incremental plan");

        // Reply-is-the-file: "unchanged" must never mean "reply with nothing".
        let whole = plan.update_rules_block(IncrementalOutputMode::WholeDocumentReply);
        assert!(whole.contains("entire document verbatim"), "{whole}");
        assert!(whole.contains("DELETES the document"), "{whole}");

        // Edit-in-place: changing no file is the correct no-op.
        let in_place = plan.update_rules_block(IncrementalOutputMode::EditFilesInPlace);
        assert!(in_place.contains("change no file"), "{in_place}");
        assert!(!in_place.contains("DELETES the document"), "{in_place}");
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
