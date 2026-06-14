# Third-party patches

Vendored `[patch.crates-io]` overrides for the Tauri dependency tree.

## Why these exist

Since **Rust 1.88**, the compiler rejects overlapping blanket `From<T>` impls that conflict with `time` 0.3.48 (used transitively by Tauri). This still fails on current stable (e.g. 1.96) with upstream crates — the patches are **not** obsolete yet.

| Crate | Issue | Patch |
|-------|-------|-------|
| `cookie` | `impl<T: Into<Option<OffsetDateTime>>> From<T> for Expiration` | Concrete `From` impls only |
| `tauri-utils` | Blanket `From<T>` on `Value` / `AssetKey` | Concrete `From` impls only |
| `tauri` | Blanket `From<T: Serialize> for InvokeError`, `EventTarget` | `from_serializable()`, concrete `From`, `Serialize for Error` |

Remove this directory when upstream releases fixes (track [tauri](https://github.com/tauri-apps/tauri), [cookie](https://github.com/SergioBenitez/cookie-rs)).

## Toolchain (project-wide)

| Setting | Value | Reason |
|---------|-------|--------|
| `rust-toolchain.toml` | `stable` | Matches team/dev CI; currently ≥ 1.94 |
| `Cargo.toml` `rust-version` | `1.94` | **MSRV** — required by `adk-*` 1.0.0 |

Rust **1.88** is only mentioned historically (when the coherence breakage appeared). The project does **not** target 1.88; do not pin the toolchain to 1.88.

## `tauri` vendored tree

Location: `third-party/tauri/` (fork of crates.io release, not a git submodule).

Patched files:

| File | Change |
|------|--------|
| `src/event/mod.rs` | Concrete `From<String>` / `From<&str>` for `EventTarget` |
| `src/ipc/mod.rs` | `from_serializable()`, `Serialize for InvokeError`, concrete `From` impls |
| `src/ipc/command.rs` | `ResultKind` / `ResultFutureKind` use `E: Serialize` |
| `src/error.rs` | `Serialize for Error` for invoke handlers returning `Result<_, Error>` |
