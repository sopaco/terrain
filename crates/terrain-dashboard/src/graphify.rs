//! Drive the external `graphify` CLI: binary resolution, graph.json summary, and
//! phased runs with a generic progress callback. The Tauri/CLI layers map the
//! callback into events or stdout.
//!
//! Terrain standardizes graphify output at `.terrain/graphify-out/` (via the
//! GRAPHIFY_OUT env var, which graphify resolves relative to the scan root), so
//! the graph ships with the repo like the rest of the `.terrain/` knowledge assets.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use serde::Serialize;

/// Graphify output dir, relative to the scan root (the repo).
pub const GRAPHIFY_OUT_REL: &str = ".terrain/graphify-out";
/// Skip counting/parsing graphs larger than this (serde_json::Value is memory-hungry).
const MAX_STATUS_GRAPH_BYTES: u64 = 48 * 1024 * 1024;

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Locate the graphify CLI: GRAPHIFY_BIN override → ~/.terrain/bin/ → PATH.
pub fn resolve_graphify_bin() -> Option<PathBuf> {
    if let Some(bin) = std::env::var_os("GRAPHIFY_BIN") {
        let path = PathBuf::from(bin);
        if path.is_file() {
            return Some(path);
        }
        // An explicit override that is missing wins and fails loudly.
        return None;
    }
    if let Some(home) = home_dir() {
        let base = home.join(".terrain").join("bin");
        let mut candidates = vec![base.join("graphify")];
        if cfg!(windows) {
            candidates.push(base.join("graphify.exe"));
            candidates.push(base.join("graphify.cmd"));
        }
        if let Some(path) = candidates.into_iter().find(|p| p.is_file()) {
            return Some(path);
        }
    }
    terrain_core::resolve_executable("graphify")
}

pub fn install_hint() -> String {
    "graphify CLI not found. Install it with `uv tool install graphifyy` or `pipx install graphifyy` (then reopen Terrain).".to_owned()
}

pub fn git_head(repo: &Path) -> Option<String> {
    let output = terrain_core::process_command("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_owned())
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct GraphifyCommunity {
    pub name: String,
    pub size: u32,
}

/// Presence flags + heavy stats for one project's graph, read from its outputs.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct GraphifyGraphInfo {
    pub graph_present: bool,
    pub report_present: bool,
    pub html_present: bool,
    pub wiki_present: bool,
    pub graph_mtime_secs: Option<u64>,
    /// None when graph.json is missing or too large to parse.
    pub nodes: Option<u64>,
    pub links: Option<u64>,
    pub communities: Option<Vec<GraphifyCommunity>>,
    pub built_at_commit: Option<String>,
}

/// Top-level stats from graph.json (no full deserialization of every field).
fn summarize_graph_json(
    graph_json: &Path,
) -> (
    Option<u64>,
    Option<u64>,
    Option<Vec<GraphifyCommunity>>,
    Option<String>,
) {
    let meta = match std::fs::metadata(graph_json) {
        Ok(m) => m,
        Err(_) => return (None, None, None, None),
    };
    if meta.len() > MAX_STATUS_GRAPH_BYTES {
        return (None, None, None, None);
    }
    let bytes = std::fs::read(graph_json).unwrap_or_default();
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (None, None, None, None);
    };
    let nodes = value
        .get("nodes")
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u64);
    let links = value
        .get("links")
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u64);
    let mut comm_counts: Vec<(String, u32)> = Vec::new();
    if let Some(node_list) = value.get("nodes").and_then(|v| v.as_array()) {
        for node in node_list {
            if let Some(name) = node.get("community_name").and_then(|v| v.as_str()) {
                if name.is_empty() {
                    continue;
                }
                match comm_counts.iter_mut().find(|(n, _)| n == name) {
                    Some((_, count)) => *count += 1,
                    None => comm_counts.push((name.to_owned(), 1)),
                }
            }
        }
    }
    comm_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    comm_counts.truncate(8);
    let communities = Some(
        comm_counts
            .into_iter()
            .map(|(name, size)| GraphifyCommunity { name, size })
            .collect(),
    );
    let built_at = value
        .get("built_at_commit")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    (nodes, links, communities, built_at)
}

