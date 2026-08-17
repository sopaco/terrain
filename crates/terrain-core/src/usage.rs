//! Developer token usage via [ccusage](https://github.com/ccusage/ccusage) (local, read-only).
//!
//! Designed for lazy invocation from the desktop UI — no work at app startup.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, NaiveDate};
use serde::Deserialize;
use serde::Serialize;

use crate::bundled_tools::{bundled_tools, ensure_bundled_tools_initialized, packages_root};
use crate::error::{CoreError, Result};
use crate::platform::{agent_tool_filename, bundled_binary_candidates, platform_key, user_home};
use crate::process::command as hidden_command;
use crate::shell_path::{command_on_path, resolve_executable};

const CACHE_TTL: Duration = Duration::from_secs(120);
const OFFLINE_ARGS: &[&str] = &["--offline"];

static SNAPSHOT_CACHE: Mutex<Option<CachedSnapshot>> = Mutex::new(None);

#[derive(Clone)]
struct CachedSnapshot {
    detail: UsageDetailLevel,
    snapshot: UsageSnapshot,
    at: Instant,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageDetailLevel {
    Summary,
    Full,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize)]
pub struct UsageSourceStatus {
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub detected: bool,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub file_count: u32,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageTotals {
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub input_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub output_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_creation_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_read_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize)]
pub struct UsageModelBreakdown {
    pub model_name: String,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub input_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub output_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_creation_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize)]
pub struct UsagePeriodEntry {
    pub period: String,
    pub agent: Option<String>,
    pub agents: Vec<String>,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub input_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub output_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_creation_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub cache_read_tokens: u64,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub models_used: Vec<String>,
    pub model_breakdowns: Vec<UsageModelBreakdown>,
    /// Resolved local log file or folder for session rows (when available).
    pub source_path: Option<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize)]
pub struct UsageProbeResult {
    pub ccusage_available: bool,
    pub ccusage_version: Option<String>,
    pub ccusage_path: Option<String>,
    pub sources: Vec<UsageSourceStatus>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "snake_case"))]
#[derive(Debug, Clone, Serialize)]
pub struct UsageSnapshot {
    pub probe: UsageProbeResult,
    pub today: UsageTotals,
    pub week: UsageTotals,
    pub month: UsageTotals,
    pub daily: Vec<UsagePeriodEntry>,
    pub monthly: Vec<UsagePeriodEntry>,
    pub sessions: Vec<UsagePeriodEntry>,
    #[cfg_attr(feature = "ts-export", ts(type = "number"))]
    pub generated_at: u64,
    pub cached: bool,
    pub error: Option<String>,
}

/// Fast filesystem probe — no ccusage subprocess for reports.
pub fn probe_usage_sources() -> UsageProbeResult {
    let launch = resolve_ccusage_launch();
    let available = launch.is_some();
    let version = launch.as_ref().and_then(read_ccusage_version);
    UsageProbeResult {
        ccusage_available: available,
        ccusage_version: version,
        ccusage_path: launch.map(|l| l.label()),
        sources: detect_local_sources(),
    }
}

/// Load usage snapshot (summary or full). Uses in-memory cache unless `force_refresh`.
pub fn load_usage_snapshot(detail: UsageDetailLevel, force_refresh: bool) -> UsageSnapshot {
    let lang = crate::language::current_language();
    if !force_refresh
        && let Some(cached) = read_cache(detail) {
            return cached;
        }

    let probe = probe_usage_sources();
    let generated_at = now_ms();

    if !probe.ccusage_available {
        return UsageSnapshot {
            probe,
            today: UsageTotals::default(),
            week: UsageTotals::default(),
            month: UsageTotals::default(),
            daily: Vec::new(),
            monthly: Vec::new(),
            sessions: Vec::new(),
            generated_at,
            cached: false,
            error: Some(
                lang.tr(
                    "未检测到用量分析环境。请安装 bun 或 Node.js 后重试。",
                    "Usage analysis environment not detected. Install bun or Node.js and try again.",
                )
                .into(),
            ),
        };
    }

    if probe.sources.iter().all(|s| !s.detected) {
        return UsageSnapshot {
            probe,
            today: UsageTotals::default(),
            week: UsageTotals::default(),
            month: UsageTotals::default(),
            daily: Vec::new(),
            monthly: Vec::new(),
            sessions: Vec::new(),
            generated_at,
            cached: false,
            error: Some(
                lang.tr(
                    "未检测到本地 Agent 用量日志（Claude Code、OpenCode、Codex 等）。开始使用编码 Agent 后将自动出现。",
                    "No local agent usage logs detected (Claude Code, OpenCode, Codex, etc.). They will appear automatically once you start using a coding agent.",
                )
                .into(),
            ),
        };
    }

    match build_snapshot(&probe, detail, generated_at) {
        Ok(snapshot) => {
            write_cache(detail, &snapshot);
            snapshot
        }
        Err(e) => UsageSnapshot {
            probe,
            today: UsageTotals::default(),
            week: UsageTotals::default(),
            month: UsageTotals::default(),
            daily: Vec::new(),
            monthly: Vec::new(),
            sessions: Vec::new(),
            generated_at,
            cached: false,
            error: Some(e.to_string()),
        },
    }
}

