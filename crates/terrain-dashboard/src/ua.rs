//! Understand-Anything (UA) driver: status + viewer-server resolution.
//!
//! UA is an assistant-driven Claude Code plugin (no headless analysis — see
//! docs/CLONE_SPEC_UA-integration.md E20): Terrain's job is limited to
//! inspecting its artifacts and hosting its read-only viewer process.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use serde::Serialize;

use terrain_core::CoreError;

/// Canonical UA output dir at a project root (skill pipeline writes here).
pub const UA_DIR_REL: &str = ".ua";
/// Legacy UA output dir (wins for reads AND writes when it exists, E5).
pub const UA_LEGACY_DIR_REL: &str = ".understand-anything";
/// Terrain's memory-bank dir for UA knowledge markdown (index/report).
pub const UA_TERRAIN_DIR_REL: &str = ".terrain/understand";
/// Where the tool-understand integration clones + builds the UA repo.
pub const UA_REPO_REL: &str = "understand-anything/repo";

/// Large-JSON cap for status counting (mirrors graphify.rs MAX_STATUS_GRAPH_BYTES).
const MAX_STATUS_GRAPH_BYTES: u64 = 48 * 1024 * 1024;

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Locate the built UA repo checkout used for the viewer binary.
pub fn resolve_ua_repo() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("UA_REPO_DIR") {
        let path = PathBuf::from(dir);
        return path.is_dir().then_some(path);
    }
    home_dir()
        .map(|h| h.join(".terrain").join(UA_REPO_REL))
        .filter(|p| p.is_dir())
}

/// Locate the viewer entry script (`viewer.mjs`): UA_VIEWER_BIN override →
/// the built checkout → None. The viewer bundles its dashboard under dist/ at
/// build time (viewer build.mjs), so an un-built checkout does not count.
pub fn resolve_ua_viewer() -> Option<PathBuf> {
    if let Some(bin) = std::env::var_os("UA_VIEWER_BIN") {
        let path = PathBuf::from(bin);
        return path.is_file().then_some(path);
    }
    let script = resolve_ua_repo().map(|r| {
        r.join("understand-anything-plugin/packages/viewer/bin/viewer.mjs")
    })?;
    let dist = script.parent()?.join("dist");
    (script.is_file() && dist.is_dir()).then_some(script)
}

