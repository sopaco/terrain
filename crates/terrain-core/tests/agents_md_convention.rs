//! Guard: every repository path quoted in `AGENTS.md` must still exist.
//!
//! `AGENTS.md` is the trust anchor every agent reads before choosing a knowledge layer. A path
//! that drifted (for example `schema.rs` after it became a `schema/` directory) silently sends
//! agents to a file that is not there, and the resulting "not found" is easily misread as
//! "the project has no such module". This test keeps the anchor honest without freezing prose.
//!
//! Two forms are checked:
//! - paths with a repository prefix (`crates/`, `src/`, `src-tauri/`, `env-catalog/`);
//! - bare file names in backticks (`status.rs`), resolved against the whole tree.
//! Fenced code blocks, commands, placeholders, globs and home-relative paths are skipped.

use std::fs;
use std::path::{Path, PathBuf};

const PATH_PREFIXES: [&str; 4] = ["crates/", "src/", "src-tauri/", "env-catalog/"];
const SOURCE_EXTENSIONS: [&str; 6] = ["rs", "ts", "tsx", "svelte", "md", "json"];
const SKIP_DIRS: [&str; 6] = [
    "target",
    "node_modules",
    ".git",
    "dist",
    ".codegraph",
    ".workbuddy",
];

fn workspace_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2)?;
    // No-op when the crate is built outside the monorepo (e.g. a published tarball).
    root.join("AGENTS.md").is_file().then(|| root.to_path_buf())
}

/// Inline-code spans outside fenced blocks. Parsed by hand so the test needs no extra deps.
fn backticked(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for span in line.split('`').skip(1).step_by(2) {
            let token = span.trim();
            if !token.is_empty() {
                out.push(token.to_string());
            }
        }
    }
    out
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(name) if SKIP_DIRS.contains(&name)
            ) {
                collect_files(&path, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn repo_paths(text: &str) -> Vec<String> {
    backticked(text)
        .into_iter()
        .filter(|token| PATH_PREFIXES.iter().any(|p| token.starts_with(p)))
        .filter(|token| {
            ![' ', '*', '~', '<', '>', '…', '\\']
                .iter()
                .any(|c| token.contains(*c))
        })
        .collect()
}

fn bare_filenames(text: &str) -> Vec<String> {
    backticked(text)
        .into_iter()
        .filter(|token| !token.contains(['/', '\\']))
        .filter(|token| {
            SOURCE_EXTENSIONS
                .iter()
                .any(|ext| token.ends_with(&format!(".{ext}")))
        })
        .filter(|token| {
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        })
        .collect()
}

#[test]
fn agents_md_repository_paths_exist() {
    let Some(root) = workspace_root() else {
        return;
    };
    let text = fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md is readable");

    let mut offenders = Vec::new();
    for path in repo_paths(&text) {
        let candidate = root.join(path.trim_end_matches('/'));
        if !candidate.exists() {
            offenders.push(path);
        }
    }

    assert!(
        offenders.is_empty(),
        "AGENTS.md quotes repository paths that no longer exist. Fix the anchor (or update the \
         path) instead of letting agents chase missing files: {offenders:#?}"
    );
}

#[test]
fn agents_md_bare_filenames_exist() {
    let Some(root) = workspace_root() else {
        return;
    };
    let text = fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md is readable");

    let mut files = Vec::new();
    collect_files(&root, &mut files);
    let known: Vec<String> = files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();

    let mut offenders = Vec::new();
    for name in bare_filenames(&text) {
        if !known.iter().any(|candidate| candidate == &name) {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "AGENTS.md quotes bare file names that exist nowhere in the repository: {offenders:#?}"
    );
}
