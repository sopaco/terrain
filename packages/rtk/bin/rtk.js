#!/usr/bin/env node
/**
 * Wrapper that runs RTK from vendor/ or delegates to PATH when globally installed.
 */
const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const root = path.join(__dirname, "..");
const vendor = path.join(root, "vendor");
const candidates = [
  path.join(vendor, process.platform === "win32" ? "rtk.exe" : "rtk"),
  path.join(vendor, "rtk", process.platform === "win32" ? "rtk.exe" : "rtk"),
];

function findVendorBinary() {
  for (const p of candidates) {
    if (fs.existsSync(p)) return p;
  }
  const entries = fs.existsSync(vendor) ? fs.readdirSync(vendor) : [];
  for (const name of entries) {
    const full = path.join(vendor, name);
    if (name === "rtk" || name === "rtk.exe") return full;
  }
  return null;
}

const vendorBin = findVendorBinary();
const program = vendorBin ?? "rtk";

const result = spawnSync(program, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});
process.exit(result.status ?? 1);
