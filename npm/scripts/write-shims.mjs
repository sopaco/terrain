#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { writeShim } from "./lib/write-shim.mjs";

const npmRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

writeShim({
  outFile: path.join(npmRoot, "packages/rtk/bin/rtk.js"),
  tool: "rtk",
  packagePrefix: "@terrain-ai/rtk",
});

writeShim({
  outFile: path.join(npmRoot, "packages/cli/bin/terrain.js"),
  tool: "terrain",
  packagePrefix: "@terrain-ai/cli",
});

console.log("[write-shims] updated bin shims");
