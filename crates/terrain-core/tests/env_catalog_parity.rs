//! Guard: `env-catalog/` is the single source of truth for the agent contract.
//!
//! The same catalog is materialized in several places: the platform npm package (shipped to
//! users), `target/debug` (what `~/.terrain/env-catalog` symlinks to in this dev tree), and the
//! release app bundle (what `init_app_env_catalog` reads *first* before re-pointing that symlink).
//! A copy left behind at an older version does not fail loudly — the desktop app simply deploys
//! the stale catalog to home and the next `env apply` rolls `AGENTS.md` and the Skills back.
//!
//! These tests make "edit the source, forget a copy" a compile-time-style failure instead of a
//! silent rollback. Build-output copies are checked when present and skipped on a clean CI tree.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use terrain_core::fingerprint::fingerprint;

const SKILL_DIRS: [&str; 4] = [
    "terrain-knowledge-skill",
    "repomix-context-skill",
    "codegraph-skill",
    "rtk-skill",
];

/// Copies that must always be in sync with `env-catalog/` (both are version-controlled).
const REQUIRED_COPIES: [&str; 1] = ["npm/packages/cli-darwin-arm64/env-catalog"];

/// Build outputs. Tauri / npm packaging refresh them, but a local release tree can still be
/// read at runtime today, so check them whenever they exist.
const OPTIONAL_COPIES: [&str; 5] = [
    // Platform packages other than the host's are staged by npm/scripts/prepare-binaries.mjs
    // on their own build machines, so they are absent from this checkout most of the time.
    "npm/packages/cli-win32-x64/env-catalog",
    "target/debug/env-catalog",
    "target/release/env-catalog",
    "target/release/bundle/macos/Terrain.app/Contents/Resources/env-catalog",
    // The installed app is what `~/.terrain/bin/terrain` and the home catalog symlink read
    // from; a stale copy here silently reverts AGENTS.md on the next apply. Absent on CI.
    "/Applications/Terrain.app/Contents/Resources/env-catalog",
];

fn workspace_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2)?;
    // No-op when the crate is built outside the monorepo (e.g. a published tarball).
    root.join("AGENTS.md").is_file().then(|| root.to_path_buf())
}

/// Every managed file below `dir`, keyed by its slash-separated relative path.
/// Dotfiles (`.DS_Store`, editor state) are local noise and are deliberately ignored.
fn managed_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    collect_managed_files(dir, dir, &mut out);
    out
}

fn collect_managed_files(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_managed_files(base, &path, out);
        } else if let Ok(bytes) = fs::read(&path) {
            let key = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(key, bytes);
        }
    }
}

fn compare_copy(root: &Path, rel: &str, required: bool, source: &BTreeMap<String, Vec<u8>>) {
    let copy = root.join(rel);
    if !copy.join("catalog.json").is_file() {
        assert!(
            !required,
            "shipped env catalog copy is missing: {} (copy it from env-catalog/)",
            copy.display()
        );
        return;
    }

    let actual = managed_files(&copy);
    let mut problems = Vec::new();
    for (key, expected) in source {
        match actual.get(key) {
            None => problems.push(format!("{key}: missing from copy")),
            Some(bytes) if bytes != expected => problems.push(format!("{key}: content differs")),
            Some(_) => {}
        }
    }
    for key in actual.keys() {
        if !source.contains_key(key) {
            problems.push(format!("{key}: stale extra file in copy"));
        }
    }

    assert!(
        problems.is_empty(),
        "env-catalog copy '{}' is out of sync with env-catalog/ — sync all nine managed files \
         (catalog.json, four fragments, four SKILL.md) or the deployed agent contract will drift:\n  {}",
        rel,
        problems.join("\n  ")
    );
}

