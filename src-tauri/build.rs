use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=MIND_MESH_TARGET_TRIPLE={target}");
    println!("cargo:rerun-if-changed=../packages/rtk/darwin-arm64/rtk");
    println!("cargo:rerun-if-changed=../packages/mind-mesh/darwin-arm64/mind-mesh");
    println!("cargo:rerun-if-changed=../packages/codegraph/darwin-arm64");

    if target == "aarch64-apple-darwin" {
        stage_sidecar("rtk", "../packages/rtk/darwin-arm64/rtk");
        stage_sidecar("mind-mesh-cli", "../packages/mind-mesh/darwin-arm64/mind-mesh");
    }

    tauri_build::build()
}

fn stage_sidecar(name: &str, source_rel: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let target = env::var("TARGET").expect("TARGET");
    let source = Path::new(&manifest_dir).join(source_rel);
    let dest_dir = Path::new(&manifest_dir).join("binaries");
    let dest = dest_dir.join(format!("{name}-{target}"));

    if !source.is_file() {
        println!(
            "cargo:warning=missing bundled sidecar source {} (skip staging {name})",
            source.display()
        );
        return;
    }

    fs::create_dir_all(&dest_dir).expect("create binaries dir");
    fs::copy(&source, &dest).unwrap_or_else(|e| {
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

    println!("cargo:rerun-if-changed={}", dest.display());
}
