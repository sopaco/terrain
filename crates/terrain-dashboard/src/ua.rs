//! Understand-Anything (UA) driver: status + centralized pipeline + viewer.

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tokio::process::Command;

use terrain_core::CoreError;

pub const UA_DIR_REL: &str = ".ua";
pub const UA_LEGACY_DIR_REL: &str = ".understand-anything";
pub const UA_TERRAIN_DIR_REL: &str = ".terrain/understand";
pub const UA_REPO_REL: &str = "understand-anything/repo";

const MAX_STATUS_GRAPH_BYTES: u64 = 48 * 1024 * 1024;

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn resolve_ua_repo() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("UA_REPO_DIR") {
        let p = PathBuf::from(dir);
        return p.is_dir().then_some(p);
    }
    home_dir()
        .map(|h| h.join(".terrain").join(UA_REPO_REL))
        .filter(|p| p.is_dir())
}

pub fn resolve_ua_viewer() -> Option<PathBuf> {
    if let Some(bin) = std::env::var_os("UA_VIEWER_BIN") {
        let p = PathBuf::from(bin);
        return p.is_file().then_some(p);
    }
    let script = resolve_ua_repo().map(|r| {
        r.join("understand-anything-plugin/packages/viewer/bin/viewer.mjs")
    })?;
    let dist = script.parent()?.join("dist");
    (script.is_file() && dist.is_dir()).then_some(script)
}

pub fn node_version() -> Option<(PathBuf, u32)> {
    let node = terrain_core::resolve_executable("node")?;
    let out = terrain_core::process_command(&node.display().to_string())
        .args(["--version"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())?;
    Some((node, v))
}

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

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Serialize)]
pub struct UaStatus {
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
    pub auto_update: bool,
    pub install_steps_hint: String,
}

pub fn ua_status(repo: &Path) -> UaStatus {
    let head = crate::graphify::git_head(repo);
    let found = ua_data_dir(repo);
    let mut st_data_dir = String::new();
    let mut st_path = String::new();
    let mut graph_present = false;
    let mut meta_present = false;
    let mut fingerprints_present = false;
    let mut nodes = None;
    let mut links = None;
    let mut analyzed_at = None;
    let mut built_at_commit = None;
    let mut auto_update = false;
    if let Some((dir, kind)) = found {
        let meta = read_dir_meta(&dir);
        let (n, l) = count_graph(&dir);
        auto_update = read_config(&dir).map(|c| c.auto_update).unwrap_or(false);
        st_data_dir = kind.to_owned();
        st_path = dir.display().to_string();
        graph_present = true;
        meta_present = meta.is_some();
        fingerprints_present = dir.join("fingerprints.json").is_file();
        nodes = n;
        links = l;
        analyzed_at = meta .as_ref().and_then(|m| m.last_analyzed_at.clone());
        built_at_commit = meta .as_ref().and_then(|m| m.git_commit_hash.clone());
    }
    let staleness = crate::dashboard::staleness_of(&built_at_commit, &head);
    let (node_available, node_major) = match node_version() {
        Some((_, m)) => (true, Some(m)),
        None => (false, None),
    };
    let viewer = resolve_ua_viewer();
    let viewer_runnable = viewer.is_some() && node_available && node_major.unwrap_or(0) >= 18;
    UaStatus {
        data_dir: st_data_dir,
        data_dir_path: st_path,
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
        auto_update,
        install_steps_hint: "Run the env integration tool-understand (Terrain desktop app).".to_owned(),
    }
}

fn plugin_available() -> bool {
    let Some(home) = home_dir() else {
        return false;
    };
    home.join(".understand-anything-plugin").exists()
        || home.join(".claude/plugins/marketplaces/Understand-Anything").exists()
        || home.join(".claude/plugins/understand-anything").exists()
}

