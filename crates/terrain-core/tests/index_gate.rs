//! End-to-end coverage for the index freshness gate.
//!
//! The pack is a snapshot at `baseline_git_head`. A source file added after that commit cannot
//! have a slice in it, yet the age/commit heuristics saturate at small caps and still reported
//! a green score (88 / `stale: false`) for exactly that state. `compute_freshness` now names the
//! missing paths and caps the verdict in the warn band.
//!
//! All scenarios live in one `#[test]`: they share the process-wide `TERRAIN_REGISTRY_FILE`
//! override, which is what keeps the developer's real `~/.terrain/registry.json` untouched.

use std::fs;
use std::path::Path;

use terrain_core::freshness::{
    compute_freshness, format_freshness_trust_block, read_freshness_ledger,
    resolve_freshness_summary, write_freshness_ledger,
};
use terrain_core::paths::KnowledgePaths;
use terrain_core::process_command;
use terrain_core::schema::FreshnessSummary;
use terrain_core::{FRESH_THRESHOLD, VERIFY_THRESHOLD};

const NEW_SOURCE: &str = "crates/terrain-core/src/assets/new_module.rs";
const UNREACHABLE: &str = "0000000000000000000000000000000000000000";

fn git(repo: &Path, args: &[&str]) -> String {
    let out = process_command("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn init_repo(repo: &Path) -> String {
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.email", "t@test.com"]);
    git(repo, &["config", "user.name", "t"]);
    fs::write(repo.join("main.rs"), "fn main() {}\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "init"]);
    git(repo, &["rev-parse", "HEAD"])
}

/// A body that satisfies `context_body_ready` (>= 500 chars, >= 4 `## ` sections).
fn context_body() -> String {
    let mut body = String::from("# Architecture\n\n");
    for section in ["Modules", "Flows", "Boundaries", "Dependencies"] {
        body.push_str(&format!("## {section}\n\n"));
        body.push_str(
            &"Narrative for the module map, the core flows and the system boundaries. ".repeat(3),
        );
        body.push('\n');
    }
    body
}

/// Ready-to-read assets whose pack was generated at `baseline`.
fn write_assets(repo: &Path, slug: &str, baseline: &str, now: &str) {
    let paths = KnowledgePaths::for_repo(repo);
    let (pack_meta, pack_main) = (paths.agent_pack_meta(slug), paths.agent_pack_main(slug));
    let (ctx_meta, ctx_main) = (paths.agent_context_meta(slug), paths.agent_context_main(slug));
    for path in [&pack_meta, &pack_main, &ctx_meta, &ctx_main] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
    }

    fs::write(
        &pack_meta,
        format!(
            r#"{{
  "project": "{slug}",
  "repo_path": ".",
  "generator": "repomix-core",
  "pack_strategy": "architecture-context-v3",
  "output_file": "repomix.md",
  "total_files": 24,
  "total_tokens": 1000,
  "total_characters": 4000,
  "top_files_by_tokens": [],
  "directory_structure": "src/",
  "synced_at": "{now}",
  "baseline_git_head": "{baseline}"
}}"#
        ),
    )
    .unwrap();
    fs::write(&pack_main, "# Repomix\n\n### main.rs\n\n```rust\nfn main() {}\n```\n").unwrap();
    fs::write(
        &ctx_meta,
        format!(
            r#"{{
  "project": "{slug}",
  "repo_path": ".",
  "output_file": "context.md",
  "generated_at": "{now}",
  "section_count": 4,
  "char_count": 900,
  "baseline_git_head": "{baseline}",
  "language": "en"
}}"#
        ),
    )
    .unwrap();
    fs::write(&ctx_main, context_body()).unwrap();
}

fn factor_ids(summary: &FreshnessSummary) -> Vec<&str> {
    summary.drift_factors.iter().map(|f| f.id.as_str()).collect()
}

