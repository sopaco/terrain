//! Cross-platform subprocess helpers.
//!
//! On Windows, GUI apps (Tauri) that spawn console tools (`powershell`, `where.exe`,
//! bundled CLIs, `git`, …) flash black terminal windows unless `CREATE_NO_WINDOW` is set.

use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Suppress the console window for a synchronous child process (Windows only).
pub fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Suppress the console window for an async child process (Windows only).
pub fn hide_console_async(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// `Command::new` with Windows console suppression when spawned from a GUI parent.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    hide_console(&mut cmd);
    cmd
}

/// `tokio::process::Command::new` with Windows console suppression.
pub fn async_command(program: impl AsRef<std::ffi::OsStr>) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    hide_console_async(&mut cmd);
    cmd
}
