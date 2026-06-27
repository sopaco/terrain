//! Resolve executables and augment `PATH` for GUI-launched apps.
//!
//! macOS `.app` bundles started from Finder/Dock inherit a minimal `PATH`
//! (`/usr/bin:/bin:/usr/sbin:/sbin`). Developer tools are often installed via
//! Homebrew, bun, cargo, nvm, etc. and only added to PATH in *interactive*
//! shell init files (`.zshrc`), so a plain login shell (`-l`) is not enough.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, Once};

use crate::platform::{expand_user_path, user_home};
use crate::process::command as hidden_command;

static EXECUTABLE_CACHE: Mutex<Option<HashMap<String, Option<PathBuf>>>> = Mutex::new(None);
static PATH_INIT: Once = Once::new();

/// Merge an interactive login-shell `PATH` into the current process environment.
pub fn augment_path_from_login_shell() {
    let current = std::env::var("PATH").unwrap_or_default();
    let shell_path = interactive_shell_path()
        .or_else(login_shell_path)
        .or_else(path_helper_path)
        .or_else(windows_user_path)
        .unwrap_or_default();
    let merged = merge_paths(
        &merge_paths(&shell_path, &standard_user_bins_path()),
        &current,
    );
    if merged != current {
        // SAFETY: called during single-threaded app startup before worker threads spawn.
        unsafe { std::env::set_var("PATH", &merged) };
    }
}

fn ensure_path_initialized() {
    PATH_INIT.call_once(augment_path_from_login_shell);
}

/// Return `true` when `name` resolves to an executable file on `PATH`.
pub fn command_on_path(name: &str) -> bool {
    ensure_path_initialized();
    resolve_executable(name).is_some()
}

/// Resolve an executable name or path to an absolute path when possible.
pub fn resolve_executable(name: &str) -> Option<PathBuf> {
    ensure_path_initialized();
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(cached) = cached_lookup(trimmed) {
        return cached;
    }

    let resolved = resolve_executable_uncached(trimmed);
    cache_lookup(trimmed, resolved.clone());
    resolved
}

/// Rewrite the first token of a shell command to an absolute path when resolvable.
pub fn resolve_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut parts = trimmed.split_whitespace();
    let binary = parts.next().unwrap_or_default();
    let rest = parts.collect::<Vec<_>>().join(" ");

    let Some(resolved) = resolve_executable(binary) else {
        return trimmed.to_string();
    };

    let mut out = resolved.to_string_lossy().into_owned();
    if !rest.is_empty() {
        out.push(' ');
        out.push_str(&rest);
    }
    out
}

fn resolve_executable_uncached(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if has_path_component(path) {
        let expanded = expand_user_path_local(path);
        if is_executable_file(&expanded) {
            return Some(expanded);
        }
        return None;
    }

    for dir in search_directories() {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{name}.exe"));
            if is_executable_file(&with_exe) {
                return Some(with_exe);
            }
        }
    }

    shell_lookup_executable(name)
}

fn search_directories() -> Vec<PathBuf> {
    let mut dirs = standard_user_bin_dirs();
    dirs.extend(path_directories());
    dedupe_paths(dirs)
}

fn standard_user_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(not(windows))]
    {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    if let Some(home) = user_home() {
        #[cfg(windows)]
        {
            for rel in [
                ".cargo\\bin",
                ".bun\\bin",
                "AppData\\Local\\pnpm",
                "AppData\\Roaming\\npm",
                ".local\\bin",
                "go\\bin",
                ".volta\\bin",
            ] {
                dirs.push(home.join(rel));
            }
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                dirs.push(PathBuf::from(local).join("Programs"));
            }
        }
        #[cfg(not(windows))]
        {
            for rel in [
                ".bun/bin",
                ".local/bin",
                ".cargo/bin",
                "go/bin",
                ".npm-global/bin",
                "Library/pnpm",
                ".volta/bin",
                ".fnm/aliases/default/bin",
            ] {
                dirs.push(home.join(rel));
            }
        }
    }
    dirs
}

fn standard_user_bins_path() -> String {
    standard_user_bin_dirs()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(path_separator())
}

fn dedupe_paths(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let key = dir.to_string_lossy().into_owned();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(dir);
    }
    out
}

fn has_path_component(path: &Path) -> bool {
    path.is_absolute()
        || path.starts_with(".")
        || path
            .to_str()
            .is_some_and(|s| s.starts_with("~/") || s.starts_with("./"))
}

