#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const key = `${process.platform}-${process.arch}`;
const pkg = `@terrain-ai/cli-${key}`;
let binary;
try {
  binary = require.resolve(`${pkg}/bin/terrain`);
} catch {
  console.error(
    `@terrain-ai/cli: no prebuilt binary for ${key}.\n` +
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
