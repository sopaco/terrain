//! Durable progress for a Litho documentation run.
//!
//! One Litho session writes several Markdown files. If the session is interrupted (timeout,
//! agent failure, process kill), the files already on disk are still perfectly good documents.
//! This module records them so the next run resumes from the gap instead of wiping `human/`
//! and paying for the whole four-phase pipeline again.
//!
//! The checkpoint is a local, non-versioned derivative (`.terrain/.meta/litho-run.json`):
//! it describes an in-flight run on one machine, not shared knowledge.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{doc, fingerprint};

pub const LITHO_RUN_STATE_SCHEMA: u32 = 1;
pub const STATUS_IN_PROGRESS: &str = "in_progress";
pub const STATUS_COMPLETE: &str = "complete";

/// Empty or placeholder files don't count as finished documents.
const MIN_DOC_BYTES: u64 = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LithoDocRecord {
    /// Path relative to `human/`, using `/` separators.
    pub path: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LithoRunState {
    pub schema_version: u32,
    pub project: String,
    /// HEAD the run targets; `None` outside a Git repo. A different HEAD invalidates the run.
    pub target_head: Option<String>,
    pub mode: String,
    pub status: String,
    pub docs: Vec<LithoDocRecord>,
    pub updated_at: String,
}

pub fn load(path: &Path) -> Option<LithoRunState> {
    doc::read_json::<LithoRunState>(path)
        .ok()
        .filter(|state| state.schema_version == LITHO_RUN_STATE_SCHEMA)
}

/// Whether an unfinished run can be resumed against the current checkout.
///
/// Requires the same HEAD and at least one finished document — a run that produced nothing
/// has no progress worth preserving, and a different HEAD means the docs describe other code.
pub fn is_resumable(state: Option<&LithoRunState>, current_head: Option<&str>) -> bool {
    match state {
        Some(state)
            if state.status == STATUS_IN_PROGRESS
                && state.target_head.as_deref() == current_head
                && !state.docs.is_empty() =>
        {
            true
        }
        _ => false,
    }
}

/// Whether a completed run still describes the files on disk, unchanged.
pub fn still_valid(state: &LithoRunState, current_head: Option<&str>, human_dir: &Path) -> bool {
    state.status == STATUS_COMPLETE
        && state.target_head.as_deref() == current_head
        && !state.docs.is_empty()
        && state.docs == scan_docs(human_dir)
}

/// Document names to hand to the agent as "already done, leave these alone".
pub fn completed_names(state: Option<&LithoRunState>) -> Vec<String> {
    state
        .map(|s| s.docs.iter().map(|d| d.path.clone()).collect())
        .unwrap_or_default()
}

/// Start (or continue) a run, seeded with whatever documents are already on disk.
pub fn begin(
    path: &Path,
    project: &str,
    target_head: Option<&str>,
    mode: &str,
    human_dir: &Path,
) -> LithoRunState {
    let state = LithoRunState {
        schema_version: LITHO_RUN_STATE_SCHEMA,
        project: project.to_string(),
        target_head: target_head.map(str::to_string),
        mode: mode.to_string(),
        status: STATUS_IN_PROGRESS.into(),
        docs: scan_docs(human_dir),
        updated_at: now(),
    };
    let _ = doc::write_json(path, &state);
    state
}

/// Refresh the checkpoint from the files currently on disk.
///
/// Cheap enough to call on every progress tick: it only rewrites the JSON when the set of
/// finished documents actually changed.
pub fn record(path: &Path, human_dir: &Path) {
    let Some(mut state) = load(path) else {
        return;
    };
    let docs = scan_docs(human_dir);
    if docs == state.docs {
        return;
    }
    state.docs = docs;
    state.updated_at = now();
    let _ = doc::write_json(path, &state);
}

pub fn complete(path: &Path, human_dir: &Path) {
    let Some(mut state) = load(path) else {
        return;
    };
    state.status = STATUS_COMPLETE.into();
    state.docs = scan_docs(human_dir);
    state.updated_at = now();
    let _ = doc::write_json(path, &state);
}

pub fn reset(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn scan_docs(human_dir: &Path) -> Vec<LithoDocRecord> {
    if !human_dir.is_dir() {
        return Vec::new();
    }
    let mut docs: Vec<LithoDocRecord> = walkdir::WalkDir::new(human_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().is_some_and(|ext| ext == "md")
                && entry.metadata().is_ok_and(|m| m.len() >= MIN_DOC_BYTES)
        })
        .filter_map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(human_dir)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(entry.path()).ok()?;
            Some(LithoDocRecord {
                path: relative,
                fingerprint: fingerprint::fingerprint(&bytes),
            })
        })
        .collect();
    docs.sort_by(|a, b| a.path.cmp(&b.path));
    docs
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_doc(dir: &Path, name: &str, body: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn records_progress_and_skips_unchanged_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let human = tmp.path().join("human");
        let state_path = tmp.path().join("state.json");
        write_doc(&human, "1.md", &"a".repeat(300));

        let state = begin(&state_path, "p", Some("abc"), "full", &human);
        assert_eq!(state.status, STATUS_IN_PROGRESS);
        assert_eq!(state.docs.len(), 1);

        write_doc(&human, "4.Deep-Exploration/m.md", &"b".repeat(300));
        record(&state_path, &human);
        assert_eq!(load(&state_path).unwrap().docs.len(), 2);

        record(&state_path, &human);
        assert_eq!(load(&state_path).unwrap().docs.len(), 2);
    }

    #[test]
    fn resumable_only_for_same_head_and_unfinished_run() {
        let tmp = tempfile::tempdir().unwrap();
        let human = tmp.path().join("human");
        let state_path = tmp.path().join("state.json");
        write_doc(&human, "1.md", &"a".repeat(300));

        let mut state = begin(&state_path, "p", Some("abc"), "full", &human);
        assert!(is_resumable(Some(&state), Some("abc")));
        assert!(!is_resumable(Some(&state), Some("def")));
        assert!(!is_resumable(None, Some("abc")));

        state.status = STATUS_COMPLETE.into();
        assert!(!is_resumable(Some(&state), Some("abc")));
    }

    #[test]
    fn completed_run_is_valid_until_a_document_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let human = tmp.path().join("human");
        let state_path = tmp.path().join("state.json");
        write_doc(&human, "1.md", &"a".repeat(300));

        begin(&state_path, "p", Some("abc"), "full", &human);
        complete(&state_path, &human);
        let state = load(&state_path).unwrap();
        assert!(still_valid(&state, Some("abc"), &human));

        write_doc(&human, "1.md", &"c".repeat(400));
        assert!(!still_valid(&state, Some("abc"), &human));
    }

    #[test]
    fn tiny_placeholder_files_do_not_count_as_finished() {
        let tmp = tempfile::tempdir().unwrap();
        let human = tmp.path().join("human");
        write_doc(&human, "1.md", "TODO");
        let state_path = tmp.path().join("state.json");
        let state = begin(&state_path, "p", None, "full", &human);
        assert!(state.docs.is_empty());
    }
}
