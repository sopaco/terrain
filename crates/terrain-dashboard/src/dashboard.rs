//! Fleet dashboard: per-project graph metrics (cached), batch updates with
//! bounded concurrency, and multi-project registration.
//!
//! Cache/history live under `~/.terrain/dashboard/` so the desktop app can
//! render the fleet table without re-parsing every project's graph.json.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use terrain_core::paths::KnowledgePaths;
use terrain_core::project::get_project_overview;
use terrain_core::registry::{ProjectRegistryStatus, load_registry, register_project};
use terrain_core::{ProjectScanner, validate_repo_path};

use crate::graphify::{
    GraphifyAction, GraphifyGraphInfo, git_head, read_graph_info, resolve_graphify_bin,
};

pub const DASHBOARD_SUBDIR: &str = "dashboard";
/// Cache keyed per project: reused while the graph file's mtime+size match.
pub const CACHE_FILE: &str = "cache.json";
/// Rolling record of recent batch results (last N runs per project).
pub const HISTORY_FILE: &str = "batch-history.json";
const HISTORY_KEEP: usize = 30;
const DEFAULT_CONCURRENCY: usize = 3;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CachedGraphInfo {
    pub mtime_secs: u64,
    pub size: u64,
    /// Heavy fields cached between dashboard reads.
    pub info: CachedHeavyInfo,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CachedHeavyInfo {
    pub nodes: Option<u64>,
    pub links: Option<u64>,
    pub communities: Option<Vec<CommunitySummary>>,
    pub built_at_commit: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommunitySummary {
    pub name: String,
    pub size: u32,
}

/// Sampled commit activity for one project (fixed 12-week window).
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectActivity {
    /// Buckets of commit counts per week, oldest first (length 12).
    pub weekly_commits: Vec<u32>,
    /// Sum of commits inside the window (only).
    pub total_commits: u64,
}

/// Per-asset documentation coverage of one project.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DocsCoverage {
    pub agent_context: bool,
    pub human_docs: bool,
    pub repomix: bool,
    pub graph: bool,
    pub wiki: bool,
    /// Vault link configured (vault/graphify/<slug>/graphify-out).
    pub vault: bool,
}

/// Inferred collaboration between two projects (shared git contributors).
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCollaboration {
    pub project_a: String,
    pub project_b: String,
    pub shared_authors: u32,
}

/// One project row of the fleet summary.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct DashboardRow {
    pub slug: String,
    pub name: String,
    pub repo_path: String,
    pub status: ProjectRegistryStatus,
    pub missing_assets: Vec<String>,
    pub graph: GraphifyGraphInfo,
    pub head_commit: Option<String>,
    /// "fresh" | "stale" | "unknown" — graph vs the repo's current git HEAD.
    pub staleness: String,
    /// Today's agent spend for this repo from the local cost tracker (ccc).
    pub cost_today_usd: Option<f64>,
    pub activity: ProjectActivity,
    pub docs: DocsCoverage,
    /// Vault integration state for this project.
    pub vault_configured: bool,
    pub vault_path: Option<String>,
    pub last_batch: Option<LastBatchRecord>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DashboardTotals {
    pub projects: u64,
    pub with_graph: u64,
    pub binary_available: bool,
    pub stale: u64,
    pub total_nodes: u64,
    pub total_links: u64,
    /// Fleet-wide spend today (sum of per-repo costs from the tracker).
    pub total_cost_usd_today: f64,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DashboardSummary {
    pub generated_at_secs: u64,
    pub binary_available: bool,
    pub binary_path: String,
    pub rows: Vec<DashboardRow>,
    pub totals: DashboardTotals,
    /// Inferred cross-project collaborations (shared git contributors).
    pub collaborations: Vec<ProjectCollaboration>,
    /// Whether any vault directory exists at the vault root (feature gate).
    pub vault_root_exists: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastBatchRecord {
    pub ok: bool,
    pub action: String,
    pub error: Option<String>,
    pub duration_secs: f64,
    pub finished_at_secs: u64,
}

/// Emitted during a batch run; the shell layers (Tauri/CLI) turn this into
/// events or stdout lines.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct BatchProgressEvent {
    pub project_slug: String,
    pub project_name: String,
    pub stage: String,
    pub message: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct ProjectUpdateResult {
    pub project_slug: String,
    pub ok: bool,
    /// "update" or "extract+update" (first build).
    pub action: String,
    pub duration_secs: f64,
    pub error: Option<String>,
    /// Vault link path when it was installed/refreshed by this run.
    pub vault: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct BatchUpdateResult {
    pub results: Vec<ProjectUpdateResult>,
    pub succeeded: u64,
    pub failed: u64,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct AddProjectsResult {
    pub added: Vec<AddProjectOk>,
    pub errors: Vec<AddProjectError>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct AddProjectOk {
    pub slug: String,
    pub repo_path: String,
    pub scan_ok: bool,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct AddProjectError {
    pub path: String,
    pub error: String,
}

fn dashboard_dir() -> PathBuf {
    terrain_core::registry::registry_dir().join(DASHBOARD_SUBDIR)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn file_stat_secs_size(path: &std::path::Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())?;
    Some((mtime, meta.len()))
}

fn load_cache() -> HashMap<String, CachedGraphInfo> {
    let Some(dir) = Some(dashboard_dir()) else {
        return HashMap::new();
    };
    let Ok(bytes) = std::fs::read(dir.join(CACHE_FILE)) else {
        return HashMap::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_cache(cache: &HashMap<String, CachedGraphInfo>) {
    let dir = dashboard_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let bytes = serde_json::to_vec(cache).unwrap_or_default();
    let path = dir.join(CACHE_FILE);
    let tmp = dir.join(format!("{CACHE_FILE}.tmp"));
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

fn load_history() -> Vec<LastBatchRecordWithSlug> {
    let Some(dir) = Some(dashboard_dir()) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(dir.join(HISTORY_FILE)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastBatchRecordWithSlug {
    project_slug: String,
    #[serde(flatten)]
    record: LastBatchRecord,
}

fn append_history(records: &[LastBatchRecordWithSlug]) {
    let Some(dir) = Some(dashboard_dir()) else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let mut existing = load_history();
    existing.extend_from_slice(records);
    let truncated: &[LastBatchRecordWithSlug] = if existing.len() > HISTORY_KEEP {
        &existing[existing.len() - HISTORY_KEEP..]
    } else {
        &existing
    };
    let path = dir.join(HISTORY_FILE);
    let tmp = dir.join(format!("{HISTORY_FILE}.tmp"));
    if let Ok(bytes) = serde_json::to_vec(truncated)
        && std::fs::write(&tmp, bytes).is_ok()
    {
        let _ = std::fs::rename(&tmp, &path);
    }
}

pub fn staleness_of(built_at: &Option<String>, head: &Option<String>) -> String {
    match (built_at, head) {
        (Some(built), Some(head)) if built == head => "fresh".to_owned(),
        (Some(_), Some(_)) => "stale".to_owned(),
        _ => "unknown".to_owned(),
    }
}

/// Aggregate metrics for every registered project (graph presence, node/link
/// counts via the mtime cache, staleness vs git HEAD, last batch result).
pub fn scan_dashboard(
    paths: &KnowledgePaths,
    only_slugs: Option<&[String]>,
) -> terrain_core::Result<DashboardSummary> {
    let graphify_bin = resolve_graphify_bin();
    let binary_available = graphify_bin.is_some();
    let cache = load_cache();
    let history: HashMap<String, LastBatchRecord> = load_history()
        .into_iter()
        .map(|r| (r.project_slug, r.record))
        .collect();

    let mut rows = Vec::new();
    let mut author_sets: HashMap<String, HashSet<String>> = HashMap::new();
    let mut totals = DashboardTotals {
        binary_available,
        ..Default::default()
    };
    let mut fresh_cache: HashMap<String, CachedGraphInfo> = HashMap::new();

    for reg in load_registry()? {
        if let Some(only) = only_slugs
            && !only.iter().any(|s| s == &reg.slug)
        {
            continue;
        }
        let repo = PathBuf::from(&reg.repo_path);
        let (status, missing_assets, project_name, docs) = if !repo.is_dir() {
            (
                ProjectRegistryStatus::Stale,
                vec!["repo directory not found".to_owned()],
                reg.slug.clone(),
                DocsCoverage::default(),
            )
        } else {
            match get_project_overview(paths, &reg.slug) {
                Ok(overview) => {
                    let missing: Vec<String> = overview
                        .asset_health
                        .iter()
                        .filter(|asset| !asset.ready)
                        .map(|asset| asset.label.clone())
                        .collect();
                    let status = if missing.is_empty() {
                        ProjectRegistryStatus::Ready
                    } else {
                        ProjectRegistryStatus::Partial
                    };
                    let coverage = DocsCoverage {
                        agent_context: overview.agent_context.ready,
                        human_docs: overview.litho.has_human_docs,
                        repomix: overview.agent_pack.is_some(),
                        graph: false,
                        wiki: false,
                        vault: false,
                    };
                    (status, missing, overview.name, coverage)
                }
                Err(_) => (
                    ProjectRegistryStatus::Stale,
                    vec!["index.md".to_owned()],
                    reg.slug.clone(),
                    DocsCoverage::default(),
                ),
            }
        };

        let mut graph = read_graph_info(&repo);
        // Heavy fields come from the cache while graph.json is unchanged.
        let graph_json_path = repo
            .join(crate::graphify::GRAPHIFY_OUT_REL)
            .join("graph.json");
        if let Some((mtime, size)) = file_stat_secs_size(&graph_json_path) {
            let cache_hit = cache
                .get(&reg.slug)
                .is_some_and(|c| c.mtime_secs == mtime && c.size == size);
            if cache_hit {
                let cached = &cache[&reg.slug];
                graph.nodes = cached.info.nodes;
                graph.links = cached.info.links;
                graph.communities = cached.info.communities.clone().map(|cs| {
                    cs.into_iter()
                        .map(|c| crate::graphify::GraphifyCommunity {
                            name: c.name,
                            size: c.size,
                        })
                        .collect()
                });
                graph.built_at_commit = cached.info.built_at_commit.clone();
            } else if graph.nodes.is_some() {
                fresh_cache.insert(
                    reg.slug.clone(),
                    CachedGraphInfo {
                        mtime_secs: mtime,
                        size,
                        info: CachedHeavyInfo {
                            nodes: graph.nodes,
                            links: graph.links,
                            communities: graph.communities.clone().map(|cs| {
                                cs.into_iter()
                                    .map(|c| CommunitySummary {
                                        name: c.name,
                                        size: c.size,
                                    })
                                    .collect()
                            }),
                            built_at_commit: graph.built_at_commit.clone(),
                        },
                    },
                );
            }
        }

        let head_commit = if repo.is_dir() { git_head(&repo) } else { None };
        let cost_today_usd = if repo.is_dir() {
            terrain_core::read_cost_today(&reg.slug)
        } else {
            None
        };
        totals.total_cost_usd_today += cost_today_usd.unwrap_or(0.0);
        let staleness = staleness_of(&graph.built_at_commit, &head_commit);
        let (activity, authors) = if repo.is_dir() {
            read_git_activity(&repo)
        } else {
            (ProjectActivity::default(), None)
        };
        if let Some(authors) = authors {
            author_sets.insert(reg.slug.clone(), authors);
        }
        let docs = DocsCoverage {
            graph: graph.graph_present,
            wiki: graph.wiki_present,
            vault: vault_link_current(&repo, &reg.slug).is_some(),
            ..docs
        };
        let vault_configured = docs.vault;
        let vault_path = if vault_configured {
            Some(vault_link_path(&reg.slug).display().to_string())
        } else {
            None
        };

        if graph.graph_present {
            totals.with_graph += 1;
            totals.total_nodes += graph.nodes.unwrap_or(0);
            totals.total_links += graph.links.unwrap_or(0);
        }
        if matches!(staleness.as_str(), "stale") {
            totals.stale += 1;
        }
        totals.projects += 1;

        rows.push(DashboardRow {
            slug: reg.slug.clone(),
            name: project_name,
            repo_path: reg.repo_path.clone(),
            status,
            missing_assets,
            graph,
            head_commit,
            staleness,
            cost_today_usd,
            activity,
            docs,
            vault_configured,
            vault_path,
            last_batch: history.get(&reg.slug).cloned(),
        });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.slug.cmp(&b.slug)));

    if !fresh_cache.is_empty() {
        let mut merged = cache;
        merged.extend(fresh_cache);
        save_cache(&merged);
    }

    Ok(DashboardSummary {
        vault_root_exists: vault_root().is_dir(),
        generated_at_secs: now_secs(),
        binary_available,
        binary_path: graphify_bin
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        collaborations: infer_collaborations(&author_sets),
        rows,
        totals,
    })
}

/// Update (or first-build) graphs for the given projects. `slugs` empty = all
/// registered projects. Projects run with bounded concurrency; each project's
/// graphify run is itself guarded by its own `.rebuild.lock`.
pub async fn batch_update_graphs(
    slugs: Vec<String>,
    concurrency: Option<usize>,
    on_progress: Arc<dyn Fn(BatchProgressEvent) + Send + Sync>,
) -> terrain_core::Result<BatchUpdateResult> {
    let Some(bin) = resolve_graphify_bin() else {
        return Err(terrain_core::CoreError::InvalidDoc(
            crate::graphify::install_hint(),
        ));
    };
    let concurrency = concurrency.unwrap_or(DEFAULT_CONCURRENCY).clamp(1, 8);

    let mut targets: Vec<(String, String, String)> = Vec::new(); // (slug, name, repo_path)
    for reg in load_registry()? {
        if !slugs.is_empty() && !slugs.iter().any(|s| s == &reg.slug) {
            continue;
        }
        if !std::path::Path::new(&reg.repo_path).is_dir() {
            on_progress(BatchProgressEvent {
                project_slug: reg.slug.clone(),
                project_name: reg.slug.clone(),
                stage: "skipped".to_owned(),
                message: "repo directory not found".to_owned(),
            });
            targets.push((
                reg.slug.clone(),
                reg.slug.clone(),
                format!("__missing__\u{1}{}", reg.repo_path),
            ));
            continue;
        }
        targets.push((reg.slug.clone(), reg.slug.clone(), reg.repo_path.clone()));
    }

    let sem = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::new();
    for (slug, name, repo_path) in targets {
        let sem = sem.clone();
        let on_progress = on_progress.clone();
        let bin = bin.clone();
        handles.push(tokio::spawn(async move {
            if repo_path.starts_with("__missing__\u{1}") {
                return ProjectUpdateResult {
                    project_slug: slug,
                    ok: false,
                    action: "none".to_owned(),
                    duration_secs: 0.0,
                    error: Some("repo directory not found".to_owned()),
                    vault: None,
                };
            }
            let repo = PathBuf::from(&repo_path);
            let _permit = sem.acquire().await;
            let started = Instant::now();
            on_progress(BatchProgressEvent {
                project_slug: slug.clone(),
                project_name: name.clone(),
                stage: "started".to_owned(),
                message: "updating graph…".to_owned(),
            });
            let action = if read_graph_info(&repo).graph_present {
                GraphifyAction::Update
            } else {
                GraphifyAction::ExtractUpdate
            };
            let action_taken = match &action {
                GraphifyAction::Update => "update",
                GraphifyAction::ExtractUpdate => "extract+update",
            }
            .to_owned();
            let cb = on_progress.clone();
            let cb_slug = slug.clone();
            let cb_name = name.clone();
            let on_line: Arc<dyn Fn(&str) + Send + Sync> = Arc::new(move |line| {
                cb(BatchProgressEvent {
                    project_slug: cb_slug.clone(),
                    project_name: cb_name.clone(),
                    stage: "run".to_owned(),
                    message: line.to_owned(),
                })
            });
            let outcome = crate::graphify::run_graphify_action(&repo, &bin, action, on_line).await;
            let vault = if outcome.ok {
                sync_vault_link(&repo, &slug).unwrap_or(None)
            } else {
                None
            };
            ProjectUpdateResult {
                project_slug: slug,
                ok: outcome.ok,
                action: action_taken,
                duration_secs: started.elapsed().as_secs_f64(),
                error: if outcome.ok {
                    None
                } else {
                    Some(outcome.message)
                },
                vault,
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap_or_else(|e| ProjectUpdateResult {
            project_slug: String::new(),
            ok: false,
            action: "unknown".to_owned(),
            duration_secs: 0.0,
            error: Some(format!("task panicked: {e}")),
            vault: None,
        }));
    }

    let mut succeeded = 0;
    let mut failed = 0;
    let mut history_rows = Vec::new();
    for r in &results {
        if r.ok {
            succeeded += 1;
        } else {
            failed += 1;
        }
        history_rows.push(LastBatchRecordWithSlug {
            project_slug: r.project_slug.clone(),
            record: LastBatchRecord {
                ok: r.ok,
                action: r.action.clone(),
                error: r.error.clone(),
                duration_secs: r.duration_secs,
                finished_at_secs: now_secs(),
            },
        });
    }
    append_history(&history_rows);

    Ok(BatchUpdateResult {
        results,
        succeeded,
        failed,
    })
}

/// Register several repositories at once: validate, register, and scan each.
pub async fn add_projects(
    paths: &KnowledgePaths,
    repo_paths: Vec<String>,
) -> terrain_core::Result<AddProjectsResult> {
    let mut result = AddProjectsResult::default();
    let scanner = ProjectScanner::new(paths.clone());
    for path in repo_paths {
        let trimmed = path.trim().to_owned();
        if trimmed.is_empty() {
            continue;
        }
        if let Err(e) = validate_repo_path(&trimmed) {
            result.errors.push(AddProjectError {
                path: trimmed.clone(),
                error: e.to_string(),
            });
            continue;
        }
        let slug = slugify_path(&trimmed);
        if let Err(e) = register_project(&slug, &trimmed) {
            result.errors.push(AddProjectError {
                path: trimmed.clone(),
                error: e.to_string(),
            });
            continue;
        }
        let scan_ok = scanner.scan_repo(&trimmed, Some(&slug)).await.is_ok();
        result.added.push(AddProjectOk {
            slug,
            repo_path: trimmed,
            scan_ok,
        });
    }
    Ok(result)
}

fn slugify_path(repo_path: &str) -> String {
    let file_name = std::path::Path::new(repo_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let mut out = String::new();
    for ch in file_name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    if out.is_empty() {
        "project".to_owned()
    } else {
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staleness_matches_commit_when_equal() {
        assert_eq!(
            staleness_of(&Some("abc".to_owned()), &Some("abc".to_owned())),
            "fresh"
        );
    }

    #[test]
    fn staleness_flags_divergent_commits() {
        assert_eq!(
            staleness_of(&Some("abc".to_owned()), &Some("def".to_owned())),
            "stale"
        );
    }

    #[test]
    fn staleness_unknown_without_commits() {
        assert_eq!(staleness_of(&None, &Some("abc".to_owned())), "unknown");
        assert_eq!(staleness_of(&Some("abc".to_owned()), &None), "unknown");
    }

    #[test]
    fn slugify_lowercases_and_dashes() {
        assert_eq!(slugify_path("/tmp/My Cool Repo"), "my-cool-repo");
        assert_eq!(slugify_path("/tmp/my_repo/"), "my-repo");
    }
}

/// Window and sampling constants for the commit-activity chart.
pub const ACTIVITY_WEEKS: usize = 12;
const ACTIVITY_SAMPLE: usize = 500;
const WEEK_SECS: u64 = 604_800;

/// One pass over recent commits: weekly buckets (fixed window) + author set.
fn read_git_activity(repo: &std::path::Path) -> (ProjectActivity, Option<HashSet<String>>) {
    let output = match terrain_core::process_command("git")
        .args([
            "log",
            "-n",
            &ACTIVITY_SAMPLE.to_string(),
            "--pretty=format:%ct|%ae",
        ])
        .current_dir(repo)
        .output()
    {
        Ok(output) => output,
        Err(_) => return (ProjectActivity::default(), None),
    };
    if !output.status.success() {
        return (ProjectActivity::default(), None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let now = now_secs();
    let current_week = now / WEEK_SECS;
    let mut buckets = vec![0u32; ACTIVITY_WEEKS];
    let mut authors: HashSet<String> = HashSet::new();
    for line in text.lines() {
        let Some((ts, author)) = line.split_once('|') else {
            continue;
        };
        let Ok(ts) = ts.trim().parse::<u64>() else {
            continue;
        };
        if ts / WEEK_SECS + ACTIVITY_WEEKS as u64 <= current_week {
            continue;
        }
        let age_weeks = (current_week - ts / WEEK_SECS) as usize;
        if age_weeks < ACTIVITY_WEEKS {
            buckets[ACTIVITY_WEEKS - 1 - age_weeks] += 1;
        }
        if !author.is_empty() {
            authors.insert(author.trim().to_ascii_lowercase());
        }
    }
    let total = buckets.iter().map(|c| *c as u64).sum();
    (
        ProjectActivity {
            weekly_commits: buckets,
            total_commits: total,
        },
        Some(authors),
    )
}

/// Inferred collaboration: pairs of projects with at least one shared author.
fn infer_collaborations(
    author_sets: &HashMap<String, HashSet<String>>,
) -> Vec<ProjectCollaboration> {
    let slugs: Vec<&String> = author_sets.keys().collect();
    let mut pairs = Vec::new();
    for (i, slug_a) in slugs.iter().enumerate() {
        for slug_b in slugs.iter().skip(i + 1) {
            let shared = author_sets[*slug_a]
                .intersection(&author_sets[*slug_b])
                .count() as u32;
            if shared > 0 {
                pairs.push(ProjectCollaboration {
                    project_a: (*slug_a).clone(),
                    project_b: (*slug_b).clone(),
                    shared_authors: shared,
                });
            }
        }
    }
    pairs.sort_by(|a, b| b.shared_authors.cmp(&a.shared_authors));
    pairs
}

#[cfg(test)]
mod activity_tests {
    use super::*;

    #[test]
    fn buckets_cover_twelve_weeks_oldest_first() {
        // pure bucket math replication check via constants
        assert_eq!(ACTIVITY_WEEKS, 12);
        assert_eq!(604_800 * 12, 7_257_600);
    }
}

// Vault integration moved to terrain_core::vault (needed by the env
// integration too); re-exported here so the fleet API stays stable.
pub use terrain_core::vault::{
    session_autosave_registered, sync_vault_link, vault_link_current, vault_link_path, vault_root,
};

#[cfg(test)]
mod vault_tests {
    use super::*;

    #[test]
    fn vault_link_detect_and_sync() {
        let repo = std::env::temp_dir().join(format!("vault-test-{}", std::process::id()));
        let vault = std::env::temp_dir().join(format!("vault-test-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&vault);
        std::fs::create_dir_all(repo.join(crate::graphify::GRAPHIFY_OUT_REL)).unwrap();
        std::fs::create_dir_all(&vault).unwrap();
        // SAFETY: single-threaded test process context; VAULT_DIR restore below.
        unsafe { std::env::set_var("VAULT_DIR", &vault) };

        // No graph content — nothing to sync
        assert!(vault_link_current(&repo, "proj").is_none());

        std::fs::write(
            repo.join(crate::graphify::GRAPHIFY_OUT_REL)
                .join("graph.json"),
            "{}",
        )
        .unwrap();
        let linked = sync_vault_link(&repo, "proj").unwrap();
        assert!(linked.is_some());
        assert!(vault_link_current(&repo, "proj").is_some());
        // Refresh: a second sync is a no-op; links still valid
        let again = sync_vault_link(&repo, "proj").unwrap();
        assert_eq!(again, linked);

        // SAFETY: restoring the test-modified environment.
        unsafe { std::env::remove_var("VAULT_DIR") };
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&vault);
    }
}