#[test]
fn catalog_copies_match_source() {
    let Some(root) = workspace_root() else {
        return;
    };
    let source = managed_files(&root.join("env-catalog"));
    assert!(
        source.contains_key("catalog.json") && source.len() >= 9,
        "env-catalog/ looks incomplete: {} managed files",
        source.len()
    );

    for rel in REQUIRED_COPIES {
        compare_copy(&root, rel, true, &source);
    }
    for rel in OPTIONAL_COPIES {
        compare_copy(&root, rel, false, &source);
    }
}

#[test]
fn deployed_skills_match_source_and_manifest() {
    let Some(root) = workspace_root() else {
        return;
    };
    let mut problems = Vec::new();

    for skill in SKILL_DIRS {
        let src = root.join("env-catalog/skills").join(skill).join("SKILL.md");
        let expected =
            fs::read(&src).unwrap_or_else(|e| panic!("missing source skill {skill}: {e}"));
        let expected_fp = fingerprint(&expected);

        for base in [".agents/skills", ".claude/skills"] {
            let dir = root.join(base).join(skill);
            let deployed = dir.join("SKILL.md");
            match fs::read(&deployed) {
                Ok(bytes) if bytes == expected => {}
                Ok(_) => problems.push(format!(
                    "{base}/{skill}/SKILL.md: content differs from source"
                )),
                Err(_) => problems.push(format!(
                    "{base}/{skill}/SKILL.md: missing (run `terrain env apply`)"
                )),
            }

            // An absent or stale manifest makes sync_skill_dir treat the next upgrade as a
            // "user edit" and permanently keep the old SKILL.md.
            let manifest = dir.join(".terrain-skill.json");
            match fs::read_to_string(&manifest)
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(s.as_str()).ok())
            {
                Some(v) => {
                    if v["files"]["SKILL.md"].as_str() != Some(expected_fp.as_str()) {
                        problems.push(format!(
                            "{base}/{skill}/.terrain-skill.json: fingerprint is stale"
                        ));
                    }
                }
                None => problems.push(format!(
                    "{base}/{skill}/.terrain-skill.json: missing or invalid"
                )),
            }
        }
    }

    assert!(
        problems.is_empty(),
        "deployed Skills are out of sync with env-catalog/:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn catalog_references_resolve_and_agents_md_is_current() {
    let Some(root) = workspace_root() else {
        return;
    };
    let catalog: Value = serde_json::from_slice(
        &fs::read(root.join("env-catalog/catalog.json")).expect("catalog.json is readable"),
    )
    .expect("catalog.json is valid JSON");
    let agents_md = fs::read_to_string(root.join("AGENTS.md")).expect("AGENTS.md is readable");
    let mut problems = Vec::new();

    if let Some(items) = catalog["integrations"].as_array() {
        for item in items.iter().filter(|i| i["kind"] == "skill") {
            if let Some(dir) = item["skill_dir"].as_str() {
                let path = root.join("env-catalog/skills").join(dir).join("SKILL.md");
                if !path.is_file() {
                    problems.push(format!("integration references missing skill: {dir}"));
                }
                let text = fs::read_to_string(&path).unwrap_or_default();
                if !text.starts_with("---\n")
                    || !text.contains("\nname:")
                    || !text.contains("\ndescription:")
                {
                    problems.push(format!("{dir}/SKILL.md: missing valid frontmatter"));
                }
            }
        }
    }

    if let Some(blocks) = catalog["agents_md_blocks"].as_array() {
        for block in blocks {
            let id = block["id"].as_str().unwrap_or_default();
            let version = block["version"].as_u64().unwrap_or_default();
            let fragment = block["fragment"].as_str().unwrap_or_default();
            if !root.join("env-catalog").join(fragment).is_file() {
                problems.push(format!(
                    "block {id} references missing fragment: {fragment}"
                ));
            }
            let marker = format!("<!-- terrain:begin {id} v{version} -->");
            if !agents_md.contains(&marker) {
                problems.push(format!(
                    "AGENTS.md has no '{marker}' — bump the block version and run `terrain env apply`"
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "env catalog references or AGENTS.md managed blocks are stale:\n  {}",
        problems.join("\n  ")
    );
}
