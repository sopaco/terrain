use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TERRAIN_TARGET_TRIPLE={target}");
    let platform = platform_key_for_target(&target);
    println!("cargo:rerun-if-changed=../packages/rtk/{platform}");
    println!("cargo:rerun-if-changed=../packages/terrain/{platform}");
    println!("cargo:rerun-if-changed=../packages/codegraph/{platform}");
    println!("cargo:rerun-if-changed=../preset_skills");

    stage_sidecar(&target, "rtk", &format!("../packages/rtk/{platform}/rtk"));
    stage_sidecar(
        &target,
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

fn stage_sidecar(target: &str, name: &str, base_rel: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let source_rel = sidecar_source_name(base_rel, target);
    let source = Path::new(&manifest_dir).join(&source_rel);
    let source = if source.is_file() {
        source
    } else {
        Path::new(&manifest_dir).join(base_rel)
    };
    let dest_dir = Path::new(&manifest_dir).join("binaries");
    let dest_name = sidecar_source_name(&format!("{name}-{target}"), target);
    let dest = dest_dir.join(dest_name);

    if !source.is_file() {
        println!(
            "cargo:warning=missing bundled sidecar source {} (skip staging {name})",
            source.display()
        );
        return;
    }

    fs::create_dir_all(&dest_dir).expect("create binaries dir");

    let source_bytes = fs::read(&source).unwrap_or_else(|e| {
        panic!("failed to read sidecar {}: {e}", source.display())
    });
    if fs::read(&dest).ok().as_deref() == Some(source_bytes.as_slice()) {
        return;
    }

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

fn stage_codegraph_resource(platform: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let source = Path::new(&manifest_dir).join(format!("../packages/codegraph/{platform}"));
    let dest = Path::new(&manifest_dir).join("bundled-resources/codegraph");
    if !source.is_dir() {
        println!(
            "cargo:warning=missing codegraph bundle {} (skip staging)",
            source.display()
        );
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

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
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
