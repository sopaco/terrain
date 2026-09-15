//! End-to-end freshness scoring through its public entry point.
//!
//! These drive `compute_freshness` itself (not just the git/scoring helpers) so a regression in
//! the wiring between them — which is where the reported gap lived — fails here.
//!
//! Context: a baseline commit recorded in `agent/meta.json` stops resolving after a rebase,
//! squash, amend, force-push or shallow clone. `git log <baseline>..HEAD` then exits non-zero,
//! and its empty output used to be indistinguishable from "nothing changed", so the assets
//! scored ~90 and stayed `stale: false` exactly when drift could not be measured.
//!
//! All three scenarios live in one `#[test]`: they share the process-wide registry override
//! (`TERRAIN_REGISTRY_FILE`), and Rust's env access is process-global, so separate `#[test]`s
//! racing on it would be flaky. The override is what keeps the developer's real
//! `~/.terrain/registry.json` untouched.

use std::fs;
use std::path::Path;
use std::process::Command;

use terrain_core::freshness::{compute_freshness, format_freshness_trust_block};
use terrain_core::paths::KnowledgePaths;
use terrain_core::schema::FreshnessSummary;
use terrain_core::{FRESH_THRESHOLD, MACRO_PRELOAD_THRESHOLD};

/// A syntactically valid commit hash that does not exist — the same effect on `git log` as a
/// baseline rewritten away by a rebase or dropped by a shallow clone.
const UNREACHABLE: &str = "0000000000000000000000000000000000000000";

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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

/// Git repo with one commit; returns its HEAD.
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
        let narrative =
            "Narrative for the module map, the core flows and the system boundaries. ".repeat(3);
        body.push_str(&narrative);
        body.push('\n');
    }
    body
}

/// Write a ready-to-read knowledge asset set recording `baseline` for every layer.
fn write_assets(repo: &Path, slug: &str, baseline: &str, synced_at: &str, generated_at: &str) {
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
  "pack_strategy": "architecture-context",
  "output_file": "repomix.md",
  "total_files": 24,
  "total_tokens": 1000,
  "total_characters": 4000,
  "top_files_by_tokens": [],
  "directory_structure": "src/",
  "synced_at": "{synced_at}",
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
  "generated_at": "{generated_at}",
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

/// Build a repo with the given asset state and score it through `compute_freshness`.
/// The returned `TempDir` must stay alive for the duration of the assertions.
fn run_scenario(
    slug: &str,
    head_baseline: bool,
    synced_at: &str,
    generated_at: &str,
) -> (FreshnessSummary, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    let head = init_repo(repo);
    let repo_s = repo.display().to_string();
    // Register before touching `paths`, whose accessors resolve through the registry.
    terrain_core::registry::register_project(slug, &repo_s).expect("temporary project registers");

    let baseline = if head_baseline { head.as_str() } else { UNREACHABLE };
    write_assets(repo, slug, baseline, synced_at, generated_at);

    let paths = KnowledgePaths::for_repo(repo);
    let summary = compute_freshness(&paths, slug, &repo_s).expect("freshness computes");
    (summary, dir)
}

#[test]
fn freshness_scoring_is_fail_closed_when_drift_cannot_be_measured() {
    // Point the registry at a scratch file so the developer's real one is never touched.
    let registry_dir = tempfile::tempdir().unwrap();
    let registry_file = registry_dir.path().join("registry.json");
    // SAFETY: this test binary runs a single `#[test]`, so no other thread reads the environment
    // concurrently, and the variable is only consumed by the registry lookup below.
    unsafe { std::env::set_var("TERRAIN_REGISTRY_FILE", &registry_file) };

    let now = chrono::Utc::now().to_rfc3339();

    // --- Scenario 1: the recorded baseline no longer resolves ---
    let (unreachable, _keep) = run_scenario("e2e-unreachable", false, &now, &now);
    // The assets themselves are present, so "missing" must not be conflated with "untrusted".
    assert!(unreachable.agent_pack_score > 0, "{unreachable:?}");
    assert!(
        unreachable.agent_pack_score < MACRO_PRELOAD_THRESHOLD,
        "{unreachable:?}"
    );
    assert!(!unreachable.macro_preload_allowed, "{unreachable:?}");
    assert!(unreachable.overall_score < FRESH_THRESHOLD, "{unreachable:?}");
    assert!(unreachable.overall_stale, "{unreachable:?}");
    assert_eq!(
        unreachable.stale_reason.as_deref(),
        Some("baseline_unreachable"),
        "{unreachable:?}"
    );
    // Every deducted point must be attributable to a named factor...
    assert!(
        factor_ids(&unreachable).contains(&"baseline_unreachable"),
        "{unreachable:?}"
    );
    // ...and the false "in sync with the baseline commit" claim must be gone.
    assert!(
        !factor_ids(&unreachable).contains(&"baseline_match"),
        "{unreachable:?}"
    );
    // The agent-facing trust block has to carry the same verdict.
    let block = format_freshness_trust_block(&unreachable);
    assert!(block.contains("stale_reason: baseline_unreachable"), "{block}");
    assert!(
        block.contains("DO NOT rely on preloaded macro context"),
        "{block}"
    );

    // --- Scenario 2 (control): a baseline that resolves is genuinely fresh ---
    let (fresh, _keep2) = run_scenario("e2e-fresh", true, &now, &now);
    assert_eq!(fresh.stale_reason, None, "{fresh:?}");
    assert!(!fresh.overall_stale, "{fresh:?}");
    assert!(fresh.macro_preload_allowed, "{fresh:?}");
    assert!(fresh.overall_score >= FRESH_THRESHOLD, "{fresh:?}");
    assert!(factor_ids(&fresh).contains(&"baseline_match"), "{fresh:?}");

    // --- Scenario 3: drift is measurable, but the sync timestamp is unparseable ---
    let (bad_time, _keep3) = run_scenario("e2e-bad-time", true, "not-a-timestamp", "not-a-timestamp");
    // An unknown age is charged the age cap; it must not outscore a genuinely today-fresh sync,
    // which would otherwise let a corrupted timestamp buy the full 90-point ceiling.
    assert!(bad_time.overall_score < 90, "{bad_time:?}");
    assert!(
        factor_ids(&bad_time).contains(&"sync_time_unknown"),
        "{bad_time:?}"
    );
    assert!(!factor_ids(&bad_time).contains(&"pack_age"), "{bad_time:?}");
}