struct DirMeta {
    last_analyzed_at: Option<String>,
    git_commit_hash: Option<String>,
}
struct DirConfig {
    auto_update: bool,
}
fn read_dir_meta(dir: &Path) -> Option<DirMeta> {
    let bytes = std::fs::read(dir.join("meta.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    Some(DirMeta {
        last_analyzed_at: v.get("lastAnalyzedAt").and_then(|x| x.as_str()).map(str::to_owned),
        git_commit_hash: v.get("gitCommitHash").and_then(|x| x.as_str()).map(str::to_owned),
    })
}
fn read_config(dir: &Path) -> Option<DirConfig> {
    let bytes = std::fs::read(dir.join("config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    Some(DirConfig {
        auto_update: v.get("autoUpdate").and_then(|x| x.as_bool()).unwrap_or(false),
    })
}
fn count_graph(dir: &Path) -> (Option<u64>, Option<u64>) {
    let path = dir.join("knowledge-graph.json");
    let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if len == 0 || len > MAX_STATUS_GRAPH_BYTES {
        return (None, None);
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (None, None);
    };
    (
        v.get("nodes").and_then(|a| a.as_array()).map(|a| a.len() as u64),
        v.get("links").and_then(|a| a.as_array()).map(|a| a.len() as u64),
    )
}

pub fn pick_viewer_port() -> Option<u16> {
    TcpListener::bind(("127.0.0.1", 0)).ok()?.local_addr().ok().map(|a| a.port())
}
pub fn parse_viewer_banner(line: &str) -> Option<String> {
    let idx = line.find("Dashboard URL:")?;
    let url = line[idx + "Dashboard URL:".len()..].trim().to_owned();
    url.starts_with("http://127.0.0.1").then_some(url)
}
pub fn port_alive(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}
pub fn viewer_start_error(reason: &str) -> CoreError {
    CoreError::InvalidDoc(format!("UA viewer failed to start: {reason}"))
}

// ── Terrain-centralized UA pipeline ───────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct UaRunOutcome {
    pub ok: bool,
    pub message: String,
}
pub enum UaAction {
    Analyze,
    Update,
}

type ProgressCallback = Arc<dyn Fn(&str) + Send + Sync>;

async fn run_child(
    program: &str,
    args: &[String],
    cwd: &Path,
    progress: Arc<dyn Fn(&str) + Send + Sync>,
) -> Result<std::process::Output, String> {
    (progress)(&format!("  ▶ {} {}", program, args.join(" ")));
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| format!("failed to launch {}: {}", program, e))?;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        (progress)(&format!("  │ {}", line));
    }
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        (progress)(&format!("  │ ! {}", line));
    }
    if !out.status.success() {
        // Surface both streams so a runtime failure (Python traceback etc.)
        // lands in the UI's  payload, not only on the progress tick.
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        let short_stderr = if stderr.len() > 1_200 {
            format!("…{}", &stderr[stderr.len() - 1_200..])
        } else {
            stderr
        };
        return Err(format!(
            "{} exited non-zero ({})
{}",
            program,
            out.status.code().unwrap_or(-1),
            short_stderr,
        ));
    }
    Ok(out)
}

