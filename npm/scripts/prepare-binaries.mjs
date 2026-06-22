#!/usr/bin/env node
/**
 * Stage native binaries into @terrain-ai/*-platform npm packages before publish.
 *
 * Usage (from repo root):
 *   node npm/scripts/prepare-binaries.mjs
 *   node npm/scripts/prepare-binaries.mjs --build-cli
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const npmRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(npmRoot, "..");

const buildCli = process.argv.includes("--build-cli");

function copyBinary({ label, src, dest }) {
  if (!fs.existsSync(src)) {
    console.error(`[prepare-binaries] missing ${label}: ${src}`);
    process.exit(1);
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
  fs.chmodSync(dest, 0o755);
  console.log(`[prepare-binaries] ${label} → ${path.relative(repoRoot, dest)}`);
}

if (buildCli) {
  console.log("[prepare-binaries] cargo build --release -p terrain-cli …");
  execFileSync("cargo", ["build", "--release", "-p", "terrain-cli"], {
    cwd: repoRoot,
    stdio: "inherit",
  });
}

const terrainRelease = path.join(repoRoot, "target/release/terrain");
const terrainSidecar = path.join(repoRoot, "packages/terrain/darwin-arm64/terrain");

copyBinary({
  label: "rtk",
  src: path.join(repoRoot, "packages/rtk/darwin-arm64/rtk"),
  dest: path.join(npmRoot, "packages/rtk-darwin-arm64/bin/rtk"),
});

if (fs.existsSync(terrainRelease)) {
  copyBinary({
    label: "terrain-cli (npm)",
    src: terrainRelease,
    dest: path.join(npmRoot, "packages/cli-darwin-arm64/bin/terrain"),
  });
  fs.mkdirSync(path.dirname(terrainSidecar), { recursive: true });
  fs.copyFileSync(terrainRelease, terrainSidecar);
  fs.chmodSync(terrainSidecar, 0o755);
  console.log(`[prepare-binaries] terrain-cli (sidecar) → ${path.relative(repoRoot, terrainSidecar)}`);
} else {
  console.warn("[prepare-binaries] skip terrain-cli — run with --build-cli or cargo build --release -p terrain-cli");
}

console.log("[prepare-binaries] done");
