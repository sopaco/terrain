import fs from "node:fs";
import path from "node:path";

/**
 * @param {{ outFile: string; tool: string; packagePrefix: string }} opts
 */
export function writeShim({ outFile, tool, packagePrefix }) {
  const dir = path.dirname(outFile);
  fs.mkdirSync(dir, { recursive: true });
  const source = `#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const key = \`\${process.platform}-\${process.arch}\`;
const pkg = \`${packagePrefix}-\${key}\`;
const candidates =
  process.platform === "win32"
    ? [\`\${pkg}/bin/${tool}.exe\`, \`\${pkg}/bin/${tool}\`]
    : [\`\${pkg}/bin/${tool}\`];
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
    \`${packagePrefix}: no prebuilt binary for \${key}.\\n\` +
      "Install Terrain desktop, run env integration (~/.terrain/bin), or publish the matching platform package.\",
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
`;
  fs.writeFileSync(outFile, source, { mode: 0o755 });
}