pub async fn run_ua_action(
    repo: &Path,
    _action: UaAction,
    progress: Arc<dyn Fn(&str) + Send + Sync>,
) -> UaRunOutcome {
    (progress)("terrain: verify UA toolchain");
    let ua_root = match resolve_ua_repo() {
        Some(r) => r,
        None => {
            return UaRunOutcome {
                ok: false,
                message: "Understand-Anything repo not installed — run the env integration tool-understand first.".to_owned(),
            };
        }
    };
    let script_dir = ua_root.join("understand-anything-plugin/skills/understand");
    for f in ["scan-project.mjs", "extract-import-map.mjs", "compute-batches.mjs",
              "merge-batch-graphs.py", "build-fingerprints.mjs"] {
        if !script_dir.join(f).is_file() {
            return UaRunOutcome { ok: false, message: format!("UA checkout incomplete: missing {}",
                script_dir.join(f).display()) };
        }
    }
    if !ua_root
        .join("understand-anything-plugin/packages/core/dist/index.js")
        .is_file()
    {
        return UaRunOutcome { ok: false, message:
            "UA core is not built — run the tool-understand env integration (pnpm build) first.".to_owned()
        };
    }
    let node_bin = match terrain_core::resolve_executable("node") {
        Some(p) => p.display().to_string(),
        None => return UaRunOutcome { ok: false, message: "node (>=18) not found on PATH".to_owned() },
    };
    let python_bin = terrain_core::resolve_executable("python3")
        .or_else(|| terrain_core::resolve_executable("python"))
        .map(|p| p.display().to_string());
    let python_bin = match python_bin {
        Some(p) => p,
        None => return UaRunOutcome { ok: false, message: "python3 not found (merge-batch-graphs.py needs it)".to_owned() },
    };

    let ua_dir = repo.join(".ua");
    let intermediate = ua_dir.join("intermediate");
    if let Err(e) = std::fs::create_dir_all(&intermediate) {
        return UaRunOutcome { ok: false, message: format!("mkdir .ua/intermediate: {e}") };
    }

    // phase 1 scan
    (progress)("phase 1: scan-project");
    let scan_out = intermediate.join("scan-result.json");
    if let Err(e) = run_child(
        &node_bin,
        &[script_dir.join("scan-project.mjs").display().to_string(),
          repo.display().to_string(),
          scan_out.display().to_string(),
          "--exclude-analysis-data".to_owned()],
        repo, progress.clone(),
    ).await {
        return UaRunOutcome { ok: false, message: format!("scan failed: {e}") };
    }

    // Phase 1b: import map — UA's project-scanner runs this and merges the
    // importMap into scan-result.json; without it there are no import edges
    // and merge-batch-graphs reports "no importMap field".
    (progress)("phase 1b: extract-import-map");
    let im_in = intermediate.join("import-map-input.json");
    let im_out = intermediate.join("import-map-output.json");
    if let Err(e) = build_import_map_input(&scan_out, &im_in, repo) {
        return UaRunOutcome { ok: false, message: format!("import map input: {e}") };
    }
    if let Err(e) = run_child(
        &node_bin,
        &[script_dir.join("extract-import-map.mjs").display().to_string(),
          im_in.display().to_string(),
          im_out.display().to_string()],
        repo, progress.clone(),
    ).await {
        return UaRunOutcome { ok: false, message: format!("import map failed: {e}") };
    }
    if let Err(e) = inject_import_map(&scan_out, &im_out) {
        return UaRunOutcome { ok: false, message: format!("import map inject failed: {e}") };
    }

    // phase 1.5 batch
    (progress)("phase 1.5: compute-batches");
    let batches_path = intermediate.join("batches.json");
    if let Err(e) = run_child(
        &node_bin,
        &[script_dir.join("compute-batches.mjs").display().to_string(),
          repo.display().to_string(),
          format!("--scan-result={}", scan_out.display()),
          format!("--output={}", batches_path.display())],
        repo, progress.clone(),
    ).await {
        return UaRunOutcome { ok: false, message: format!("batching failed: {e}") };
    }

    // phase 2 extract per batch
    let batches_raw = match std::fs::read_to_string(&batches_path) {
        Ok(s) => s,
        Err(e) => return UaRunOutcome { ok: false, message: format!("read batches.json: {e}") },
    };
    let batches_json: serde_json::Value = match serde_json::from_str(&batches_raw) {
        Ok(v) => v,
        Err(e) => return UaRunOutcome { ok: false, message: format!("batches.json malformed: {e}") },
    };
    // Phase 2: synthesize structural batch graph fragments in-process (no
    // LLM): one `file:<path>` node per file + `imports` edges whose endpoints
    // both resolve in the full file set. merge-batch-graphs.py consumes these,
    // normalizes IDs/complexity, and re-recovers imports from the scan's
    // importMap deterministically — so the structural tier is complete e2e.
    (progress)("phase 2: synthesizing structural batch graphs (no LLM)");
    let written = match synthesize_structural_batches(&batches_json, &intermediate) {
        Ok(n) => n,
        Err(e) => return UaRunOutcome { ok: false, message: format!("batch synthesis failed: {e}") },
    };
    (progress)(&format!("phase 2: wrote {} structural batches", written));

    // phase 3 merge
    (progress)("phase 3: merge batches");
    if let Err(e) = run_child(
        &python_bin,
        &[script_dir.join("merge-batch-graphs.py").display().to_string(),
          repo.display().to_string()],
        repo, progress.clone(),
    ).await {
        return UaRunOutcome { ok: false, message: format!("merge failed: {e}") };
    }
    let assembled = intermediate.join("assembled-graph.json");
    let knowledge_graph = ua_dir.join("knowledge-graph.json");
    if !assembled.is_file() {
        return UaRunOutcome { ok: false, message: "merge produced no assembled-graph.json".to_owned() };
    }
    if let Err(e) = promote_knowledge_graph(&assembled, &scan_out, &knowledge_graph, repo) {
        return UaRunOutcome { ok: false, message: format!("promote: {e}") };
    }

    // phase 7 fingerprints (best effort)
    let all_files: Vec<String> = batches_json["batches"]
        .as_array().cloned().unwrap_or_default()
        .into_iter()
        .flat_map(|b| b["files"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f["path"].as_str().map(str::to_owned))
        .collect();
    let fp_input = serde_json::json!({
        "projectRoot": repo.display().to_string(),
        "filePaths": all_files,
        "gitCommitHash": crate::graphify::git_head(repo).unwrap_or_default(),
    });
    let fp_path = intermediate.join("fp-input.json");
    if let Err(e) = std::fs::write(&fp_path, serde_json::to_string_pretty(&fp_input).unwrap()) {
        (progress)(&format!("warn: fp input write: {}", e));
    } else {
        (progress)("phase 7: build fingerprints");
        let _ = run_child(
            &node_bin,
            &[script_dir.join("build-fingerprints.mjs").display().to_string(),
              fp_path.display().to_string()],
            repo, progress.clone(),
        ).await;
    }

    // meta + config
    let stamp = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let meta = serde_json::json!({
        "lastAnalyzedAt": stamp,
        "gitCommitHash": crate::graphify::git_head(repo).unwrap_or_default(),
        "version": "UA-via-terrain",
        "analyzedFiles": all_files.len(),
    });
    let _ = std::fs::write(ua_dir.join("meta.json"), serde_json::to_string_pretty(&meta).unwrap());
    if !ua_dir.join("config.json").is_file() {
        let _ = std::fs::write(ua_dir.join("config.json"), "{\"autoUpdate\": true, \"outputLanguage\": null}");
    }

    let n = count_graph(&ua_dir).0.unwrap_or(0);
    (progress)(&format!( "done: knowledge-graph ready ({} nodes)", n));
    UaRunOutcome {
        ok: true,
        message: format!("UA (centralized) pipeline complete — {} nodes in .ua/knowledge-graph.json.", n),
    }
}

/// Wrap the merge's bare `{nodes,edges}` assembled-graph into UA's full
/// `KnowledgeGraphSchema` envelope (validateGraph is fatal on missing
/// `project` meta), deriving the project meta from the scan + repo.
fn promote_knowledge_graph(
    assembled: &Path,
    scan_out: &Path,
    knowledge_graph: &Path,
    repo: &Path,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(assembled)
        .map_err(|e| format!("read {}: {}", assembled.display(), e))?;
    let graph: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("assembled-graph malformed: {e}"))?;
    let raw_scan = std::fs::read_to_string(scan_out)
        .map_err(|e| format!("read {}: {}", scan_out.display(), e))?;
    let scan: serde_json::Value = serde_json::from_str(&raw_scan)
        .map_err(|e| format!("scan-result malformed: {e}"))?;
    let name = repo
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_owned();
    let languages: Vec<String> = scan["stats"]["byLanguage"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    let scan_frameworks: Vec<String> = scan["frameworks"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let stamp = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());
    let envelope = serde_json::json!({
        "version": "1.0.0",
        "project": {
            "name": name,
            "languages": languages,
            "frameworks": scan_frameworks,
            "description": format!(
                "Structural (AST-tier) knowledge graph of {name}, built centrally by Terrain."
            ),
            "analyzedAt": stamp,
            "gitCommitHash": crate::graphify::git_head(repo).unwrap_or_default(),
        },
        "nodes": graph["nodes"],
        "edges": graph["edges"],
        "layers": [],
        "tour": [],
    });
    std::fs::write(
        knowledge_graph,
        serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write {}: {}", knowledge_graph.display(), e))
}

/// Phase-1b input for extract-import-map.mjs: `{projectRoot, files}` taken
/// straight from the fresh scan-result.json.
fn build_import_map_input(
    scan_out: &Path,
    im_in: &Path,
    repo: &Path,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(scan_out).map_err(|e| format!("read {}: {}", scan_out.display(), e))?;
    let mut scan: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("scan-result malformed: {e}"))?;
    let input = serde_json::json!({
        "projectRoot": repo.display().to_string(),
        "files": scan["files"].clone(),
    });
    std::fs::write(im_in, serde_json::to_string_pretty(&input).map_err(|e| e.to_string())?)
        .map_err(|e| format!("write {}: {}", im_in.display(), e))
}

/// Merge extract-import-map's output into scan-result.json (`importMap` key),
/// rewriting the scan file in place.
fn inject_import_map(scan_out: &Path, im_out: &Path) -> Result<(), String> {
    let raw = std::fs::read_to_string(im_out).map_err(|e| format!("read {}: {}", im_out.display(), e))?;
    let im: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("import-map output malformed: {e}"))?;
    let import_map = im.get("importMap").cloned().unwrap_or(serde_json::json!({}));
    let raw_scan = std::fs::read_to_string(scan_out).map_err(|e| format!("read {}: {}", scan_out.display(), e))?;
    let mut scan: serde_json::Value =
        serde_json::from_str(&raw_scan).map_err(|e| format!("scan-result malformed: {e}"))?;
    scan["importMap"] = import_map;
    std::fs::write(scan_out, serde_json::to_string_pretty(&scan).map_err(|e| e.to_string())?)
        .map_err(|e| format!("write {}: {}", scan_out.display(), e))
}

