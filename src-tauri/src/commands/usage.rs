use terrain_core::{load_usage_snapshot, probe_usage_sources, UsageDetailLevel, UsageProbeResult, UsageSnapshot};

#[tauri::command]
pub fn usage_probe_cmd() -> UsageProbeResult {
    probe_usage_sources()
}

#[tauri::command]
pub async fn usage_snapshot_cmd(
    detail: UsageDetailLevel,
    force_refresh: bool,
) -> Result<UsageSnapshot, String> {
    let level = detail;
    tokio::task::spawn_blocking(move || load_usage_snapshot(level, force_refresh))
        .await
        .map_err(|e| e.to_string())
}
