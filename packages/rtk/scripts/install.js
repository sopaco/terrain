#!/usr/bin/env node
/**
 * Download RTK prebuilt binary from GitHub releases into vendor/.
 */
const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const RTK_VERSION = "v0.42.4";
const ROOT = path.join(__dirname, "..");
const VENDOR = path.join(ROOT, "vendor");

function platformAsset() {
  const p = process.platform;
  const a = process.arch;
  if (p === "darwin" && a === "arm64") {
    return { archive: `rtk-aarch64-apple-darwin.tar.gz`, bin: "rtk" };
  }
  if (p === "darwin" && a === "x64") {
    return { archive: `rtk-x86_64-apple-darwin.tar.gz`, bin: "rtk" };
  }
  if (p === "linux" && a === "arm64") {
    return { archive: `rtk-aarch64-unknown-linux-gnu.tar.gz`, bin: "rtk" };
  }
  if (p === "linux" && a === "x64") {
    return { archive: `rtk-x86_64-unknown-linux-musl.tar.gz`, bin: "rtk" };
  }
  if (p === "win32" && a === "x64") {
    return { archive: `rtk-x86_64-pc-windows-msvc.zip`, bin: "rtk.exe" };
  }
  return null;
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https
      .get(url, { headers: { "User-Agent": "mind-mesh-rtk-installer" } }, (res) => {
        if (res.statusCode === 302 || res.statusCode === 301) {
          file.close();
          fs.unlinkSync(dest);
          return download(res.headers.location, dest).then(resolve).catch(reject);
        }
        if (res.statusCode !== 200) {
          file.close();
          fs.unlinkSync(dest);
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
      })
      .on("error", reject);
  });
}

async function main() {
  if (commandExists("rtk")) {
    console.log("[ @mind-mesh/rtk ] rtk already on PATH — skipping download");
    return;
  }

  const asset = platformAsset();
  if (!asset) {
    console.warn("[ @mind-mesh/rtk ] Unsupported platform; install RTK manually from https://github.com/rtk-ai/rtk");
    return;
  }

  fs.mkdirSync(VENDOR, { recursive: true });
  const destBin = path.join(VENDOR, asset.bin);
  if (fs.existsSync(destBin)) {
    if (process.platform !== "win32") fs.chmodSync(destBin, 0o755);
    return;
  }

  const url = `https://github.com/rtk-ai/rtk/releases/download/${RTK_VERSION}/${asset.archive}`;
  const archivePath = path.join(VENDOR, asset.archive);

  console.log(`[ @mind-mesh/rtk ] Downloading ${url}`);
  try {
    await download(url, archivePath);
    if (asset.archive.endsWith(".tar.gz")) {
      execSync(`tar -xzf "${archivePath}" -C "${VENDOR}"`, { stdio: "inherit" });
    } else if (asset.archive.endsWith(".zip")) {
      execSync(`unzip -o "${archivePath}" -d "${VENDOR}"`, { stdio: "inherit" });
    }
    fs.unlinkSync(archivePath);
    if (fs.existsSync(destBin) && process.platform !== "win32") {
      fs.chmodSync(destBin, 0o755);
    }
    console.log(`[ @mind-mesh/rtk ] Installed to ${destBin}`);
  } catch (e) {
    console.warn(`[ @mind-mesh/rtk ] Download failed: ${e.message}`);
    console.warn("  Install manually: brew install rtk-ai/tap/rtk");
  }
}

function commandExists(name) {
  try {
    execSync(process.platform === "win32" ? `where ${name}` : `command -v ${name}`, {
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

main();
