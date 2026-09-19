use std::env;
use std::fs;
use std::io::{Read, Seek};
use std::path::Path;
use std::time::Duration;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TERRAIN_TARGET_TRIPLE={target}");
    let platform = platform_key_for_target(&target);
    println!("cargo:rerun-if-changed=../packages/rtk/{platform}");
    println!("cargo:rerun-if-changed=../packages/terrain/{platform}");
    println!("cargo:rerun-if-changed=../packages/codegraph/{platform}");
    println!("cargo:rerun-if-changed=../packages/codegraph/VERSION");
    println!("cargo:rerun-if-env-changed=TERRAIN_SKIP_CODEGRAPH_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=TERRAIN_SKIP_SIDECAR_DOWNLOAD");
    println!("cargo:rerun-if-changed=../preset_skills");
    println!("cargo:rerun-if-changed=../env-catalog");

    stage_sidecar(&target, platform, "rtk", &format!("../packages/rtk/{platform}/rtk"));
    stage_sidecar(
        &target,
        platform,
        "terrain-cli",
        &format!("../packages/terrain/{platform}/terrain"),
    );
    stage_codegraph_resource(platform);

    tauri_build::build()
}

fn platform_key_for_target(target: &str) -> &'static str {
    match target {
        "aarch64-apple-darwin" => "darwin-arm64",
        "x86_64-apple-darwin" => "darwin-x64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" | "x86_64-pc-windows-gnullvm" => "win32-x64",
        "aarch64-pc-windows-msvc" => "win32-arm64",
        "x86_64-unknown-linux-gnu" => "linux-x64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        _ => "darwin-arm64",
    }
}

fn sidecar_source_name(base_rel: &str, target: &str) -> String {
    if target.contains("windows") {
        format!("{base_rel}.exe")
    } else {
        base_rel.to_string()
    }
}

fn is_release_profile() -> bool {
    env::var("PROFILE").map(|p| p == "release").unwrap_or(false)
}

/// Copy a single file, first removing any existing file at `to`.
///
/// `fs::copy` overwrites an existing destination *in place* (open + truncate +
/// write), which reuses the same inode and therefore keeps whatever extended
/// attributes it already carried — notably a stale `com.apple.quarantine` flag
/// from an earlier bad download, which macOS Gatekeeper will still act on even
/// though the file's content is now a freshly-verified, correct binary. Removing
/// the destination first forces a new inode with no inherited metadata.
fn copy_file_fresh(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = fs::remove_file(to);
    fs::copy(from, to)?;
    Ok(())
}

/// Shared blocking HTTP GET used by both the sidecar and CodeGraph auto-download
/// paths (no `npm`/`node`/`curl` required on the build machine).
fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(180))
        .call()
        .map_err(|e| format!("GET {url} failed: {e}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read response body from {url}: {e}"))?;
    Ok(bytes)
}

/// Sniff the target platform a compiled binary was built for, from its file header
/// (Mach-O magic + cputype for macOS, PE machine field for Windows) — independent
/// of which `packages/<tool>/<platform>/` folder it happens to sit in. This is how
/// a wrong-architecture binary silently vendored into the wrong slot (e.g. an
/// x86_64 build placed under `darwin-arm64/`) gets caught instead of shipping.
/// Returns `None` for formats we don't recognize (e.g. ELF) — callers treat that as
/// "can't verify, don't block".
fn sniff_binary_platform(path: &Path) -> Option<&'static str> {
    let mut head = [0u8; 64];
    let mut f = fs::File::open(path).ok()?;
    let n = f.read(&mut head).ok()?;
    if n < 8 {
        return None;
    }
    // Mach-O 64-bit, on-disk little-endian magic (both Apple Silicon and Intel Macs
    // write native-byte-order MH_MAGIC_64 = 0xfeedfacf -> on-disk bytes CF FA ED FE).
    if head[0..4] == [0xCF, 0xFA, 0xED, 0xFE] {
        let cputype = i32::from_le_bytes(head[4..8].try_into().unwrap());
        return match cputype {
            0x0100_000C => Some("darwin-arm64"), // CPU_TYPE_ARM64
            0x0100_0007 => Some("darwin-x64"),   // CPU_TYPE_X86_64
            _ => None,
        };
    }
    // PE (Windows): "MZ" stub, e_lfanew at offset 0x3C points to the "PE\0\0" header,
    // immediately followed by IMAGE_FILE_HEADER.Machine.
    if n >= 0x40 && head[0..2] == *b"MZ" {
        let pe_off = u32::from_le_bytes(head[0x3C..0x40].try_into().unwrap()) as u64;
        let mut f2 = fs::File::open(path).ok()?;
        f2.seek(std::io::SeekFrom::Start(pe_off)).ok()?;
        let mut header = [0u8; 6];
        f2.read_exact(&mut header).ok()?;
        if header[0..4] == *b"PE\0\0" {
            let machine = u16::from_le_bytes(header[4..6].try_into().unwrap());
            return match machine {
                0x8664 => Some("win32-x64"),
                0xAA64 => Some("win32-arm64"),
                0x014c => Some("win32-ia32"),
                _ => None,
            };
        }
    }
    None
}

