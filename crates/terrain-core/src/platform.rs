//! Cross-platform helpers (paths, binary names, platform keys).
//!
//! Platform directory keys match npm `platform.mjs` (`darwin-arm64`, `win32-x64`, …).

use std::path::{Path, PathBuf};

/// Runtime platform key, e.g. `darwin-arm64` or `win32-x64`.
pub fn platform_key() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    };
    format!("{os}-{arch}")
}

/// Map a Cargo `TARGET` triple to a `packages/` platform directory name.
pub fn platform_key_for_target(target: &str) -> &'static str {
    match target {
        "aarch64-apple-darwin" => "darwin-arm64",
        "x86_64-apple-darwin" => "darwin-x64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" | "x86_64-pc-windows-gnullvm" => "win32-x64",
        "aarch64-pc-windows-msvc" => "win32-arm64",
        "x86_64-unknown-linux-gnu" => "linux-x64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        _ => "darwin-arm64",
    }
}

pub fn is_windows() -> bool {
    cfg!(windows)
}

pub fn user_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Filename for an agent tool deployed under `~/.terrain/bin/`.
pub fn agent_tool_filename(base: &str) -> String {
    if is_windows() {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

/// Candidate relative paths for a bundled CLI inside `packages/<tool>/<platform>/`.
pub fn bundled_binary_candidates(tool: &str) -> Vec<String> {
    if is_windows() {
        vec![format!("{tool}.exe"), tool.to_string()]
    } else {
        vec![tool.to_string()]
    }
}

/// Candidate relative paths for the CodeGraph wrapper under `.../bin/`.
pub fn codegraph_wrapper_candidates() -> &'static [&'static str] {
    if is_windows() {
        &[
            "bin/codegraph.exe",
            "bin/codegraph.cmd",
            "bin/codegraph.bat",
            "bin/codegraph",
        ]
    } else {
        &["bin/codegraph"]
    }
}

/// Expand `~/…` and `%USERPROFILE%\…` / `%HOME%\…` prefixes.
pub fn expand_user_path(path: &Path) -> PathBuf {
    if let Some(rest) = path
        .to_str()
        .and_then(|s| s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")))
    {
        if let Some(home) = user_home() {
            return home.join(rest);
        }
    }
    if let Some(s) = path.to_str() {
        if let Some(expanded) = expand_windows_env_prefix(s) {
            return PathBuf::from(expanded);
        }
    }
    path.to_path_buf()
}

fn expand_windows_env_prefix(s: &str) -> Option<String> {
    const VARS: &[&str] = &["USERPROFILE", "HOME", "LOCALAPPDATA", "APPDATA"];
    for var in VARS {
        let prefix = format!("%{var}%");
        if let Some(rest) = s.strip_prefix(&prefix) {
            let rest = rest
                .strip_prefix('\\')
                .or_else(|| rest.strip_prefix('/'))
                .unwrap_or(rest);
            if let Ok(val) = std::env::var(var) {
                return Some(format!("{val}\\{rest}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_key_matches_npm_convention() {
        let key = platform_key();
        assert!(key.contains('-'));
    }

    #[test]
    fn agent_tool_filename_on_windows_has_exe() {
        if is_windows() {
            assert_eq!(agent_tool_filename("rtk"), "rtk.exe");
        } else {
            assert_eq!(agent_tool_filename("rtk"), "rtk");
        }
    }
}
