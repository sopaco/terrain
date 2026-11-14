use terrain_core::{
    list_human_docs, process_command, read_doc_at, read_source_slice, resolve_source_citation,
    HumanDocEntry, KnowledgeDoc, KnowledgeSearch, SearchHit, SearchOptions, SourceSlice,
};
use tauri::State;

use crate::AppState;

use super::util::{map_core_err, validate_repo};

#[tauri::command]
pub fn search_knowledge(
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    KnowledgeSearch::new(state.paths())
        .search(
            &query,
            SearchOptions {
                project,
                doc_type: None,
                limit: limit.unwrap_or(20),
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_document(state: State<'_, AppState>, path: String) -> Result<KnowledgeDoc, String> {
    read_doc_at(state.paths(), &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_human_docs_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<Vec<HumanDocEntry>, String> {
    list_human_docs(state.paths(), &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_source_slice_cmd(
    repo_path: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
) -> Result<SourceSlice, String> {
    map_core_err(read_source_slice(&repo_path, &file_path, start_line, end_line))
}

#[tauri::command]
pub fn resolve_source_citation_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    file_path: String,
    start_line: Option<u32>,
    end_line: Option<u32>,
    repo_path: Option<String>,
) -> Result<SourceSlice, String> {
    resolve_source_citation(
        state.paths(),
        &project_slug,
        repo_path.as_deref(),
        &file_path,
        start_line.unwrap_or(0),
        end_line.unwrap_or(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_repo_folder_cmd(path: String) -> Result<(), String> {
    validate_repo(&path)?;
    // Spawned through `terrain_core::process_command` (not `std::process::Command`): Windows
    // builds are GUI-subsystem, so an unflagged console child flashes a terminal window.
    #[cfg(target_os = "macos")]
    let launcher = "open";
    #[cfg(target_os = "windows")]
    let launcher = "explorer";
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let launcher = "xdg-open";

    process_command(launcher)
        .arg(&path)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}
