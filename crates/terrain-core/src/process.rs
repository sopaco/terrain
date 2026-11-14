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

/// Give the Windows GUI process a hidden console of its own (no-op elsewhere).
///
/// `CREATE_NO_WINDOW` covers the spawns *we* control. It cannot cover third-party code
/// running inside our process that spawns console programs itself — most notably
/// `repomix-core`, which shells out to `git rev-parse` / `git log` on the hot path of every
/// source-pack build, and any library that shells out to `npm`/`tar`/`ssh`. A GUI-subsystem
/// process has no console, so each of those children makes Windows allocate a fresh console
/// window: the black flash seen while generating knowledge assets.
///
/// Attaching a hidden console to our own process makes such children *inherit* it instead
/// of allocating a window. Children we start with `CREATE_NO_WINDOW` keep their own
/// invisible console, so the flag path is unaffected.
///
/// Call once, as early as possible in `main`, before any window is shown.
#[cfg(windows)]
pub fn ensure_hidden_console() {
    use windows_sys::Win32::System::Console::{AllocConsole, GetConsoleWindow};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

    // SAFETY: both calls are side-effect-only Win32 APIs. `GetConsoleWindow` returns a
    // borrowed HWND that is only handed back to `ShowWindow`, which does not take ownership.
    unsafe {
        // Already attached — a dev build, or the app was launched from a terminal. Never
        // hide a console the user can see on purpose.
        if !GetConsoleWindow().is_null() {
            return;
        }
        if AllocConsole() == 0 {
            return;
        }
        let window = GetConsoleWindow();
        if !window.is_null() {
            ShowWindow(window, SW_HIDE);
        }
    }
}

/// Give the Windows GUI process a hidden console of its own (no-op elsewhere).
#[cfg(not(windows))]
pub fn ensure_hidden_console() {}
