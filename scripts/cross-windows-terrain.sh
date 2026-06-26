#!/usr/bin/env bash
# Cross-compile terrain-cli for Windows x64 on macOS/Linux hosts.
# Requires: rustup, curl, tar
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="x86_64-pc-windows-gnullvm"
DEST="$ROOT/packages/terrain/win32-x64/terrain.exe"
LLVM_MINGW_VERSION="${LLVM_MINGW_VERSION:-20260616}"
LLVM_MINGW_DIR="${LLVM_MINGW_DIR:-/tmp/llvm-mingw-${LLVM_MINGW_VERSION}-ucrt-macos-universal}"
LLVM_MINGW_TAR="/tmp/llvm-mingw-${LLVM_MINGW_VERSION}-ucrt-macos-universal.tar.xz"
LLVM_MINGW_URL="${LLVM_MINGW_URL:-https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_MINGW_VERSION}/llvm-mingw-${LLVM_MINGW_VERSION}-ucrt-macos-universal.tar.xz}"
# Mirror example (faster in some regions):
# LLVM_MINGW_URL="https://ghfast.top/https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_MINGW_VERSION}/llvm-mingw-${LLVM_MINGW_VERSION}-ucrt-macos-universal.tar.xz"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script currently expects macOS universal llvm-mingw; set LLVM_MINGW_DIR manually on other hosts." >&2
  exit 1
fi

if [[ ! -x "${LLVM_MINGW_DIR}/bin/x86_64-w64-mingw32-clang" ]]; then
  echo "[cross-windows] downloading llvm-mingw ${LLVM_MINGW_VERSION} …"
  curl -fL --retry 3 --retry-delay 5 -o "$LLVM_MINGW_TAR" "$LLVM_MINGW_URL"
  rm -rf "$LLVM_MINGW_DIR"
  tar -xf "$LLVM_MINGW_TAR" -C /tmp
fi

export PATH="${LLVM_MINGW_DIR}/bin:$PATH"
export CC_x86_64_pc_windows_gnullvm=x86_64-w64-mingw32-clang
export CXX_x86_64_pc_windows_gnullvm=x86_64-w64-mingw32-clang++
export AR_x86_64_pc_windows_gnullvm=llvm-ar
export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER=x86_64-w64-mingw32-clang

rustup target add "$TARGET" >/dev/null

echo "[cross-windows] building terrain-cli ($TARGET) …"
cd "$ROOT"
cargo build --release --target "$TARGET" -p terrain-cli

mkdir -p "$(dirname "$DEST")"
cp "$ROOT/target/$TARGET/release/terrain.exe" "$DEST"
echo "[cross-windows] staged → $DEST"
file "$DEST"
