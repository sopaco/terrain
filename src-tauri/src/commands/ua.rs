//! UA commands — status + read-only viewer process (docs/CLONE_SPEC_UA-integration.md).

use std::collections::HashMap;
use std::io::BufRead;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use terrain_dashboard::{UaStatus, parse_viewer_banner, pick_viewer_port, ua_status};

use super::util::validate_repo;
use crate::AppState;

#[tauri::command]
pub fn ua_status_cmd(repo_path: String) -> Result<UaStatus, String> {
    let repo = validate_repo(&repo_path)?;
    Ok(ua_status(&repo))
}

#[derive(Debug, Clone, Serialize)]
pub struct UaViewerInfo {
    pub running: bool,
    /// `http://127.0.0.1:<port>/?token=<t>` for the iframe src.
    pub url: String,
    pub port: u16,
    pub message: String,
}

/// Live viewer record: enough to probe liveness (port) and replace the process.
#[derive(Debug, Clone, Serialize)]
pub struct UaViewerEntry {
    pub pid: u32,
    pub url: String,
    pub port: u16,
}

pub type UaViewerMap = Arc<Mutex<HashMap<String, UaViewerEntry>>>;

/// Start (or return the already-running) UA viewer for a repo: spawn
/// `node <viewer.mjs> <repo> --no-open --port <n>`, parse the startup banner.
/// A previously registered viewer whose port went dead is replaced.
#[tauri::command]
pub async fn ua_viewer_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<UaViewerInfo, String> {
    let repo = validate_repo(&repo_path)?;
    let key = repo.display().to_string();
    let map = state.ua_viewers.clone();

    let status = ua_status(&repo);
    if !status.graph_present {
        return Err(
            "No UA data dir (.ua / .understand-anything / .terrain/understand) found in this \
             project — run the understand skill first."
                .to_owned(),
        );
    }
    if !status.viewer_available {
        return Err(
            "UA viewer unavailable: need Node >= 18 and a built Understand-Anything checkout \
             (run env integration tool-understand, then reopen Terrain)."
                .to_owned(),
        );
    }
    if let Some(entry) = map.lock().expect("ua_viewers poisoned").get(&key).cloned() {
        if terrain_dashboard::port_alive(entry.port) {
            return Ok(UaViewerInfo {
                running: true,
                url: entry.url,
                port: entry.port,
                message: "already serving".to_owned(),
            });
        }
        // Stale registration — the process is gone.
        map.lock().expect("ua_viewers poisoned").remove(&key);
    }

    let viewer = terrain_dashboard::resolve_ua_viewer()
        .ok_or_else(|| "UA viewer script not found (tool-understand incomplete)".to_owned())?;
    let node_bin = terrain_core::resolve_executable("node")
        .ok_or_else(|| "node not found".to_owned())?;

    let port = pick_viewer_port().ok_or_else(|| "no free local port".to_owned())?;
    let (child, url, port) = {
        let node_bin = node_bin.display().to_string();
        let viewer = viewer.display().to_string();
        let repo_dir = repo.display().to_string();
        tauri::async_runtime::spawn_blocking(move || -> Result<(Child, String, u16), String> {
            let mut child = std::process::Command::new(&node_bin)
                .arg(&viewer)
                .arg(&repo_dir)
                .arg("--no-open")
                .arg(format!("--port={port}"))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("failed to spawn viewer: {e}"))?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| "viewer stdout closed".to_owned())?;
            let deadline = Instant::now() + Duration::from_secs(15);
            let mut reader = std::io::BufReader::new(stdout);
            while Instant::now() < deadline {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = child.kill();
                        return Err("viewer exited before printing a Dashboard URL".to_owned());
                    }
                    Ok(_) => {
                        if let Some(url) = parse_viewer_banner(&line) {
                            return Ok((child, url, port));
                        }
                    }
                    Err(_) => {
                        let _ = child.kill();
                        return Err("viewer stdout read failed".to_owned());
                    }
                }
            }
            let _ = child.kill();
            Err("viewer produced no Dashboard URL within 15s; is the graph data dir present?".to_owned())
        })
        .await
        .map_err(|e| format!("viewer spawn task failed: {e}"))?
    }?;

    let pid = child.id();
    // std Child::drop does not kill on Unix — the viewer keeps serving after
    // this call returns; registration detects death via the port probe.
    drop(child);

    let info = UaViewerInfo {
        running: true,
        url: url.clone(),
        port,
        message: "started".to_owned(),
    };
    map.lock()
        .expect("ua_viewers poisoned")
        .insert(key.clone(), UaViewerEntry { pid, url, port });
    let _ = app.emit(
        "ua-viewer-ready",
        super::payloads::UaViewerReadyPayload {
            project_slug: super::slugify_repo(&repo_path),
            url: info.url.clone(),
            port,
        },
    );
    Ok(info)
}