/// `mode`: `NewSource` commits a source file after the pack baseline, `Clean` leaves HEAD at
/// the baseline, `Unreachable` records a baseline that no longer resolves.
fn run(slug: &str, mode: Mode) -> (FreshnessSummary, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    let head = init_repo(repo);
    let repo_s = repo.display().to_string();
    terrain_core::registry::register_project(slug, &repo_s).expect("temporary project registers");

    let baseline = if matches!(mode, Mode::Unreachable) {
        UNREACHABLE
    } else {
        head.as_str()
    };
    let now = chrono::Utc::now().to_rfc3339();
    write_assets(repo, slug, baseline, &now);

    if matches!(mode, Mode::NewSource) {
        let file = repo.join(NEW_SOURCE);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "pub fn added_later() {}\n").unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "add a new source module"]);
    }

    let paths = KnowledgePaths::for_repo(repo);
    let summary = compute_freshness(&paths, slug, &repo_s).expect("freshness computes");
    (summary, dir)
}

#[derive(Clone, Copy)]
enum Mode {
    NewSource,
    Clean,
    Unreachable,
}

#[test]
fn index_gate_names_missing_sources_and_fails_closed_on_unreachable_baselines() {
    let registry_dir = tempfile::tempdir().unwrap();
    let registry_file = registry_dir.path().join("registry.json");
    // SAFETY: this test binary runs a single `#[test]`, so no other thread reads the environment
    // concurrently; the variable is only consumed by the registry lookup below.
    unsafe { std::env::set_var("TERRAIN_REGISTRY_FILE", &registry_file) };

    // --- Gate case: a source file was added after the pack baseline ---
    let (gated, _keep) = run("gate-new-source", Mode::NewSource);
    assert_eq!(
        gated.overall_score,
        VERIFY_THRESHOLD - 1,
        "the gate caps the verdict in the warn band: {gated:?}"
    );
    assert!(gated.overall_stale, "{gated:?}");
    assert_eq!(
        gated.stale_reason.as_deref(),
        Some("index_missing_new_sources"),
        "{gated:?}"
    );
    assert!(!gated.macro_preload_allowed, "{gated:?}");
    // `read_agent_context` gates its warning on agent_context_score, not overall_score.
    assert!(gated.agent_context_score < FRESH_THRESHOLD, "{gated:?}");

    let ids = factor_ids(&gated);
    assert!(ids.contains(&"index_missing_new_sources"), "{gated:?}");
    let factor = gated
        .drift_factors
        .iter()
        .find(|f| f.id == "index_missing_new_sources")
        .expect("named factor");
    assert_eq!(factor.severity, "high");
    assert!(factor.detail.contains("new_module.rs"), "{}", factor.detail);

    let block = format_freshness_trust_block(&gated);
    assert!(
        block.contains("stale_reason: index_missing_new_sources"),
        "{block}"
    );

    // --- Control: index generated at HEAD, no new sources ---
    let (control, _keep2) = run("gate-control", Mode::Clean);
    assert!(
        !factor_ids(&control).contains(&"index_missing_new_sources"),
        "{control:?}"
    );
    assert!(control.macro_preload_allowed, "{control:?}");
    assert!(control.overall_score >= FRESH_THRESHOLD, "{control:?}");
    assert!(!control.overall_stale, "{control:?}");

    // --- Fail-closed: an unreachable baseline keeps its own diagnosis ---
    let (unreachable, _keep3) = run("gate-unreachable", Mode::Unreachable);
    assert!(
        !factor_ids(&unreachable).contains(&"index_missing_new_sources"),
        "an unreachable baseline must not be double-charged: {unreachable:?}"
    );
    assert_eq!(
        unreachable.stale_reason.as_deref(),
        Some("baseline_unreachable"),
        "{unreachable:?}"
    );
    assert!(factor_ids(&unreachable).contains(&"baseline_unreachable"));

    // --- Cache invalidation: a ledger written by older scoring rules must be recomputed ---
    let slug = "gate-ledger-version";
    let (_clean, keep4) = run(slug, Mode::Clean);
    let paths = KnowledgePaths::for_repo(keep4.path());
    let mut ledger = read_freshness_ledger(&paths, slug).expect("compute writes a ledger");
    ledger.version = 1; // produced by the previous binary
    ledger.summary.overall_score = 0; // sentinel: a cached verdict would return 0
    write_freshness_ledger(&paths, slug, &ledger).unwrap();
    let recomputed = resolve_freshness_summary(
        &paths,
        slug,
        &keep4.path().display().to_string(),
    )
    .expect("freshness recomputes");
    assert_ne!(
        recomputed.overall_score, 0,
        "a stale-version ledger must not be served from cache"
    );
    assert!(recomputed.overall_score >= FRESH_THRESHOLD, "{recomputed:?}");
}
