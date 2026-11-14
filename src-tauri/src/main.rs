#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Windows release builds are GUI-subsystem: without a console of our own, every console
    // child spawned by us *or by a library inside this process* (repomix-core shells out to
    // `git`) makes Windows open a terminal window. Attach a hidden console first. No-op on
    // macOS/Linux, and no-op when a console already exists (dev builds, launched from a shell).
    terrain_core::ensure_hidden_console();
    terrain_lib::run();
}