pub enum GraphifyAction {
    /// First build: extract (lays down graph.json + caches), then update to
    /// produce the full artifact set (labels, GRAPH_REPORT.md, graph.html).
    ExtractUpdate,
    /// Incremental refresh (AST-only).
    Update,
}

pub struct GraphifyRunOutcome {
    pub ok: bool,
    pub message: String,
}

/// Stream graphify stdout/stderr lines into the progress callback (capped length).
async fn forward_output<R>(on_line: Arc<dyn Fn(&str) + Send + Sync>, stream: R)
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;

    let mut lines = tokio::io::BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut message = trimmed.to_owned();
        if message.len() > 200 {
            // Cap per-event payload; progress lines are informational only.
            let mut end = 200;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
            message.push('…');
        }
        on_line(&message);
    }
}

/// Run graphify phases for one repo, returning the outcome. Progress lines go
/// to `on_line` (per spawned reader); phase starts/failures are also reported.
pub async fn run_graphify_action(
    repo: &Path,
    bin: &Path,
    action: GraphifyAction,
    on_line: Arc<dyn Fn(&str) + Send + Sync>,
) -> GraphifyRunOutcome {
    let phases: Vec<&[&str]> = match action {
        GraphifyAction::ExtractUpdate => {
            vec![&["extract", ".", "--code-only"][..], &["update", "."][..]]
        }
        GraphifyAction::Update => vec![&["update", "."][..]],
    };

    for phase_args in phases {
        let display_cmd = format!("graphify {}", phase_args.join(" "));
        on_line(&format!("$ {display_cmd}"));

        let mut child = match terrain_core::async_command(bin)
            .args(phase_args)
            .current_dir(repo)
            .env("GRAPHIFY_OUT", GRAPHIFY_OUT_REL)
            .env("GRAPHIFY_NO_AUTO_REFRESH", "1")
            .env("GRAPHIFY_NO_TIPS", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                return GraphifyRunOutcome {
                    ok: false,
                    message: format!("Failed to launch graphify: {e}"),
                };
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let mut readers = Vec::new();
        if let Some(stdout) = stdout {
            readers.push(tokio::spawn(forward_output(on_line.clone(), stdout)));
        }
        if let Some(stderr) = stderr {
            readers.push(tokio::spawn(forward_output(on_line.clone(), stderr)));
        }

        let status = child.wait().await;
        for reader in readers {
            let _ = reader.await;
        }
        match status {
            Ok(status) if status.success() => continue,
            Ok(status) => {
                return GraphifyRunOutcome {
                    ok: false,
                    message: format!("{display_cmd} failed (exit {status})."),
                };
            }
            Err(e) => {
                return GraphifyRunOutcome {
                    ok: false,
                    message: format!("graphify failed: {e}"),
                };
            }
        }
    }

    GraphifyRunOutcome {
        ok: true,
        message: "Knowledge graph updated in .terrain/graphify-out/.".to_owned(),
    }
}

/// Read `.terrain/graphify-out/` presence flags + stats for one repo.
pub fn read_graph_info(repo: &Path) -> GraphifyGraphInfo {
    let out = repo.join(GRAPHIFY_OUT_REL);
    let graph_json = out.join("graph.json");
    let graph_present = graph_json.is_file();
    let mut info = GraphifyGraphInfo {
        graph_present,
        report_present: out.join("GRAPH_REPORT.md").is_file(),
        html_present: out.join("graph.html").is_file(),
        wiki_present: out.join("wiki/index.md").is_file(),
        graph_mtime_secs: None,
        nodes: None,
        links: None,
        communities: None,
        built_at_commit: None,
    };
    if graph_present {
        let (nodes, links, communities, built_at) = summarize_graph_json(&graph_json);
        info.nodes = nodes;
        info.links = links;
        info.communities = communities;
        info.built_at_commit = built_at;
        info.graph_mtime_secs = std::fs::metadata(&graph_json)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
    }
    info
}
