#!/usr/bin/env node
/** Sync npm package versions from Cargo workspace version. */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const npmRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(npmRoot, "..");
const cargoToml = fs.readFileSync(path.join(repoRoot, "Cargo.toml"), "utf8");
const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
if (!match) {
  console.error("Could not read workspace version from Cargo.toml");
  process.exit(1);
}
const version = match[1];
const packagesDir = path.join(npmRoot, "packages");

for (const name of fs.readdirSync(packagesDir)) {
  const pkgPath = path.join(packagesDir, name, "package.json");
  if (!fs.existsSync(pkgPath)) continue;
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  pkg.version = version;
  if (pkg.optionalDependencies) {
    for (const dep of Object.keys(pkg.optionalDependencies)) {
      pkg.optionalDependencies[dep] = version;
    }
  }
  fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`[sync-version] ${pkg.name} → ${version}`);
}