/// Build one merge-compatible batch file: `{nodes:[{id:"file:<path>",
/// type:"file",name,filePath,complexity}], edges:[{source,target,type:"imports",
/// direction:"forward",weight:1}]}`. Edges keep only in-batch + cross-batch
/// pairs whose endpoints are in the global file set (merge drops the rest and
/// re-recovers imports from scan-result's importMap on its own).
fn synthesize_structural_batches(
    batches_json: &serde_json::Value,
    intermediate: &Path,
) -> Result<usize, String> {
    let batches = batches_json["batches"]
        .as_array()
        .ok_or("batches.json has no 'batches' array")?;
    // Global file set (used to keep only resolvable import edges).
    let mut all_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for b in batches {
        for f in b["files"].as_array().cloned().unwrap_or_default() {
            if let Some(p) = f["path"].as_str() {
                all_paths.insert(p.to_owned());
            }
        }
    }
    let mut written = 0usize;
    for b in batches {
        let idx = b["batchIndex"].as_i64().unwrap_or(1);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for f in b["files"].as_array().cloned().unwrap_or_default() {
            let Some(path) = f["path"].as_str() else { continue };
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            nodes.push(serde_json::json!({
                "id": format!("file:{}", path),
                "type": "file",
                "name": name.clone(),
                "summary": name,
                "tags": [],
                "filePath": path,
                "complexity": "moderate",
            }));
            for tgt in b["batchImportData"][path].as_array().cloned().unwrap_or_default() {
                let Some(t) = tgt.as_str() else { continue };
                if all_paths.contains(t) {
                    edges.push(serde_json::json!({
                        "source": format!("file:{}", path),
                        "target": format!("file:{}", t),
                        "type": "imports",
                        "direction": "forward",
                        "weight": 1,
                    }));
                }
            }
        }
        let batch_out = serde_json::json!({ "nodes": nodes, "edges": edges });
        let out = intermediate.join(format!("batch-{}.json", idx));
        std::fs::write(&out, serde_json::to_string_pretty(&batch_out).map_err(|e| e.to_string())?)
            .map_err(|e| format!("write {}: {}", out.display(), e))?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod ua_pipeline_tests {
    use super::*;

    #[test]
    fn synthesis_writes_merge_compatible_batches() {
        let dir = std::env::temp_dir().join(format!("ua-synth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let batches_json = serde_json::json!({
            "batches": [
                { "batchIndex": 1,
                  "files": [ {"path": "src/a.py", "language": "python"},
                             {"path": "src/b.py", "language": "python"} ],
                  "batchImportData": { "src/a.py": ["src/b.py", "missing/not Listed.py"] } },
                { "batchIndex": 2,
                  "files": [ {"path": "docs/x.md", "language": "markdown"} ],
                  "batchImportData": {} }
            ]
        });
        let n = synthesize_structural_batches(&batches_json, &dir).unwrap();
        assert_eq!(n, 2);
        let b1: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("batch-1.json")).unwrap()).unwrap();
        let ids: Vec<&str> = b1["nodes"].as_array().unwrap().iter()
            .filter_map(|nd| nd["id"].as_str()).collect();
        assert_eq!(ids, ["file:src/a.py", "file:src/b.py"]);
        let edges = b1["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "dangling import target must be dropped: {:?}", edges);
        assert_eq!(edges[0]["target"], "file:src/b.py");
        let b2 = std::fs::read_to_string(dir.join("batch-2.json")).unwrap();
        assert!(b2.contains("file:docs/x.md"));
    }
}