/// Resolve the node runtime and return it with its major version (viewer needs >= 18).
pub fn node_version() -> Option<(PathBuf, u32)> {
    let node = terrain_core::resolve_executable("node")?;
    let out = terrain_core::process_command(node.display().to_string().as_str())
        .args(["--version"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let major = text.trim().trim_start_matches('v').split('.').next()?;
    major.parse().ok().map(|m| (node, m))
}

/// Data dir of a repo per UA's own conventions: the viewer only serves
/// `.ua/` and legacy `.understand-anything/`; terrain's memory-bank dir is
/// probed last (status-only in v1).
pub fn ua_data_dir(repo: &Path) -> Option<(PathBuf, &'static str)> {
    [
        (repo.join(UA_DIR_REL), "ua" as &str),
        (repo.join(UA_LEGACY_DIR_REL), "legacy"),
        (repo.join(UA_TERRAIN_DIR_REL), "terrain"),
    ]
    .into_iter()
    .find(|(p, _)| p.join("knowledge-graph.json").is_file())
    .map(|(p, k)| (p, k as &'static str))
}

#[derive(Debug, Clone, Serialize)]
pub struct UaStatus {
    /// Directory kind found: "ua" | "legacy" | "terrain"; empty when none.
    pub data_dir: String,
    pub data_dir_path: String,
    pub graph_present: bool,
    pub meta_present: bool,
    pub fingerprints_present: bool,
    pub nodes: Option<u64>,
    pub links: Option<u64>,
    pub analyzed_at: Option<String>,
    pub built_at_commit: Option<String>,
    pub head_commit: Option<String>,
    pub staleness: String,
    pub plugin_available: bool,
    pub node_available: bool,
    pub node_major: Option<u32>,
    pub viewer_available: bool,
    pub viewer_path: String,
    pub install_steps_hint: String,
}

/// Probe a repo's UA artifacts + toolchain availability (graphify_status_cmd shape).
pub fn ua_status(repo: &Path) -> UaStatus {
    let head = crate::graphify::git_head(repo);
    let found = ua_data_dir(repo);
    let mut st_data_dir = String::new();
    let mut st_data_dir_path = String::new();
    let mut graph_present = false;
    let mut meta_present = false;
    let mut fingerprints_present = false;
    let mut nodes = None;
    let mut links = None;
    let mut analyzed_at = None;
    let mut built_at_commit = None;
    if let Some((dir, kind)) = found {
        let meta = read_dir_meta(&dir);
        let (n, l) = count_graph(&dir);
        st_data_dir = kind.to_owned();
        st_data_dir_path = dir.display().to_string();
        graph_present = true;
        meta_present = meta.is_some();
        fingerprints_present = dir.join("fingerprints.json").is_file();
        nodes = n;
        links = l;
        analyzed_at = meta.as_ref().and_then(|m| m.last_analyzed_at.clone());
        built_at_commit = meta.as_ref().and_then(|m| m.git_commit_hash.clone());
    }
    let staleness = crate::dashboard::staleness_of(&built_at_commit, &head);
    let (node_available, node_major) = match node_version() {
        Some((_, major)) => (true, Some(major)),
        None => (false, None),
    };
    let viewer = resolve_ua_viewer();
    let viewer_runnable = viewer.is_some() && node_available && node_major.is_some_and(|m| m >= 18);
    UaStatus {
        data_dir: st_data_dir,
        data_dir_path: st_data_dir_path,
        graph_present,
        meta_present,
        fingerprints_present,
        nodes,
        links,
        analyzed_at,
        built_at_commit,
        head_commit: head,
        staleness,
        plugin_available: plugin_available(),
        node_available,
        node_major,
        viewer_available: viewer_runnable,
        viewer_path: viewer.map(|p| p.display().to_string()).unwrap_or_default(),
        install_steps_hint: viewer_install_hint(),
    }
}

fn plugin_available() -> bool {
    let Some(home) = home_dir() else {
        return false;
    };
    // Universal symlink (all platforms, install.sh E16) or a native Claude
    // Code plugin marketplace install.
    home.join(".understand-anything-plugin").exists()
        || home.join(".claude/plugins/marketplaces/Understand-Anything").exists()
        || home.join(".claude/plugins/understand-anything").exists()
}

fn viewer_install_hint() -> String {
    "Run the env integration tool-understand (Terrain desktop app): it clones the \
     Understand-Anything repo into ~/.terrain/understand-anything/repo and builds the viewer."
        .to_owned()
}

struct DirMeta {
    last_analyzed_at: Option<String>,
    git_commit_hash: Option<String>,
}

/// Read meta.json tolerantly: malformed UA state must never panic status.
fn read_dir_meta(dir: &Path) -> Option<DirMeta> {
    let bytes = std::fs::read(dir.join("meta.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    Some(DirMeta {
        last_analyzed_at: v
            .get("lastAnalyzedAt")
            .and_then(|x| x.as_str())
            .map(str::to_owned),
        git_commit_hash: v
            .get("gitCommitHash")
            .and_then(|x| x.as_str())
            .map(str::to_owned),
    })
}

/// Count graph nodes/links (top-level arrays only; size-capped).
fn count_graph(dir: &Path) -> (Option<u64>, Option<u64>) {
    let path = dir.join("knowledge-graph.json");
    let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if len == 0 || len > MAX_STATUS_GRAPH_BYTES {
        return (None, None);
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return (None, None),
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (None, None);
    };
    (
        v.get("nodes").and_then(|a| a.as_array()).map(|a| a.len() as u64),
        v.get("links").and_then(|a| a.as_array()).map(|a| a.len() as u64),
    )
}

/// Bind a TCP listener on 127.0.0.1:0 to claim an ephemeral port, then close it
/// (small TOCTOU race accepted — the viewer fails loudly if the port is taken).
pub fn pick_viewer_port() -> Option<u16> {
    TcpListener::bind(("127.0.0.1", 0))
        .ok()?
        .local_addr()
        .ok()
        .map(|a| a.port())
}

/// Parse the viewer's startup banner:
/// `  🔑  Dashboard URL: http://127.0.0.1:5173/?token=<t>`
pub fn parse_viewer_banner(line: &str) -> Option<String> {
    let idx = line.find("Dashboard URL:")?;
    let url = line[idx + "Dashboard URL:".len()..].trim().to_owned();
    url.starts_with("http://127.0.0.1").then_some(url)
}

/// Probe whether a port is already serving (cheap liveness check).
pub fn port_alive(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// Human-facing failure when the viewer cannot start.
pub fn viewer_start_error(reason: &str) -> CoreError {
    CoreError::InvalidDoc(format!("UA viewer failed to start: {reason}"))
}

#[cfg(test)]
mod ua_status_tests {
    use super::*;
    use std::fs;

    fn temp_repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ua-status-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn status_reports_empty_when_no_ua_dir() {
        let repo = temp_repo("empty");
        let st = ua_status(&repo);
        assert_eq!(st.data_dir, "");
        assert!(!st.graph_present);
        assert!(!st.fingerprints_present);
        assert!(st.install_steps_hint.contains("env integration"));
    }

    #[test]
    fn status_reads_ua_dir_and_meta() {
        let repo = temp_repo("populated");
        let ua = repo.join(".ua");
        fs::create_dir_all(&ua).unwrap();
        fs::write(
            ua.join("knowledge-graph.json"),
            r#"{"nodes":[{"id":"a"}],"links":[{"id":"l"}]}"#,
        )
        .unwrap();
        fs::write(
            ua.join("meta.json"),
            r#"{"lastAnalyzedAt":"2026-10-31","gitCommitHash":"abc123","version":"v1"}"#,
        )
        .unwrap();
        fs::write(ua.join("fingerprints.json"), "{}").unwrap();

        let st = ua_status(&repo);
        assert_eq!(st.data_dir, "ua");
        assert!(st.graph_present && st.meta_present && st.fingerprints_present);
        assert_eq!(st.nodes, Some(1));
        assert_eq!(st.links, Some(1));
        assert_eq!(st.built_at_commit.as_deref(), Some("abc123"));
        assert_eq!(st.analyzed_at.as_deref(), Some("2026-10-31"));
    }

    #[test]
    fn legacy_dir_wins_when_both_present() {
        let repo = temp_repo("both");
        let legacy = repo.join(".understand-anything");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("knowledge-graph.json"), r#"{"nodes":[],"links":[]}"#).unwrap();
        let st = ua_status(&repo);
        assert_eq!(st.data_dir, "legacy");
    }

    #[test]
    fn banner_parse_and_port() {
        assert_eq!(
            parse_viewer_banner("  🔑  Dashboard URL: http://127.0.0.1:5173/?token=abc"),
            Some("http://127.0.0.1:5173/?token=abc".to_owned())
        );
        assert_eq!(parse_viewer_banner("Some other line"), None);
        let port = pick_viewer_port().unwrap();
        assert!(port > 0);
    }
}