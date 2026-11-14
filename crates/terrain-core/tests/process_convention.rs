//! Guard: every child process goes through [`terrain_core::process`].
//!
//! On Windows a GUI app (Tauri, `windows_subsystem = "windows"`) that spawns a console
//! program without `CREATE_NO_WINDOW` gets a flashing black terminal window. Rust offers
//! no way to retrofit that flag onto an existing `Command`, so the only durable defence is
//! to keep every spawn behind `crate::process::{command, async_command}`, which sets it.
//!
//! This test fails when a direct `Command::new` appears anywhere under `crates/` or
//! `src-tauri/src` outside the helper itself. Use the helper instead:
//!
//! ```ignore
//! crate::process::command("git").args(["--version"]).output()   // sync
//! crate::process::async_command("bun").arg("install").output()  // async
//! ```

use std::fs;
use std::path::{Path, PathBuf};

/// The only file allowed to build a bare `Command` — it applies the flag for everyone else.
const ALLOWED: &[&str] = &["crates/terrain-core/src/process.rs"];

fn workspace_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2)?;
    // Stay a no-op when the crate is built outside the monorepo (e.g. from a published
    // tarball), where sibling crates are simply absent.
    root.join("crates/terrain-core/src/process.rs")
        .is_file()
        .then(|| root.to_path_buf())
}

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = matches!(
                entry.file_name().to_str(),
                Some("target" | "node_modules" | ".git" | "dist")
            );
            if !skip {
                collect_rust_files(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn spawns_go_through_the_console_suppressing_helper() {
    let Some(root) = workspace_root() else {
        return;
    };

    let mut files = Vec::new();
    for dir in ["crates", "src-tauri/src"] {
        collect_rust_files(&root.join(dir), &mut files);
    }
    assert!(
        !files.is_empty(),
        "expected Rust sources under {}",
        root.display()
    );

    // Built from parts so this file does not trip its own check.
    let needle = ["Command", "::new("].concat();
    let mut offenders = Vec::new();
    for file in files {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        if ALLOWED.contains(&rel.as_str()) {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&file) {
            if text.contains(&needle) {
                offenders.push(rel);
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "spawn these processes through terrain_core::process::command / async_command so \
         CREATE_NO_WINDOW is set on Windows, otherwise a terminal window flashes in the \
         packaged app: {offenders:#?}"
    );
}