fn build_snapshot(
    probe: &UsageProbeResult,
    detail: UsageDetailLevel,
    generated_at: u64,
) -> Result<UsageSnapshot> {
    let daily_since_days = match detail {
        UsageDetailLevel::Summary => 30,
        UsageDetailLevel::Full => 400,
    };
    let daily_since = since_days_ago(daily_since_days);
    let daily_json = run_ccusage_json(&["daily", "--since", &daily_since])?;
    let daily_report: CcusageDailyReport = serde_json::from_value(daily_json)
        .map_err(|e| CoreError::Other(format!("ccusage daily JSON: {e}")))?;

    let daily: Vec<UsagePeriodEntry> = daily_report
        .daily
        .into_iter()
        .map(map_entry)
        .collect();

    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let week_start = Local::now().date_naive() - chrono::Duration::days(6);
    let month_start = NaiveDate::from_ymd_opt(
        Local::now().year(),
        Local::now().month(),
        1,
    )
    .unwrap_or_else(|| Local::now().date_naive());

    let today = sum_entries(daily.iter().filter(|e| e.period == today_str));
    let week = sum_entries(daily.iter().filter(|e| parse_period_date(&e.period) >= week_start));
    let month = sum_entries(daily.iter().filter(|e| parse_period_date(&e.period) >= month_start));

    let monthly = if detail == UsageDetailLevel::Full {
        let monthly_since = since_days_ago(365 * 3);
        let monthly_json = run_ccusage_json(&["monthly", "--since", &monthly_since])?;
        let monthly_report: CcusageMonthlyReport = serde_json::from_value(monthly_json)
            .map_err(|e| CoreError::Other(format!("ccusage monthly JSON: {e}")))?;
        monthly_report
            .monthly
            .into_iter()
            .map(map_entry)
            .collect()
    } else {
        Vec::new()
    };

    let session_since = since_days_ago(30);
    let sessions = if detail == UsageDetailLevel::Full {
        let session_json = run_ccusage_json(&["session", "--since", &session_since])?;
        let session_report: CcusageSessionReport = serde_json::from_value(session_json)
            .map_err(|e| CoreError::Other(format!("ccusage session JSON: {e}")))?;
        session_report
            .session
            .into_iter()
            .map(map_session_entry)
            .collect()
    } else {
        Vec::new()
    };

    Ok(UsageSnapshot {
        probe: probe.clone(),
        today,
        week,
        month,
        daily,
        monthly,
        sessions,
        generated_at,
        cached: false,
        error: None,
    })
}

fn read_cache(detail: UsageDetailLevel) -> Option<UsageSnapshot> {
    let guard = SNAPSHOT_CACHE.lock().ok()?;
    let cached = guard.as_ref()?;
    if cached.at.elapsed() > CACHE_TTL {
        return None;
    }
    if cached.detail != detail && detail == UsageDetailLevel::Full {
        return None;
    }
    let mut snapshot = cached.snapshot.clone();
    snapshot.cached = true;
    Some(snapshot)
}

fn write_cache(detail: UsageDetailLevel, snapshot: &UsageSnapshot) {
    if let Ok(mut guard) = SNAPSHOT_CACHE.lock() {
        *guard = Some(CachedSnapshot {
            detail,
            snapshot: snapshot.clone(),
            at: Instant::now(),
        });
    }
}

