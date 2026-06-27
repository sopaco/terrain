use terrain_agent::{env_plan_for_repo, env_status_for_repo, run_env_integration, validate_repo_path};
use terrain_core::{EnvApplyProgress, EnvApplyResult, EnvPlan, EnvStatus};
use tauri::{AppHandle, Emitter};

use super::payloads::{EnvOptDonePayload, EnvOptProgressPayload};

#[tauri::command]
pub fn get_env_status_cmd(repo_path: String) -> Result<EnvStatus, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    env_status_for_repo(&repo_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plan_env_integration_cmd(
    repo_path: String,
    selected_ids: Vec<String>,
    reinstall_ids: Vec<String>,
) -> Result<EnvPlan, String> {
    validate_repo_path(&repo_path).map_err(|e| e.to_string())?;
    env_plan_for_repo(&repo_path, &selected_ids, &reinstall_ids).map_err(|e| e.to_string())
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
