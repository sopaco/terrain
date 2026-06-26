#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const key = `${process.platform}-${process.arch}`;
const pkg = `@terrain-ai/rtk-${key}`;
const candidates =
  process.platform === "win32"
    ? [`${pkg}/bin/rtk.exe`, `${pkg}/bin/rtk`]
    : [`${pkg}/bin/rtk`];
let binary;
for (const candidate of candidates) {
  try {
    binary = require.resolve(candidate);
    break;
  } catch {
    // try next candidate
  }
}
if (!binary) {
  console.error(
    `@terrain-ai/rtk: no prebuilt binary for ${key}.\n` +
      "Install Terrain desktop, run env integration (~/.terrain/bin), or publish the matching platform package.",
  );
  process.exit(1);
}
const args = process.argv.slice(2);
try {
  execFileSync(binary, args, { stdio: "inherit" });
} catch (err) {
  const code = typeof err.status === "number" ? err.status : 1;
  process.exit(code);
}
