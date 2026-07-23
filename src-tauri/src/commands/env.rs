use terrain_core::{
    apply_env_integration, get_env_status, plan_env_integration, EnvApplyProgress, EnvApplyResult,
    EnvPlan, EnvStatus,
};
use tauri::{AppHandle, Emitter};

use super::payloads::{EnvOptDonePayload, EnvOptProgressPayload};
use super::util::{map_core_err, validate_repo};

#[tauri::command]
pub fn get_env_status_cmd(repo_path: String) -> Result<EnvStatus, String> {
    let path = validate_repo(&repo_path)?;
    map_core_err(get_env_status(&path))
}

#[tauri::command]
pub fn plan_env_integration_cmd(
    repo_path: String,
    selected_ids: Vec<String>,
    reinstall_ids: Vec<String>,
) -> Result<EnvPlan, String> {
    let path = validate_repo(&repo_path)?;
    map_core_err(plan_env_integration(&path, &selected_ids, &reinstall_ids))
}

#[tauri::command]
pub async fn run_env_integration_cmd(
    app: AppHandle,
    repo_path: String,
    selected_ids: Vec<String>,
    reinstall_ids: Vec<String>,
) -> Result<EnvApplyResult, String> {
    let path = validate_repo(&repo_path)?;
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

    let result = apply_env_integration(&path, &selected_ids, &reinstall_ids, emit)
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
