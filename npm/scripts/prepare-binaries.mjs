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
import { platformKey, platformPackageName } from "./lib/platform.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const npmRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(npmRoot, "..");
const platform = platformKey();
const isWindows = process.platform === "win32";

const buildCli = process.argv.includes("--build-cli");

function copyBinary({ label, src, dest }) {
  if (!fs.existsSync(src)) {
    console.error(`[prepare-binaries] missing ${label}: ${src}`);
    process.exit(1);
  }
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.copyFileSync(src, dest);
  if (!isWindows) {
    fs.chmodSync(dest, 0o755);
  }
  console.log(`[prepare-binaries] ${label} → ${path.relative(repoRoot, dest)}`);
}

if (buildCli) {
  console.log("[prepare-binaries] cargo build --release -p terrain-cli …");
  execFileSync("cargo", ["build", "--release", "-p", "terrain-cli"], {
    cwd: repoRoot,
    stdio: "inherit",
  });
}

const rtkPkg = platformPackageName("@terrain-ai/rtk");
const cliPkg = platformPackageName("@terrain-ai/cli");
const rtkBinary = isWindows ? "rtk.exe" : "rtk";
const terrainBinary = isWindows ? "terrain.exe" : "terrain";

const terrainRelease = path.join(
  repoRoot,
  "target/release",
  isWindows ? "terrain.exe" : "terrain",
);
const terrainSidecar = path.join(
  repoRoot,
  "packages/terrain",
  platform,
  terrainBinary,
);

copyBinary({
  label: "rtk",
  src: path.join(repoRoot, "packages/rtk", platform, rtkBinary),
  dest: path.join(npmRoot, "packages", rtkPkg.replace("@terrain-ai/", ""), "bin", rtkBinary),
});

if (fs.existsSync(terrainRelease)) {
  copyBinary({
    label: "terrain-cli (npm)",
    src: terrainRelease,
    dest: path.join(
      npmRoot,
      "packages",
      cliPkg.replace("@terrain-ai/", ""),
      "bin",
      terrainBinary,
    ),
  });
  fs.mkdirSync(path.dirname(terrainSidecar), { recursive: true });
  fs.copyFileSync(terrainRelease, terrainSidecar);
  if (!isWindows) {
    fs.chmodSync(terrainSidecar, 0o755);
  }
  console.log(
    `[prepare-binaries] terrain-cli (sidecar) → ${path.relative(repoRoot, terrainSidecar)}`,
  );
} else {
  console.warn(
    "[prepare-binaries] skip terrain-cli — run with --build-cli or cargo build --release -p terrain-cli",
  );
}

console.log(`[prepare-binaries] done (platform=${platform})`);
