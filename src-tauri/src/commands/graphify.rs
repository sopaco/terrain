//! Graphify commands — thin Tauri wrappers over the `terrain-dashboard` crate.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use terrain_dashboard::{
    GraphifyAction, GraphifyCommunity, GraphifyGraphInfo, git_head, install_hint, read_graph_info,
    resolve_graphify_bin, run_graphify_action, staleness_of,
};

use super::payloads::{GraphifyDonePayload, GraphifyProgressPayload};
use super::slugify_repo;
use super::util::validate_repo;

/// graph.html displayed in an iframe (srcdoc); cap the payload.
const MAX_VIEW_HTML_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct GraphifyStatus {
    pub binary_available: bool,
    pub binary_path: String,
    pub out_dir: String,
    pub graph_present: bool,
    pub report_present: bool,
    pub html_present: bool,
    pub wiki_present: bool,
    pub graph_mtime_secs: Option<u64>,
    pub nodes: Option<u64>,
    pub links: Option<u64>,
    pub communities: Option<Vec<GraphifyCommunity>>,
    pub built_at_commit: Option<String>,
    pub head_commit: Option<String>,
    pub staleness: String,
}

#[tauri::command]
pub fn graphify_status_cmd(repo_path: String) -> Result<GraphifyStatus, String> {
    let repo = validate_repo(&repo_path)?;
    let info: GraphifyGraphInfo = read_graph_info(&repo);
    let head = git_head(&repo);
    let staleness = staleness_of(&info.built_at_commit, &head);
    let binary = resolve_graphify_bin();

    Ok(GraphifyStatus {
        binary_available: binary.is_some(),
        binary_path: binary.map(|p| p.display().to_string()).unwrap_or_default(),
        out_dir: repo.join(".terrain/graphify-out").display().to_string(),
        graph_present: info.graph_present,
        report_present: info.report_present,
        html_present: info.html_present,
        wiki_present: info.wiki_present,
        graph_mtime_secs: info.graph_mtime_secs,
        nodes: info.nodes,
        links: info.links,
        communities: info.communities.clone(),
        built_at_commit: info.built_at_commit.clone(),
        head_commit: head,
        staleness,
    })
}

/// `action`: "extract" (first build, AST-only) | "update" (incremental, AST-only).
#[tauri::command]
pub async fn run_graphify_cmd(
    app: AppHandle,
    repo_path: String,
    project_slug: Option<String>,
    action: String,
) -> Result<(), String> {
    let repo = validate_repo(&repo_path)?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let bin = resolve_graphify_bin().ok_or_else(install_hint)?;
    let action = match action.as_str() {
        "extract" => GraphifyAction::ExtractUpdate,
        "update" => GraphifyAction::Update,
        other => return Err(format!("unknown graphify action: {other}")),
    };

    let emitter: Arc<dyn Fn(&str) + Send + Sync> = {
        let app = app.clone();
        let slug = slug.clone();
        Arc::new(move |line: &str| {
            let _ = app.emit(
                "graphify-progress",
                GraphifyProgressPayload {
                    project_slug: slug.clone(),
                    stage: "run".to_owned(),
                    message: line.to_owned(),
                },
            );
        })
    };

    let outcome = run_graphify_action(&repo, &bin, action, emitter).await;
    let _ = app.emit(
        "graphify-done",
        GraphifyDonePayload {
            project_slug: slug,
            ok: outcome.ok,
            message: outcome.message.clone(),
        },
    );
    if !outcome.ok {
        return Err(outcome.message);
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct GraphifyViewPayload {
    /// None when graph.html is missing, or too large to send to the webview.
    pub html: Option<String>,
    pub html_too_large: bool,
}

/// Read `.terrain/graphify-out/graph.html` for display in an iframe (srcdoc).
#[tauri::command]
pub fn get_graphify_view_cmd(repo_path: String) -> Result<GraphifyViewPayload, String> {
    let repo = validate_repo(&repo_path)?;
    let html_path = repo
        .join(".terrain")
        .join("graphify-out")
        .join("graph.html");
    let too_large = std::fs::metadata(&html_path)
        .map(|m| m.len() > MAX_VIEW_HTML_BYTES)
        .unwrap_or(false);
    if too_large {
        return Ok(GraphifyViewPayload {
            html: None,
            html_too_large: true,
        });
    }
    if !html_path.is_file() {
        return Ok(GraphifyViewPayload {
            html: None,
            html_too_large: false,
        });
    }
    let bytes = std::fs::read(&html_path).map_err(|e| format!("Failed to read graph.html: {e}"))?;
    Ok(GraphifyViewPayload {
        html: Some(String::from_utf8_lossy(&bytes).into_owned()),
        html_too_large: false,
    })
}