/// A vendored sidecar binary is valid when it exists and — for formats we can
/// sniff — its detected architecture matches the platform folder it's in. Unknown
/// formats (e.g. ELF, which we don't currently vendor) only require existence.
fn sidecar_binary_is_valid(path: &Path, platform: &str) -> bool {
    if !path.is_file() {
        return false;
    }
    match sniff_binary_platform(path) {
        Some(detected) => detected == platform,
        None => true,
    }
}

/// Maps a sidecar's build.rs `name` to the npm scope package Terrain publishes a
/// prebuilt mirror under (see `npm/README.md` / `npm/packages/`). `rtk`'s package
/// keeps the `rtk` prefix; `terrain-cli`'s ships under the `cli` prefix.
fn sidecar_npm_package(name: &str, platform: &str) -> Option<String> {
    let prefix = match name {
        "rtk" => "rtk",
        "terrain-cli" => "cli",
        _ => return None,
    };
    Some(format!("{prefix}-{platform}"))
}

fn npm_dist_tag_latest(pkg_full_name: &str) -> Result<String, String> {
    let url = format!("https://registry.npmjs.org/{pkg_full_name}");
    let body = http_get_bytes(&url)?;
    let json: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| format!("failed to parse npm metadata from {url}: {e}"))?;
    json.get("dist-tags")
        .and_then(|v| v.get("latest"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("no dist-tags.latest in npm metadata from {url}"))
}

/// Download `@terrain-ai/<npm_pkg>` (Terrain's own prebuilt mirror of `rtk`/
/// `terrain-cli`) and copy its single `bin/<name>` binary over `dest`.
fn download_sidecar_from_npm_mirror(dest: &Path, npm_pkg: &str) -> Result<(), String> {
    let version = npm_dist_tag_latest(&format!("@terrain-ai/{npm_pkg}"))?;
    let url = format!("https://registry.npmjs.org/@terrain-ai/{npm_pkg}/-/{npm_pkg}-{version}.tgz");
    let bytes = http_get_bytes(&url)?;

    let parent = dest.parent().unwrap_or(dest);
    let tmp_dir = parent.join(format!(".sidecar-download-{npm_pkg}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let result = (|| -> Result<(), String> {
        let gz = flate2::read::GzDecoder::new(bytes.as_slice());
        tar::Archive::new(gz)
            .unpack(&tmp_dir)
            .map_err(|e| format!("failed to unpack {url}: {e}"))?;

        let bin_name = dest
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "sidecar destination has no file name".to_string())?;
        let extracted_bin = tmp_dir.join("package/bin").join(bin_name);
        if !extracted_bin.is_file() {
            return Err(format!(
                "expected {} in downloaded {npm_pkg}@{version} tarball, not found",
                extracted_bin.display()
            ));
        }

        fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        copy_file_fresh(&extracted_bin, dest).map_err(|e| {
            format!("failed to copy {} -> {}: {e}", extracted_bin.display(), dest.display())
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(dest)
                .map_err(|e| format!("stat {}: {e}", dest.display()))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(dest, perms).map_err(|e| format!("chmod {}: {e}", dest.display()))?;
        }
        Ok(())
    })();

    let _ = fs::remove_dir_all(&tmp_dir);
    result?;

    println!("cargo:warning=downloaded {npm_pkg}@{version} from npm and vendored it as {}", dest.display());
    Ok(())
}

/// Auto-heal a missing or wrong-architecture vendored sidecar binary by downloading
/// Terrain's own npm mirror package for it. Opt out with
/// `TERRAIN_SKIP_SIDECAR_DOWNLOAD=1` (e.g. fully offline builds) — `stage_sidecar`
/// still fails a release build outright if the binary remains invalid either way.
fn ensure_sidecar_binary(source: &Path, platform: &str, name: &str) {
    if sidecar_binary_is_valid(source, platform) {
        return;
    }
    if env::var("TERRAIN_SKIP_SIDECAR_DOWNLOAD").is_ok_and(|v| v == "1") {
        println!(
            "cargo:warning={name} sidecar for {platform} is missing/wrong-architecture and \
             TERRAIN_SKIP_SIDECAR_DOWNLOAD=1 is set, skipping auto-download"
        );
        return;
    }
    let Some(npm_pkg) = sidecar_npm_package(name, platform) else {
        return;
    };
    println!(
        "cargo:warning={name} sidecar for {platform} is missing or has the wrong architecture \
         at {}, downloading @terrain-ai/{npm_pkg} from the npm registry…",
        source.display()
    );
    if let Err(e) = download_sidecar_from_npm_mirror(source, &npm_pkg) {
        println!("cargo:warning=auto-download of {name} sidecar for {platform} failed: {e}");
    }
}