/// How Terrain invokes ccusage.
#[derive(Debug, Clone)]
enum CcusageLaunch {
    /// Native binary (bundled, deployed, or on PATH).
    Direct(PathBuf),
    /// `bunx ccusage` — official recommended launcher.
    Bunx,
    /// `npx -y ccusage` — official fallback launcher.
    Npx,
}

impl CcusageLaunch {
    fn label(&self) -> String {
        match self {
            Self::Direct(path) => path.display().to_string(),
            Self::Bunx => "bunx ccusage".into(),
            Self::Npx => "npx -y ccusage".into(),
        }
    }
}

fn resolve_ccusage_launch() -> Option<CcusageLaunch> {
    // Terrain-bundled / deployed native binary — fastest when present.
    if let Some(bin) = bundled_ccusage() {
        return Some(CcusageLaunch::Direct(bin));
    }
    if let Some(home) = user_home() {
        let deployed = home.join(".terrain/bin").join(agent_tool_filename("ccusage"));
        if is_executable_file(&deployed) {
            return Some(CcusageLaunch::Direct(deployed));
        }
    }
    // Official package runners (ccusage docs recommend bunx / npx).
    if command_on_path("bunx") || command_on_path("bun") {
        return Some(CcusageLaunch::Bunx);
    }
    if command_on_path("npx") {
        return Some(CcusageLaunch::Npx);
    }
    resolve_executable("ccusage").map(CcusageLaunch::Direct)
}

fn bundled_ccusage() -> Option<PathBuf> {
    ensure_bundled_tools_initialized();
    if let Some(p) = bundled_tools().ccusage.clone() {
        return Some(p);
    }
    let root = packages_root();
    let platform = platform_key();
    let mut dirs = vec![platform.clone()];
    if platform != "darwin-arm64" {
        dirs.push("darwin-arm64".into());
    }
    for platform in dirs {
        let base = root.join("ccusage").join(platform);
        for name in bundled_binary_candidates("ccusage") {
            let candidate = base.join(&name);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn configure_ccusage_cmd(launch: &CcusageLaunch) -> std::process::Command {
    match launch {
        CcusageLaunch::Direct(bin) => hidden_command(bin),
        CcusageLaunch::Bunx => {
            let mut cmd = if command_on_path("bunx") {
                hidden_command("bunx")
            } else {
                let mut c = hidden_command("bun");
                c.arg("x");
                c
            };
            cmd.arg("ccusage");
            cmd
        }
        CcusageLaunch::Npx => {
            let mut cmd = hidden_command("npx");
            cmd.args(["-y", "ccusage"]);
            cmd
        }
    }
}

fn read_ccusage_version(launch: &CcusageLaunch) -> Option<String> {
    let output = configure_ccusage_cmd(launch)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

fn run_ccusage_json(args: &[&str]) -> Result<serde_json::Value> {
    let launch = resolve_ccusage_launch().ok_or_else(|| {
        CoreError::Other("ccusage not found (install bun or node for bunx/npx ccusage)".into())
    })?;
    let mut cmd = configure_ccusage_cmd(&launch);
    cmd.args(args)
        .args(OFFLINE_ARGS)
        .arg("--json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| CoreError::Other(format!("failed to run {}: {e}", launch.label())))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        return Err(CoreError::Other(if msg.is_empty() {
            format!("ccusage exited with {}", output.status)
        } else {
            msg.to_string()
        }));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| CoreError::Other(format!("ccusage output is not valid JSON: {e}")))
}

fn detect_local_sources() -> Vec<UsageSourceStatus> {
    vec![
        probe_source_dir(
            "claude",
            "Claude Code",
            claude_data_dirs(),
            &["jsonl"],
        ),
        probe_source_dir(
            "opencode",
            "OpenCode",
            opencode_data_dirs(),
            &["json", "db"],
        ),
        probe_source_dir("codex", "Codex", codex_data_dirs(), &["jsonl"]),
    ]
}

fn probe_source_dir(
    id: &str,
    label: &str,
    dirs: Vec<PathBuf>,
    extensions: &[&str],
) -> UsageSourceStatus {
    let mut best_path: Option<PathBuf> = None;
    let mut file_count = 0u32;
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let count = count_matching_files(&dir, extensions, 2);
        if count > 0 {
            file_count = file_count.saturating_add(count);
            if best_path.is_none() {
                best_path = Some(dir);
            }
        }
    }
    UsageSourceStatus {
        id: id.to_string(),
        label: label.to_string(),
        path: best_path.map(|p| p.display().to_string()),
        detected: file_count > 0,
        file_count,
    }
}