fn expand_user_path_local(path: &Path) -> PathBuf {
    expand_user_path(path)
}

fn path_directories() -> Vec<PathBuf> {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(path_separator())
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn path_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

fn merge_paths(primary: &str, secondary: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();
    for entry in primary
        .split(path_separator())
        .chain(secondary.split(path_separator()))
    {
        if entry.is_empty() || !seen.insert(entry.to_string()) {
            continue;
        }
        merged.push(entry);
    }
    merged.join(path_separator())
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "/bin/zsh".into()
        } else if cfg!(windows) {
            "cmd.exe".into()
        } else {
            "/bin/bash".into()
        }
    })
}

fn interactive_shell_path() -> Option<String> {
    run_shell_output(&["-il", "-c", "printf %s \"$PATH\""])
}

fn login_shell_path() -> Option<String> {
    run_shell_output(&["-l", "-c", "printf %s \"$PATH\""])
}

fn run_shell_output(args: &[&str]) -> Option<String> {
    #[cfg(unix)]
    {
        let shell = default_shell();
        let output = hidden_command(&shell)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8(output.stdout).ok()?;
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        windows_user_path()
    }
}

fn shell_lookup_executable(name: &str) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        if name.contains('\0') {
            return None;
        }
        let escaped = name.replace('\'', r"'\''");
        let script = format!("command -v -- '{escaped}'");
        let shell = default_shell();
        let output = hidden_command(&shell)
            .args(["-il", "-c", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            return None;
        }
        let candidate = PathBuf::from(&path);
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        if name.contains('\0') {
            return None;
        }
        let output = hidden_command("where.exe")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()?
            .trim()
            .to_string();
        if line.is_empty() {
            return None;
        }
        let candidate = PathBuf::from(line);
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn path_helper_path() -> Option<String> {
    let output = hidden_command("/usr/libexec/path_helper")
        .arg("-s")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_path_helper_output(&text)
}

#[cfg(not(target_os = "macos"))]
fn path_helper_path() -> Option<String> {
    None
}

#[cfg(windows)]
fn windows_user_path() -> Option<String> {
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','User') + ';' + [Environment]::GetEnvironmentVariable('Path','Machine')",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

#[cfg(not(windows))]
fn windows_user_path() -> Option<String> {
    None
}

fn parse_path_helper_output(text: &str) -> Option<String> {
    for line in text.split(';') {
        let line = line.trim();
        let Some((_, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn cached_lookup(name: &str) -> Option<Option<PathBuf>> {
    EXECUTABLE_CACHE
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref().and_then(|map| map.get(name).cloned()))
}

fn cache_lookup(name: &str, resolved: Option<PathBuf>) {
    if let Ok(mut guard) = EXECUTABLE_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(name.to_string(), resolved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[test]
    fn merge_paths_deduplicates_and_preserves_order() {
        let merged = merge_paths("/a:/b:/c", "/b:/d");
        assert_eq!(merged, "/a:/b:/c:/d");
    }

    #[test]
    fn standard_user_bin_dirs_include_bun() {
        let dirs = standard_user_bin_dirs();
        #[cfg(windows)]
        assert!(dirs.iter().any(|p| p.ends_with(".bun\\bin")));
        #[cfg(not(windows))]
        assert!(dirs.iter().any(|p| p.ends_with(".bun/bin")));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_command_rewrites_first_token() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("my-acp");
        fs::write(&bin, b"#!/bin/sh\n").unwrap();
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();

        // SAFETY: test-only PATH mutation in this thread.
        unsafe { std::env::set_var("PATH", dir.path()) };
        let resolved = resolve_command("my-acp --foo bar");
        assert_eq!(resolved, format!("{} --foo bar", bin.display()));
    }

    #[cfg(unix)]
    #[test]
    fn resolves_opencode_from_bun_bin_when_present() {
        if let Some(home) = user_home() {
            let bun_opencode = home.join(".bun/bin/opencode");
            if !bun_opencode.is_file() {
                return;
            }
        } else {
            return;
        }

        // Simulate GUI bundle PATH.
        unsafe { std::env::set_var("PATH", "/usr/bin:/bin") };
        assert!(command_on_path("opencode"));
        let resolved = resolve_executable("opencode").expect("opencode should resolve");
        assert!(resolved.ends_with("opencode"));
    }
}