/// Stage a bundled CLI (`rtk` or `terrain-cli`) as a Tauri `externalBin` sidecar.
///
/// Before copying, the vendored source binary is verified to actually be built for
/// `platform` (see `sidecar_binary_is_valid`) — a binary can be present yet wrong
/// (e.g. an x86_64 build vendored into the `darwin-arm64/` slot runs fine under
/// Rosetta locally but is the wrong artifact to ship, and can outright fail under
/// Gatekeeper if it also carries a stale quarantine flag). When invalid, we try to
/// auto-heal via `ensure_sidecar_binary` before deciding whether to stage it.
fn stage_sidecar(target: &str, platform: &str, name: &str, base_rel: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let source_rel = sidecar_source_name(base_rel, target);
    let mut source = Path::new(&manifest_dir).join(&source_rel);
    if !source.is_file() {
        source = Path::new(&manifest_dir).join(base_rel);
    }

    ensure_sidecar_binary(&source, platform, name);

    if !sidecar_binary_is_valid(&source, platform) {
        let msg = format!(
            "{name} sidecar for {platform} is missing or has the wrong architecture at {} \
             (skip staging {name})",
            source.display()
        );
        if is_release_profile() {
            panic!("{msg} — refusing to ship a release build with a broken {name} sidecar.");
        }
        println!("cargo:warning={msg}");
        return;
    }

    let dest_dir = Path::new(&manifest_dir).join("binaries");
    let dest_name = sidecar_source_name(&format!("{name}-{target}"), target);
    let dest = dest_dir.join(dest_name);

    fs::create_dir_all(&dest_dir).expect("create binaries dir");

    let source_bytes = fs::read(&source).unwrap_or_else(|e| {
        panic!("failed to read sidecar {}: {e}", source.display())
    });
    if fs::read(&dest).ok().as_deref() == Some(source_bytes.as_slice()) {
        return;
    }

    let _ = fs::remove_file(&dest);
    fs::write(&dest, &source_bytes).unwrap_or_else(|e| {
        panic!(
            "failed to stage sidecar {} -> {}: {e}",
            source.display(),
            dest.display()
        )
    });

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest).expect("sidecar metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).expect("chmod sidecar");
    }
}

/// Stage the per-platform CodeGraph bundle (vendored Node runtime + wrapper +
/// compiled JS) into `bundled-resources/codegraph`, which `tauri.conf.json`
/// bundles as `tools/codegraph` inside the app.
///
/// The bundle is checked for completeness before staging: a bundle missing
/// `lib/dist` or `lib/node_modules` (e.g. because it was only partially
/// vendored) still has a working `bin/codegraph` wrapper script, so it looks
/// present but fails at runtime with `MODULE_NOT_FOUND` the moment an agent
/// runs `codegraph init`. When incomplete, we try to auto-heal by downloading
/// the matching `@colbymchenry/codegraph-<platform>` package straight from
/// the npm registry (see `ensure_codegraph_bundle`) before staging.
fn stage_codegraph_resource(platform: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let source = Path::new(&manifest_dir).join(format!("../packages/codegraph/{platform}"));
    let dest = Path::new(&manifest_dir).join("bundled-resources/codegraph");

    ensure_codegraph_bundle(&source, platform);

    if !codegraph_bundle_is_complete(&source) {
        let msg = format!(
            "codegraph bundle for {platform} is missing or incomplete at {} \
             (skip staging; CodeGraph will be unavailable to agents in this build)",
            source.display()
        );
        if is_release_profile() {
            panic!(
                "{msg} — refusing to ship a release build with a broken CodeGraph bundle. \
                 Check network access to registry.npmjs.org (auto-download is attempted \
                 automatically unless TERRAIN_SKIP_CODEGRAPH_DOWNLOAD=1), or vendor \
                 packages/codegraph/{platform}/ manually."
            );
        }
        println!("cargo:warning={msg}");
        return;
    }

    if dest.exists() {
        let _ = fs::remove_dir_all(&dest);
    }
    copy_dir_recursive(&source, &dest).unwrap_or_else(|e| {
        panic!(
            "failed to stage codegraph {} -> {}: {e}",
            source.display(),
            dest.display()
        )
    });
}

