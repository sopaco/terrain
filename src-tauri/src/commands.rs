use mind_mesh_agent::{
    acp_available, acp_spawn_command, agent_execution_ready, env_plan_for_repo, env_status_for_repo,
    execution_pure_acp, execution_uses_native_llm, knowledge_paths_from_env, llm_status,
    load_model_settings,
    prepare_litho_generation, resolve_acp_settings, run_agent_context_generation, run_env_integration,
    run_litho_generation, run_project_initialization, run_sdd_phase, validate_repo_path,
    AcpSettings, ChatEngine, AgentContextGenerationResult, ChatPhase, ChatReply, ChatTokenUsage,
    ChatToolCallRecord, LithoGenerationJob, LithoGenerationResult, LithoProgress, ModelSettings,
    ProjectInitProgress, ProjectInitResult, SddProgress, model_settings_from_config,
    save_model_settings,
};
use std::sync::Arc;
use mind_mesh_core::{
    compute_freshness, create_sdd_session, extract_source_citations, get_project_overview,
    get_sdd_status, list_stale_registry_projects, merge_citations, read_freshness_ledger,
    resolve_sdd_session_id, save_sdd_output, set_active_sdd_session, write_project_remark, AgentPackReport,
    AssetGenerationPlan, EnvApplyProgress, EnvApplyResult, EnvPlan, EnvStatus, FreshnessSummary,
    HumanDocEntry, KnowledgeDoc, KnowledgePaths, KnowledgeSearch, LithoPlan, ProjectOverview,
    ProjectScanner, ProjectSummary, ScanReport, SearchHit, SearchOptions, SddPhase, SddPhaseResult,
    SddSessionInfo, SddStatus, SourceCitation, SourceSlice, StaleProjectSummary,
    build_generation_plan, list_human_docs, pack_agent_assets, plan_litho_generation, read_doc_at,
    read_source_slice, resolve_source_citation,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

#[derive(Clone, Serialize)]
struct ChatChunkPayload {
    session_id: String,
    text: String,
}

#[derive(Clone, Serialize)]
struct ChatToolCallsPayload {
    session_id: String,
    tool_calls: Vec<ChatToolCallRecord>,
}

#[derive(Clone, Serialize)]
struct ChatPhasePayload {
    session_id: String,
    phase: ChatPhase,
}

#[derive(Clone, Serialize)]
struct ChatUsagePayload {
    session_id: String,
    usage: ChatTokenUsage,
}

#[derive(Clone, Serialize)]
struct ChatDonePayload {
    session_id: String,
    answer: String,
    citations: Vec<SourceCitation>,
    tool_calls: Vec<ChatToolCallRecord>,
    usage: ChatTokenUsage,
    completed_at: u64,
}

#[derive(Clone, Serialize)]
struct LithoProgressPayload {
    project_slug: String,
    stage: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct LithoDonePayload {
    project_slug: String,
    result: LithoGenerationResult,
}

#[derive(Clone, Serialize)]
struct SddProgressPayload {
    project_slug: String,
    phase: SddPhase,
    stage: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct SddDonePayload {
    project_slug: String,
    result: SddPhaseResult,
}

pub fn init_paths() -> KnowledgePaths {
    let paths = knowledge_paths_from_env();
    let _ = paths.ensure_layout();
    paths
}

#[tauri::command]
pub fn get_knowledge_root(
    state: State<'_, AppState>,
    project_slug: Option<String>,
) -> Result<String, String> {
    if let Some(slug) = project_slug.filter(|s| !s.trim().is_empty()) {
        return state
            .paths
            .try_project_dir(&slug)
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string());
    }
    Ok(String::new())
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, String> {
    KnowledgeSearch::new(&state.paths)
        .list_projects()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_project(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<ScanReport, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let scanner = ProjectScanner::new(state.paths.clone());
    scanner
        .scan_repo(&repo_path, project_slug.as_deref())
        .await
        .map_err(|e| e.to_string())
}

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

fn resolved_acp_settings() -> AcpSettings {
    load_model_settings()
        .map(|s| s.acp)
        .unwrap_or_else(resolve_acp_settings)
}

#[tauri::command]
pub fn check_acp() -> bool {
    acp_available(&resolved_acp_settings())
}

#[tauri::command]
pub fn acp_spawn_command_cmd() -> String {
    acp_spawn_command(&resolved_acp_settings())
}

/// Backward-compatible alias.
#[tauri::command]
pub fn check_opencode() -> bool {
    check_acp()
}

#[tauri::command]
pub fn check_llm(state: State<'_, AppState>) -> mind_mesh_agent::LlmStatus {
    llm_status(&state.get_model_config())
}

#[tauri::command]
pub fn get_model_settings(state: State<'_, AppState>) -> ModelSettings {
    load_model_settings().unwrap_or_else(|| model_settings_from_config(&state.get_model_config()))
}

#[tauri::command]
pub async fn save_model_settings_cmd(
    state: State<'_, AppState>,
    settings: ModelSettings,
) -> Result<mind_mesh_agent::LlmStatus, String> {
    save_model_settings(&settings).map_err(|e| e.to_string())?;
    let config = mind_mesh_agent::resolve_model_config();
    state.set_model_config(config);
    *state.chat.lock().await = None;
    Ok(llm_status(&state.get_model_config()))
}

#[tauri::command]
pub fn copy_image_to_clipboard(png_base64: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_image(arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn pack_agent_assets_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<AgentPackReport, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let report = pack_agent_assets(&state.paths, &slug, &repo_path)
        .await
        .map_err(|e| e.to_string())?;
    let _ = compute_freshness(&state.paths, &slug, &repo_path);
    Ok(report)
}

#[derive(serde::Serialize)]
pub struct QuickRefreshResult {
    pub project_slug: String,
    pub scan_files_written: usize,
    pub pack_tokens: Option<usize>,
    pub agent_context_regenerated: bool,
    pub notes: Vec<String>,
    pub freshness: FreshnessSummary,
}

#[tauri::command]
pub fn compute_freshness_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    repo_path: Option<String>,
) -> Result<FreshnessSummary, String> {
    let repo = repo_path
        .filter(|r| !r.is_empty())
        .or_else(|| mind_mesh_core::resolve_project_repo_path(&state.paths, &project_slug, None).ok())
        .ok_or_else(|| "repository path required".to_string())?;
    compute_freshness(&state.paths, &project_slug, &repo).map_err(|e| e.to_string())
}

/// Last persisted freshness ledger (no git recompute) — for instant overview paint.
#[tauri::command]
pub fn read_project_freshness_cached_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Option<FreshnessSummary> {
    read_freshness_ledger(&state.paths, &project_slug).map(|ledger| ledger.summary)
}

#[tauri::command]
pub async fn run_quick_refresh_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<QuickRefreshResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let mut notes = Vec::new();

    let scanner = ProjectScanner::new(paths.clone());
    let scan = scanner
        .scan_repo(&repo_path, Some(&slug))
        .await
        .map_err(|e| e.to_string())?;

    let pack = pack_agent_assets(&paths, &slug, &repo_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut agent_context_regenerated = false;
    let acp = resolved_acp_settings();
    let model_config = state.get_model_config();
    if agent_execution_ready(&acp, &model_config).is_ok() {
        let engine = if execution_uses_native_llm(&acp) {
            Some(Arc::new(
                ChatEngine::new_native(paths.clone(), model_config)
                    .map_err(|e| e.to_string())?,
            ))
        } else {
            None
        };
        match run_agent_context_generation(&paths, engine, &acp, &slug, &repo_path).await {
            Ok(_) => agent_context_regenerated = true,
            Err(e) => notes.push(format!("Agent context: {e}")),
        }
    } else if execution_pure_acp(&acp) {
        notes.push("Agent 友好的知识资产：请先在设置中配置 ACP 代理".into());
    } else {
        notes.push("Agent 友好的知识资产：请配置 ACP 代理与 LLM".into());
    }

    let freshness =
        compute_freshness(&paths, &slug, &repo_path).map_err(|e| e.to_string())?;

    Ok(QuickRefreshResult {
        project_slug: slug,
        scan_files_written: scan.files_written,
        pack_tokens: Some(pack.total_tokens),
        agent_context_regenerated,
        notes,
        freshness,
    })
}

#[tauri::command]
pub fn plan_litho_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<LithoPlan, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(plan_litho_generation(
        &state.paths,
        &slug,
        std::path::Path::new(&repo_path),
    ))
}

#[tauri::command]
pub fn plan_assets_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<AssetGenerationPlan, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(build_generation_plan(&state.paths, &slug, &repo_path))
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

#[tauri::command]
pub fn generate_human_docs_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<LithoGenerationJob, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    Ok(prepare_litho_generation(
        &state.paths,
        &slug,
        &repo_path,
        &resolved_acp_settings(),
    ))
}

#[tauri::command]
pub async fn run_litho_generation_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<(), String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();

    let emit_progress = |p: LithoProgress| {
        let _ = app.emit(
            "litho-progress",
            LithoProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let acp = resolved_acp_settings();
    let result = run_litho_generation(&paths, &slug, &repo_path, &acp, emit_progress)
        .await
        .map_err(|e| e.to_string())?;

    app.emit(
        "litho-done",
        LithoDonePayload {
            project_slug: slug,
            result,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Clone, Serialize)]
struct ProjectInitProgressPayload {
    project_slug: String,
    stage: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct ProjectInitDonePayload {
    project_slug: String,
    result: ProjectInitResult,
}

#[tauri::command]
pub fn list_stale_projects_cmd() -> Result<Vec<StaleProjectSummary>, String> {
    list_stale_registry_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn initialize_project_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<ProjectInitResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let model_config = state.get_model_config();
    let acp = resolved_acp_settings();

    let emit_init = |p: ProjectInitProgress| {
        let _ = app.emit(
            "project-init-progress",
            ProjectInitProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let emit_litho = |p: LithoProgress| {
        let message = p.message.clone();
        let _ = app.emit(
            "litho-progress",
            LithoProgressPayload {
                project_slug: slug.clone(),
                stage: p.stage,
                message,
            },
        );
        let _ = app.emit(
            "project-init-progress",
            ProjectInitProgressPayload {
                project_slug: slug.clone(),
                stage: "human_docs".into(),
                message: p.message,
            },
        );
    };

    let result = run_project_initialization(
        &paths,
        &model_config,
        &acp,
        &repo_path,
        Some(&slug),
        emit_init,
        emit_litho,
    )
    .await
    .map_err(|e| e.to_string())?;

    if result.litho_ran {
        let _ = app.emit(
            "litho-done",
            LithoDonePayload {
                project_slug: result.project_slug.clone(),
                result: LithoGenerationResult {
                    plan: plan_litho_generation(&paths, &result.project_slug, &result.repo_path),
                    response_excerpt: String::new(),
                    human_doc_count: result.human_doc_count,
                    human_docs_complete: result.human_docs_complete,
                },
            },
        );
    }

    app.emit(
        "project-init-done",
        ProjectInitDonePayload {
            project_slug: slug,
            result: result.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn get_env_status_cmd(repo_path: String) -> Result<EnvStatus, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    env_status_for_repo(&repo_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plan_env_integration_cmd(
    repo_path: String,
    selected_ids: Vec<String>,
) -> Result<EnvPlan, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    env_plan_for_repo(&repo_path, &selected_ids).map_err(|e| e.to_string())
}

#[derive(Clone, Serialize)]
struct EnvOptProgressPayload {
    repo_path: String,
    stage: String,
    message: String,
}

#[derive(Clone, Serialize)]
struct EnvOptDonePayload {
    repo_path: String,
    result: EnvApplyResult,
}

#[tauri::command]
pub async fn run_env_integration_cmd(
    app: AppHandle,
    repo_path: String,
    selected_ids: Vec<String>,
    reinstall_ids: Vec<String>,
) -> Result<EnvApplyResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let repo = repo_path.clone();

    let emit = |p: EnvApplyProgress| {
        let _ = app.emit(
            "env-opt-progress",
            EnvOptProgressPayload {
                repo_path: repo.clone(),
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let result = run_env_integration(&repo_path, &selected_ids, &reinstall_ids, emit)
        .await
        .map_err(|e| e.to_string())?;

    app.emit(
        "env-opt-done",
        EnvOptDonePayload {
            repo_path,
            result: result.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn get_project_overview_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<ProjectOverview, String> {
    get_project_overview(&state.paths, &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project_remark_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    remark: String,
) -> Result<ProjectOverview, String> {
    write_project_remark(&state.paths, &project_slug, &remark).map_err(|e| e.to_string())?;
    get_project_overview(&state.paths, &project_slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_sdd_status_cmd(
    state: State<'_, AppState>,
    project_slug: String,
) -> Result<SddStatus, String> {
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn create_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    title: String,
) -> Result<SddSessionInfo, String> {
    create_sdd_session(&state.paths, &project_slug, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    set_active_sdd_session(&state.paths, &project_slug, &session_id).map_err(|e| e.to_string())?;
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn delete_sdd_session_cmd(
    state: State<'_, AppState>,
    project_slug: String,
    session_id: String,
) -> Result<SddStatus, String> {
    mind_mesh_core::delete_sdd_session(&state.paths, &project_slug, &session_id)
        .map_err(|e| e.to_string())?;
    Ok(get_sdd_status(&state.paths, &project_slug))
}

#[tauri::command]
pub fn remove_project_cmd(state: State<'_, AppState>, project_slug: String) -> Result<(), String> {
    mind_mesh_core::unregister_project(&project_slug).map_err(|e| e.to_string())?;
    let _ = state;
    Ok(())
}

#[tauri::command]
pub fn save_sdd_output_cmd(
    state: State<'_, AppState>,
    output_path: String,
    content: String,
) -> Result<(), String> {
    save_sdd_output(&state.paths, &output_path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_sdd_phase_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
    session_id: Option<String>,
    phase: SddPhase,
    user_input: Option<String>,
) -> Result<SddPhaseResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let paths = state.paths.clone();
    let session_id = session_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| resolve_sdd_session_id(&paths, &slug));
    let input = user_input.unwrap_or_default();

    let acp = resolved_acp_settings();
    let engine = if execution_pure_acp(&acp) || phase == SddPhase::CodeGen {
        None
    } else {
        Some(state.chat_engine().await.map_err(|e| e.to_string())?)
    };

    let emit_progress = |p: SddProgress| {
        let _ = app.emit(
            "sdd-progress",
            SddProgressPayload {
                project_slug: slug.clone(),
                phase,
                stage: p.stage,
                message: p.message,
            },
        );
    };

    let result = run_sdd_phase(
        &paths,
        engine,
        &slug,
        &repo_path,
        &session_id,
        phase,
        &input,
        &acp,
        emit_progress,
    )
    .await
    .map_err(|e| e.to_string())?;

    app.emit(
        "sdd-done",
        SddDonePayload {
            project_slug: slug,
            result: result.clone(),
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn run_agent_context_generation_cmd(
    state: State<'_, AppState>,
    repo_path: String,
    project_slug: Option<String>,
) -> Result<AgentContextGenerationResult, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    let slug = project_slug.unwrap_or_else(|| slugify_repo(&repo_path));
    let acp = resolved_acp_settings();
    agent_execution_ready(&acp, &state.get_model_config()).map_err(|e| e.to_string())?;
    let engine = if execution_uses_native_llm(&acp) {
        Some(Arc::new(
            ChatEngine::new_native(state.paths.clone(), state.get_model_config())
                .map_err(|e| e.to_string())?,
        ))
    } else {
        None
    };
    run_agent_context_generation(&state.paths, engine, &acp, &slug, &repo_path)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct AskKnowledgeReply {
    pub answer: String,
    pub citations: Vec<SourceCitation>,
    pub tool_calls: Vec<ChatToolCallRecord>,
    pub usage: ChatTokenUsage,
    pub completed_at: u64,
}

#[tauri::command]
pub async fn ask_knowledge_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    repo_path: Option<String>,
    request_id: Option<String>,
) -> Result<AskKnowledgeReply, String> {
    let stream_id = request_id.unwrap_or_else(|| format!("ask-{}", query.len()));

    let engine = match state.chat_engine().await {
        Ok(e) => e,
        Err(e) => {
            return Ok(fallback_search_reply(&state.paths, &query, project, repo_path, &e));
        }
    };

    let app_emit = app.clone();
    let app_emit_tools = app.clone();
    let app_emit_phase = app.clone();
    let app_emit_usage = app.clone();
    let sid = stream_id.clone();
    let sid_tools = stream_id.clone();
    let sid_phase = stream_id.clone();
    let sid_usage = stream_id.clone();
    let project_ref = project.clone();
    let repo_ref = repo_path.clone();

    let ChatReply {
        answer,
        citations,
        tool_calls,
        usage,
        completed_at,
    } = engine
        .ask(
            &stream_id,
            &query,
            project_ref.as_deref(),
            repo_ref.as_deref(),
            move |chunk| {
                let _ = app_emit.emit(
                    "chat-chunk",
                    ChatChunkPayload {
                        session_id: sid.clone(),
                        text: chunk.to_string(),
                    },
                );
            },
            move |calls| {
                let _ = app_emit_tools.emit(
                    "chat-tool-calls",
                    ChatToolCallsPayload {
                        session_id: sid_tools.clone(),
                        tool_calls: calls.to_vec(),
                    },
                );
            },
            move |phase| {
                let _ = app_emit_phase.emit(
                    "chat-phase",
                    ChatPhasePayload {
                        session_id: sid_phase.clone(),
                        phase,
                    },
                );
            },
            move |usage| {
                let _ = app_emit_usage.emit(
                    "chat-usage",
                    ChatUsagePayload {
                        session_id: sid_usage.clone(),
                        usage: usage.clone(),
                    },
                );
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    app.emit(
        "chat-done",
        ChatDonePayload {
            session_id: stream_id.clone(),
            answer: answer.clone(),
            citations: citations.clone(),
            tool_calls: tool_calls.clone(),
            usage: usage.clone(),
            completed_at,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(AskKnowledgeReply {
        answer,
        citations,
        tool_calls,
        usage,
        completed_at,
    })
}

fn fallback_search_reply(
    paths: &KnowledgePaths,
    query: &str,
    project: Option<String>,
    repo_path: Option<String>,
    llm_error: &str,
) -> AskKnowledgeReply {
    let hits = KnowledgeSearch::new(paths)
        .search(
            query,
            SearchOptions {
                project: project.clone(),
                doc_type: None,
                limit: 5,
            },
        )
        .unwrap_or_default();

    let citations: Vec<SourceCitation> = hits
        .iter()
        .map(|h| SourceCitation {
            kind: if h.path.contains("/human/") {
                mind_mesh_core::CitationKind::HumanDoc
            } else {
                mind_mesh_core::CitationKind::StructuredDoc
            },
            title: h.title.clone().unwrap_or_else(|| h.path.clone()),
            path: h.path.clone(),
            repo_path: repo_path.clone(),
            start_line: None,
            end_line: None,
            excerpt: Some(h.snippet.clone()),
        })
        .collect();

    let mut all_citations = citations;
    all_citations = merge_citations(
        all_citations,
        extract_source_citations(query, repo_path.as_deref()),
    );

    let answer = if all_citations.is_empty() {
        format!("LLM unavailable ({llm_error}). No matching documents found.")
    } else {
        format!(
            "LLM unavailable ({llm_error}). Found {} document reference(s) via search.",
            all_citations.len()
        )
    };

    AskKnowledgeReply {
        answer,
        citations: all_citations,
        tool_calls: Vec::new(),
        usage: ChatTokenUsage::default(),
        completed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    }
}

fn slugify_repo(repo_path: &str) -> String {
    slug::slugify(
        std::path::Path::new(repo_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project"),
    )
}