fn claude_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = user_home() {
        dirs.push(home.join(".config/claude/projects"));
        dirs.push(home.join(".claude/projects"));
    }
    if let Ok(custom) = std::env::var("CLAUDE_CONFIG_DIR") {
        let p = PathBuf::from(custom);
        dirs.push(p.join("projects"));
        dirs.push(p);
    }
    dirs
}

fn opencode_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(custom) = std::env::var("OPENCODE_DATA_DIR") {
        for part in custom.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                dirs.push(PathBuf::from(trimmed));
            }
        }
    }
    if let Some(home) = user_home() {
        dirs.push(home.join(".local/share/opencode"));
    }
    dirs
}

fn codex_data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = user_home() {
        dirs.push(home.join(".codex"));
    }
    dirs
}

fn count_matching_files(dir: &Path, extensions: &[&str], max_depth: u32) -> u32 {
    count_matching_files_inner(dir, extensions, max_depth, 0)
}

fn count_matching_files_inner(
    dir: &Path,
    extensions: &[&str],
    max_depth: u32,
    depth: u32,
) -> u32 {
    if depth > max_depth {
        return 0;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0u32;
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && extensions.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                    count = count.saturating_add(1);
                }
        } else if path.is_dir() && depth < max_depth {
            count = count.saturating_add(count_matching_files_inner(
                &path,
                extensions,
                max_depth,
                depth + 1,
            ));
        }
    }
    count
}

fn map_entry(entry: CcusageEntry) -> UsagePeriodEntry {
    let agents = entry
        .metadata
        .and_then(|m| m.agents)
        .unwrap_or_default();
    UsagePeriodEntry {
        period: entry.period.unwrap_or_default(),
        agent: entry.agent,
        agents,
        input_tokens: entry.input_tokens,
        output_tokens: entry.output_tokens,
        cache_creation_tokens: entry.cache_creation_tokens,
        cache_read_tokens: entry.cache_read_tokens,
        total_tokens: entry.total_tokens,
        total_cost_usd: entry.total_cost,
        models_used: entry.models_used,
        model_breakdowns: entry
            .model_breakdowns
            .into_iter()
            .map(|m| UsageModelBreakdown {
                model_name: m.model_name,
                input_tokens: m.input_tokens,
                output_tokens: m.output_tokens,
                cache_creation_tokens: m.cache_creation_tokens,
                cache_read_tokens: m.cache_read_tokens,
                cost_usd: m.cost,
            })
            .collect(),
        source_path: None,
    }
}

fn map_session_entry(entry: CcusageEntry) -> UsagePeriodEntry {
    let mut row = map_entry(entry);
    row.source_path = resolve_usage_session_path(row.agent.as_deref(), &row.period)
        .map(|p| p.display().to_string());
    row
}

/// Resolve a usage session identifier to its local log file or folder.
pub fn resolve_usage_session_path(agent: Option<&str>, period: &str) -> Option<PathBuf> {
    if period.is_empty() {
        return None;
    }

    match agent.unwrap_or("") {
        "claude" => find_session_file(&claude_data_dirs(), period, "jsonl"),
        "codex" => find_codex_session(&codex_data_dirs(), period),
        "opencode" => find_opencode_session(&opencode_data_dirs(), period),
        _ => find_session_file(&claude_data_dirs(), period, "jsonl")
            .or_else(|| find_codex_session(&codex_data_dirs(), period))
            .or_else(|| find_opencode_session(&opencode_data_dirs(), period)),
    }
}

fn find_session_file(dirs: &[PathBuf], session_id: &str, ext: &str) -> Option<PathBuf> {
    let filename = format!("{session_id}.{ext}");
    for dir in dirs {
        if let Some(path) = find_file_by_name(dir, &filename, 5) {
            return Some(path);
        }
    }
    None
}