/// Required members of a complete per-platform CodeGraph bundle: the vendored
/// Node runtime, the `bin/codegraph` wrapper, the compiled CLI entrypoint, and
/// its `node_modules`. All four must be present or the wrapper execs a JS file
/// that doesn't exist.
fn codegraph_bundle_is_complete(dir: &Path) -> bool {
    let node_ok = dir.join("node").is_file() || dir.join("node.exe").is_file();
    let bin_ok = ["bin/codegraph", "bin/codegraph.cmd", "bin/codegraph.exe"]
        .iter()
        .any(|rel| dir.join(rel).is_file());
    let js_ok = dir.join("lib/dist/bin/codegraph.js").is_file();
    let modules_ok = fs::read_dir(dir.join("lib/node_modules"))
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    node_ok && bin_ok && js_ok && modules_ok
}

/// Auto-heal a missing/incomplete vendored CodeGraph bundle by downloading the
/// matching npm platform package. Opt out with `TERRAIN_SKIP_CODEGRAPH_DOWNLOAD=1`
/// (e.g. fully offline builds) — `stage_codegraph_resource` still fails the build
/// outright for a release profile if the bundle remains incomplete either way, so
/// packaging can never silently ship a broken CodeGraph.
fn ensure_codegraph_bundle(dir: &Path, platform: &str) {
    if codegraph_bundle_is_complete(dir) {
        return;
    }
    if env::var("TERRAIN_SKIP_CODEGRAPH_DOWNLOAD").is_ok_and(|v| v == "1") {
        println!(
            "cargo:warning=codegraph bundle for {platform} is incomplete and \
             TERRAIN_SKIP_CODEGRAPH_DOWNLOAD=1 is set, skipping auto-download"
        );
        return;
    }
    println!(
        "cargo:warning=codegraph bundle for {platform} is missing/incomplete at {}, \
         downloading @colbymchenry/codegraph-{platform} from the npm registry…",
        dir.display()
    );
    if let Err(e) = download_codegraph_bundle(dir, platform) {
        println!("cargo:warning=auto-download of codegraph bundle for {platform} failed: {e}");
    }
}

/// Pinned CodeGraph npm package version, read from `packages/codegraph/VERSION`
/// (kept alongside the vendored bundles so bumping the version and re-vendoring
/// stay in sync). Falls back to the last known-good version if the file is
/// missing.
fn codegraph_bundle_version() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let version_file = Path::new(&manifest_dir).join("../packages/codegraph/VERSION");
    fs::read_to_string(&version_file)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "1.1.0".to_string())
}

/// Download `@colbymchenry/codegraph-<platform>@<version>` directly from the npm
/// registry tarball URL (no `npm`/`node` required on the build machine) and unpack
/// it into `dir`, merging over whatever is already there.
fn download_codegraph_bundle(dir: &Path, platform: &str) -> Result<(), String> {
    let version = codegraph_bundle_version();
    let pkg_basename = format!("codegraph-{platform}");
    let url = format!(
        "https://registry.npmjs.org/@colbymchenry/{pkg_basename}/-/{pkg_basename}-{version}.tgz"
    );

    let bytes = http_get_bytes(&url)?;

    let tmp_dir = dir.parent().unwrap_or(dir).join(format!(
        ".codegraph-download-{platform}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let unpack_result = (|| -> Result<(), String> {
        let gz = flate2::read::GzDecoder::new(bytes.as_slice());
        tar::Archive::new(gz)
            .unpack(&tmp_dir)
            .map_err(|e| format!("failed to unpack {url}: {e}"))?;

        // npm tarballs always unpack under a single top-level "package/" dir.
        let extracted = tmp_dir.join("package");
        if !extracted.is_dir() {
            return Err(format!(
                "unexpected tarball layout: no package/ under {}",
                tmp_dir.display()
            ));
        }

        fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {e}", dir.display()))?;
        copy_dir_recursive(&extracted, dir)
            .map_err(|e| format!("failed to copy {} -> {}: {e}", extracted.display(), dir.display()))
    })();

    let _ = fs::remove_dir_all(&tmp_dir);
    unpack_result?;

    println!("cargo:warning=downloaded codegraph {version} bundle for {platform} from npm");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            copy_file_fresh(&from, &to)?;
            #[cfg(unix)]
            if from.extension().is_none() || from.extension().is_some_and(|e| e != "json") {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(&from) {
                    let mode = meta.permissions().mode();
                    if mode & 0o111 != 0 {
                        let mut perms = fs::metadata(&to)?.permissions();
                        perms.set_mode(mode);
                        fs::set_permissions(&to, perms)?;
                    }
                }
            }
        }
    }
    Ok(())
}
