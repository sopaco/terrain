use mind_mesh_core::{
    list_human_docs, read_doc_at, read_source_slice, resolve_source_citation, HumanDocEntry,
    KnowledgeDoc, KnowledgeSearch, SearchHit, SearchOptions, SourceSlice,
};
use mind_mesh_agent::validate_repo_path;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn search_knowledge(
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    KnowledgeSearch::new(&state.paths)
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
    read_doc_at(&state.paths, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_human_docs_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<Vec<HumanDocEntry>, String> {
    list_human_docs(&state.paths, &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_source_slice_cmd(
    repo_path: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
) -> Result<SourceSlice, String> {
    read_source_slice(&repo_path, &file_path, start_line, end_line).map_err(|e| e.to_string())
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
        &state.paths,
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
    validate_repo_path(&path).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .status()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .status()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .status()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