fn find_codex_session(dirs: &[PathBuf], period: &str) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(period);
        let with_jsonl = if candidate.extension().is_none() {
            candidate.with_extension("jsonl")
        } else {
            candidate.clone()
        };
        if with_jsonl.is_file() {
            return Some(with_jsonl);
        }
        if candidate.is_dir() {
            return Some(candidate);
        }
        if let Some(path) = find_file_by_name(dir, &format!("{period}.jsonl"), 6) {
            return Some(path);
        }
    }
    None
}

fn find_opencode_session(dirs: &[PathBuf], period: &str) -> Option<PathBuf> {
    for dir in dirs {
        if let Some(path) = find_file_by_name(dir, &format!("{period}.json"), 6) {
            return Some(path);
        }
        if let Some(path) = find_file_by_name(dir, period, 6) {
            return Some(path);
        }
    }
    None
}

fn find_file_by_name(dir: &Path, filename: &str, max_depth: u32) -> Option<PathBuf> {
    find_file_by_name_inner(dir, filename, max_depth, 0)
}

fn find_file_by_name_inner(
    dir: &Path,
    filename: &str,
    max_depth: u32,
    depth: u32,
) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return None;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == filename)
            {
                return Some(path);
            }
        } else if path.is_dir() && depth < max_depth {
            if let Some(found) = find_file_by_name_inner(&path, filename, max_depth, depth + 1) {
                return Some(found);
            }
        }
    }
    None
}

fn sum_entries<'a>(entries: impl Iterator<Item = &'a UsagePeriodEntry>) -> UsageTotals {
    let mut totals = UsageTotals::default();
    for e in entries {
        totals.input_tokens += e.input_tokens;
        totals.output_tokens += e.output_tokens;
        totals.cache_creation_tokens += e.cache_creation_tokens;
        totals.cache_read_tokens += e.cache_read_tokens;
        totals.total_tokens += e.total_tokens;
        totals.total_cost_usd += e.total_cost_usd;
    }
    totals
}

fn parse_period_date(period: &str) -> NaiveDate {
    NaiveDate::parse_from_str(period, "%Y-%m-%d").unwrap_or(NaiveDate::MIN)
}

fn since_days_ago(days: i64) -> String {
    let date = Local::now().date_naive() - chrono::Duration::days(days);
    date.format("%Y%m%d").to_string()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CcusageTotals {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    total_tokens: u64,
    total_cost: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CcusageEntry {
    agent: Option<String>,
    period: Option<String>,
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    total_tokens: u64,
    total_cost: f64,
    models_used: Vec<String>,
    model_breakdowns: Vec<CcusageModelBreakdown>,
    metadata: Option<CcusageMetadata>,
}

#[derive(Debug, Deserialize)]
struct CcusageMetadata {
    agents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CcusageModelBreakdown {
    model_name: String,
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: u64,
    cache_read_tokens: u64,
    cost: f64,
}

#[derive(Debug, Deserialize)]
struct CcusageDailyReport {
    daily: Vec<CcusageEntry>,
    #[allow(dead_code)]
    totals: CcusageTotals,
}

#[derive(Debug, Deserialize)]
struct CcusageMonthlyReport {
    monthly: Vec<CcusageEntry>,
    #[allow(dead_code)]
    totals: CcusageTotals,
}

#[derive(Debug, Deserialize)]
struct CcusageSessionReport {
    session: Vec<CcusageEntry>,
    #[allow(dead_code)]
    totals: CcusageTotals,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::user_home;

    #[test]
    fn probe_sources_returns_known_agents() {
        let probe = probe_usage_sources();
        assert_eq!(probe.sources.len(), 3);
        assert!(probe.sources.iter().any(|s| s.id == "claude"));
    }

    #[test]
    fn since_days_ago_is_eight_digits() {
        let s = since_days_ago(7);
        assert_eq!(s.len(), 8);
        assert!(s.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn resolve_claude_session_path_finds_jsonl() {
        let Some(home) = user_home() else {
            return;
        };
        let projects = home.join(".claude/projects");
        if !projects.is_dir() {
            return;
        }
        let Ok(read) = std::fs::read_dir(&projects) else {
            return;
        };
        for project in read.flatten() {
            let project_dir = project.path();
            if !project_dir.is_dir() {
                continue;
            }
            let Ok(files) = std::fs::read_dir(&project_dir) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                let Some(session_id) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let resolved = resolve_usage_session_path(Some("claude"), session_id);
                assert_eq!(resolved.as_deref(), Some(path.as_path()));
                return;
            }
        }
    }
}
