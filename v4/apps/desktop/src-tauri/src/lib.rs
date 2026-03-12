pub mod automations;
pub mod plugins;
use macmon_core::browser::{
    sanitize_tab_id, sanitize_tab_url, BrowserKind, BrowserTab, NativeTabProvider, TabProvider,
};
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_store::StoreExt;

const MAX_AI_RULES_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_NETWORK_ALERT_RULES_PAYLOAD_BYTES: usize = 128 * 1024;
const MAX_KILL_BATCH: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub exec_name: String,
    pub exe_path: Option<String>,
    pub bundle_id: Option<String>,
    pub icon_data_url: Option<String>,
    pub ram_mb: f64,
    pub cpu_pct: f64,
    pub disk_read_mb: f64,
    pub disk_write_mb: f64,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub energy_impact_score: Option<f64>,
    pub uptime: String,
    pub group: String,
    pub group_key: String,
    pub group_identity_type: String,
    pub grouped_name: String,
    pub process_count: u32,
    pub is_system: bool,
    pub idle: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub cpu_usage_pct: f64,
    pub ram_total_gb: f64,
    pub ram_used_pct: u32,
    pub swap_used_mb: u64,
    pub total_processes: u32,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    pub processes: Vec<ProcessEntry>,
    pub stats: SystemStats,
}

fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// IPC: Return real processes + system stats in a single call.
///
/// All data is read from caches populated by background threads — no heavy OS
/// calls or mutex contention happen on the main/IPC thread.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_metrics(idle_threshold: Option<f64>) -> Result<Metrics, String> {
    let snapshot = macmon_core::telemetry::telemetry_snapshot(Some(100));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let processes: Vec<ProcessEntry> = snapshot
        .processes
        .iter()
        .map(|entry| {
            let cpu_pct = entry.cpu_usage_percent as f64;
            let exec_name = entry.exec_name.clone();
            let uptime = format_uptime(now.saturating_sub(entry.start_time));

            let ram_mb = entry.memory_bytes as f64 / 1_048_576.0;
            let disk_read_mb = entry.disk_read_bytes as f64 / 1_048_576.0;
            let disk_write_mb = entry.disk_write_bytes as f64 / 1_048_576.0;
            let is_system = entry.is_system;
            let threshold = idle_threshold.unwrap_or(1.0);
            // Idle = no CPU activity AND no network activity AND not a system process.
            // This prevents marking apps with active connections (Chrome, WhatsApp,
            // Slack, etc.) as inactive even when their CPU is momentarily 0%.
            let has_cpu = cpu_pct >= threshold;
            let has_network = entry.net_rx_bytes_per_sec > 0 || entry.net_tx_bytes_per_sec > 0;
            let idle = !is_system && !has_cpu && !has_network;

            ProcessEntry {
                pid: entry.pid,
                name: entry.name.clone(),
                exec_name,
                exe_path: entry.exe_path.clone(),
                bundle_id: entry.bundle_id.clone(),
                icon_data_url: entry.icon_data_url.clone(),
                ram_mb: (ram_mb * 10.0).round() / 10.0,
                cpu_pct: (cpu_pct * 10.0).round() / 10.0,
                disk_read_mb: (disk_read_mb * 10.0).round() / 10.0,
                disk_write_mb: (disk_write_mb * 10.0).round() / 10.0,
                net_rx_bytes_per_sec: entry.net_rx_bytes_per_sec,
                net_tx_bytes_per_sec: entry.net_tx_bytes_per_sec,
                energy_impact_score: entry
                    .energy_impact_score
                    .map(|value| (value as f64 * 10.0).round() / 10.0),
                uptime,
                group: entry.group.clone(),
                group_key: entry.group_key.clone(),
                group_identity_type: entry.group_identity_type.clone(),
                grouped_name: entry.grouped_display_name.clone(),
                process_count: entry.process_count as u32,
                is_system,
                idle,
                state: if idle { "S".into() } else { "R".into() },
            }
        })
        .collect();

    let total_procs = processes.len() as u32;

    let stats = SystemStats {
        cpu_usage_pct: (snapshot.cpu_usage_percent as f64 * 10.0).round() / 10.0,
        ram_total_gb: (snapshot.total_memory_bytes as f64 / 1_073_741_824.0 * 10.0).round() / 10.0,
        ram_used_pct: if snapshot.total_memory_bytes > 0 {
            ((snapshot.used_memory_bytes as f64 / snapshot.total_memory_bytes as f64) * 100.0)
                as u32
        } else {
            0
        },
        swap_used_mb: snapshot.swap_used_mb,
        total_processes: total_procs,
        net_rx_bytes_per_sec: snapshot.net_rx_bytes_per_sec,
        net_tx_bytes_per_sec: snapshot.net_tx_bytes_per_sec,
    };

    Ok(Metrics { processes, stats })
}

/// Cached browser tabs — refreshed in background, served instantly.
/// Wrapped in Arc so concurrent reads clone the Arc (O(1)) instead of the Vec.
static TAB_CACHE: OnceLock<Mutex<(Arc<Vec<BrowserTab>>, Instant)>> = OnceLock::new();

/// How often to actually run AppleScript/CDP (seconds).
const TAB_CACHE_TTL_SECS: u64 = 5;

fn tab_cache() -> &'static Mutex<(Arc<Vec<BrowserTab>>, Instant)> {
    TAB_CACHE.get_or_init(|| {
        Mutex::new((
            Arc::new(Vec::new()),
            Instant::now() - std::time::Duration::from_secs(TAB_CACHE_TTL_SECS + 1),
        ))
    })
}

/// Prevents multiple concurrent tab refreshes.
static TAB_REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn refresh_tab_cache_if_stale() -> Arc<Vec<BrowserTab>> {
    // Check staleness under lock, then drop lock before expensive work.
    {
        let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        if cache.1.elapsed().as_secs() < TAB_CACHE_TTL_SECS {
            return Arc::clone(&cache.0);
        }
    }

    // Prevent multiple concurrent refreshes — if another thread is already
    // refreshing, return the (stale) cached data instead of blocking.
    if TAB_REFRESH_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        return Arc::clone(&cache.0);
    }

    // Expensive AppleScript/CDP work happens outside the Mutex.
    // Wrapped in catch_unwind so a panic here resets the flag instead of
    // permanently blocking all future tab refreshes.
    let result = std::panic::catch_unwind(|| {
        let provider = NativeTabProvider;
        let mut tabs = Vec::new();
        for browser in BrowserKind::all() {
            match provider.list_tabs(*browser) {
                Ok(t) => tabs.extend(t),
                Err(e) => eprintln!(
                    "[tab-cache] {} tab listing failed: {}",
                    browser.display_name(),
                    e
                ),
            }
        }
        tabs
    });

    let tabs = match result {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[tab-cache] panic during tab refresh — returning stale cache");
            TAB_REFRESH_IN_PROGRESS.store(false, Ordering::SeqCst);
            let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
            return Arc::clone(&cache.0);
        }
    };

    let arc_tabs = Arc::new(tabs);

    // Re-acquire lock to update cache.
    {
        let mut cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        cache.0 = Arc::clone(&arc_tabs);
        cache.1 = Instant::now();
    }

    TAB_REFRESH_IN_PROGRESS.store(false, Ordering::SeqCst);
    arc_tabs
}

/// IPC: List open browser tabs — returns from cache, refreshes in background if stale.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_browser_tabs() -> Result<Vec<BrowserTab>, String> {
    // Return cached data instantly — Arc clone is O(1)
    let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
    let tabs = Arc::clone(&cache.0);
    let stale = cache.1.elapsed().as_secs() >= TAB_CACHE_TTL_SECS;
    drop(cache);

    // If stale, refresh in background thread (don't block IPC)
    if stale {
        std::thread::spawn(|| {
            refresh_tab_cache_if_stale();
        });
    }

    // Unwrap Arc: if we're the sole owner, avoid clone; otherwise clone the Vec
    Ok(Arc::try_unwrap(tabs).unwrap_or_else(|arc| (*arc).clone()))
}

/// IPC: Gracefully close a browser tab via AppleScript/CDP (not process kill).
#[tauri::command]
#[tracing::instrument(skip_all)]
fn close_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
    macmon_core::rate_limit::check_rate_limit(
        "close_browser_tab",
        &macmon_core::rate_limit::profiles::BROWSER,
    )?;
    sanitize_tab_id(&tab_id)?;
    sanitize_tab_url(&tab_url)?;
    let kind = BrowserKind::from_str(&browser)?;
    let provider = NativeTabProvider;
    let tab = BrowserTab {
        id: tab_id,
        title: String::new(),
        url: tab_url,
        browser: kind,
    };
    provider.close_tab(kind, &tab)
}

/// IPC: Focus (navigate to) a browser tab via AppleScript/CDP.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn focus_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
    macmon_core::rate_limit::check_rate_limit(
        "focus_browser_tab",
        &macmon_core::rate_limit::profiles::BROWSER,
    )?;
    sanitize_tab_id(&tab_id)?;
    sanitize_tab_url(&tab_url)?;
    let kind = BrowserKind::from_str(&browser)?;
    let provider = NativeTabProvider;
    let tab = BrowserTab {
        id: tab_id,
        title: String::new(),
        url: tab_url,
        browser: kind,
    };
    provider.focus_tab(kind, &tab)
}

/// IPC: Kill a single process by PID using the real OS-native killer.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn kill_process(pid: u32) -> Result<bool, String> {
    macmon_core::rate_limit::check_rate_limit(
        "kill_process",
        &macmon_core::rate_limit::profiles::KILL,
    )?;
    match macmon_core::killer::kill_process_safe(pid as i32, &[]) {
        Ok(_) => Ok(true),
        Err(macmon_core::killer::KillError::ProcessNotFound(_)) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Result of a bulk kill operation, reporting both successes and failures.
#[derive(Debug, Clone, Serialize)]
pub struct KillProcessesResult {
    pub killed: Vec<u32>,
    pub failed: Vec<(u32, String)>,
}

/// IPC: Kill multiple processes by PIDs. Returns killed and failed PIDs with error messages.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn kill_processes(pids: Vec<u32>) -> Result<KillProcessesResult, String> {
    if pids.len() > MAX_KILL_BATCH {
        return Err(format!("batch limited to {} PIDs", MAX_KILL_BATCH));
    }
    let mut killed = Vec::new();
    let mut failed = Vec::new();
    for pid in pids {
        macmon_core::rate_limit::check_rate_limit(
            "kill_processes",
            &macmon_core::rate_limit::profiles::KILL,
        )?;
        match macmon_core::killer::kill_process_safe(pid as i32, &[]) {
            Ok(_) => killed.push(pid),
            Err(e) => failed.push((pid, e.to_string())),
        }
    }
    Ok(KillProcessesResult { killed, failed })
}

/// Legacy store filename — only used for migrating existing plain-text keys.
const LEGACY_STORE_FILENAME: &str = "ai_keys.json";

/// Retrieve an API key from the OS keyring.
///
/// If a legacy Tauri Store key exists, it is migrated to the keyring and
/// removed from the store.  Plain-text storage is never used for new keys.
fn get_api_key_with_fallback(app: &AppHandle, provider: &str) -> Result<String, String> {
    let ai_provider = macmon_core::ai::AiProvider::from_str(provider)?;

    // 1) Try OS keyring (secure path)
    if let Ok(key) = macmon_core::ai::get_api_key(ai_provider) {
        return Ok(key);
    }

    // 2) Check for legacy plain-text key and migrate it to keyring
    if let Ok(store) = app.store(LEGACY_STORE_FILENAME) {
        if let Some(legacy_key) = store
            .get(ai_provider.keyring_service())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            tracing::warn!(
                "Migrating legacy plain-text API key for {} to OS keyring",
                provider
            );
            if macmon_core::ai::save_api_key(ai_provider, &legacy_key).is_ok() {
                store.delete(ai_provider.keyring_service());
                let _ = store.save();
                return Ok(legacy_key);
            }
            // Keyring still broken — return the legacy key this one time but warn
            tracing::error!(
                "OS keyring unavailable — legacy key used but migration failed for {}",
                provider
            );
            return Ok(legacy_key);
        }
    }

    Err(format!(
        "No API key found for {provider}. Save one with the Settings panel or 'omnimon apikey'."
    ))
}

/// IPC: Save AI Configuration — keyring only, no plain-text fallback.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn save_ai_config(
    _app: AppHandle,
    provider: String,
    _model: String,
    key: String,
) -> Result<(), String> {
    macmon_core::rate_limit::check_rate_limit(
        "save_ai_config",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    let trimmed_key = key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;

    macmon_core::ai::save_api_key(ai_provider, &trimmed_key).map_err(|e| {
        format!(
            "Failed to save API key to OS keyring: {e}. \
             Ensure your OS keyring service is available."
        )
    })
}

/// IPC: Check whether an API key exists (keyring or store).
#[tauri::command]
#[tracing::instrument(skip_all)]
fn check_api_key(app: AppHandle, provider: String) -> Result<bool, String> {
    Ok(get_api_key_with_fallback(&app, &provider).is_ok())
}

/// IPC: Apply AI-generated rules payload directly into core rules engine.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn apply_ai_rules(payload: String) -> Result<usize, String> {
    macmon_core::rate_limit::check_rate_limit(
        "apply_ai_rules",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    if payload.len() > MAX_AI_RULES_PAYLOAD_BYTES {
        return Err(format!(
            "payload exceeds {}KB limit",
            MAX_AI_RULES_PAYLOAD_BYTES / 1024
        ));
    }
    macmon_core::rules_engine::upsert_rules_from_ai_json(&payload)
}

/// IPC: Return JSON schema contract for AI rules payload.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_ai_rules_schema() -> String {
    macmon_core::rules_engine::ai_rules_schema_json()
}

/// IPC: Validate AI API key by making a test request
#[tauri::command]
#[tracing::instrument(skip_all)]
async fn validate_api_key(provider: String, key: String) -> Result<bool, String> {
    macmon_core::rate_limit::check_rate_limit(
        "validate_api_key",
        &macmon_core::rate_limit::profiles::AI,
    )?;
    let trimmed_key = key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;
    match macmon_core::ai::validate_api_key(ai_provider, "", &trimmed_key).await {
        Ok(()) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

/// IPC: Analyze processes using AI
#[tauri::command]
#[tracing::instrument(skip_all)]
async fn analyze_processes(
    app: AppHandle,
    profile: String,
    provider: String,
    model: String,
) -> Result<Vec<macmon_core::ai::ProcessSuggestion>, String> {
    macmon_core::rate_limit::check_rate_limit(
        "analyze_processes",
        &macmon_core::rate_limit::profiles::AI,
    )?;
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;
    let api_key = get_api_key_with_fallback(&app, &provider)?;

    let sys_state = macmon_core::watcher::get_cached_state();
    let mut top_procs = sys_state.cached_process_info;
    top_procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    top_procs.truncate(30);

    let mut procs_to_send = Vec::new();

    for p in &top_procs {
        if !macmon_core::killer::is_immutable_blocked_process_name(&p.name) {
            procs_to_send.push(serde_json::json!({
                "pid": p.pid,
                "name": p.name,
                "memory_mb": p.memory_bytes / 1_048_576
            }));
        }
    }

    let processes_json = serde_json::to_string(&procs_to_send).map_err(|e| e.to_string())?;

    let mut suggestions = macmon_core::ai::analyze_with_ai_key(
        ai_provider,
        &model,
        &processes_json,
        &profile,
        &api_key,
    )
    .await
    .map_err(|e| e.to_string())?;

    suggestions.retain(|s| !macmon_core::killer::is_immutable_blocked_process_name(&s.name));

    Ok(suggestions)
}

/// IPC: Free-form AI analysis of a process context (returns plain text).
#[tauri::command]
#[tracing::instrument(skip_all)]
async fn analyze_context(
    app: AppHandle,
    context: String,
    provider: String,
    model: String,
) -> Result<String, String> {
    macmon_core::rate_limit::check_rate_limit(
        "analyze_context",
        &macmon_core::rate_limit::profiles::AI,
    )?;
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;
    let api_key = get_api_key_with_fallback(&app, &provider)?;
    macmon_core::ai::analyze_context_key(ai_provider, &model, &context, &api_key)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
fn set_network_alert_rules(payload_json: String) -> Result<usize, String> {
    macmon_core::rate_limit::check_rate_limit(
        "set_network_alert_rules",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    if payload_json.len() > MAX_NETWORK_ALERT_RULES_PAYLOAD_BYTES {
        return Err("network alert rules payload too large".to_string());
    }

    let rules: Vec<macmon_core::network_alerts::NetworkAlertRule> =
        serde_json::from_str(&payload_json)
            .map_err(|e| format!("invalid network alert rules JSON: {e}"))?;
    let count = rules.len();
    macmon_core::network_alerts::set_active_rules(rules);
    Ok(count)
}

/// IPC: Return real network telemetry data (top processes by throughput + recent connections).
///
/// Data comes from the background watcher's cached state — no expensive OS calls on the IPC thread.
/// Returns empty arrays when the network-capture feature is unavailable or no data has been collected yet.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_network_data() -> Result<serde_json::Value, String> {
    let state = macmon_core::watcher::get_cached_state();
    Ok(serde_json::json!({
        "top_processes": state.top_network_processes,
        "recent_connections": state.recent_network_connections,
        "net_rx_bytes_per_sec": state.net_rx_bytes_per_sec,
        "net_tx_bytes_per_sec": state.net_tx_bytes_per_sec,
        "capture_backend": state.net_capture_backend,
        "dpi_active": state.net_dpi_active,
    }))
}

/// IPC: Return full network analysis snapshot with per-process summaries and connection details.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_network_connections() -> Result<macmon_core::network_analysis::NetworkSnapshot, String> {
    macmon_core::watcher::get_network_snapshot()
        .ok_or_else(|| "No network snapshot available yet".to_string())
}

/// IPC: Return historical network snapshots from the last N seconds.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_network_history(
    seconds: u32,
) -> Result<Vec<macmon_core::network_analysis::NetworkSnapshot>, String> {
    Ok(macmon_core::watcher::get_network_history(seconds))
}

/// IPC: Return filtered network connections from the current snapshot.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_filtered_connections(
    filter: macmon_core::network_analysis::NetworkFilter,
) -> Result<Vec<macmon_core::network_analysis::NetworkConnection>, String> {
    Ok(macmon_core::watcher::get_filtered_connections(&filter))
}

/// IPC: Query whether the main window is currently visible.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_window_visible(app: tauri::AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
#[allow(dead_code)]
fn save_cloud_key(key: String) -> Result<(), String> {
    macmon_core::rate_limit::check_rate_limit(
        "save_cloud_key",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    let entry = keyring::Entry::new("omnimon", "crabnebula_api_key").map_err(|e| e.to_string())?;
    entry.set_password(&key).map_err(|e| e.to_string())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
#[allow(dead_code)]
fn get_cloud_key() -> Result<String, String> {
    let entry = keyring::Entry::new("omnimon", "crabnebula_api_key").map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

/// IPC: Interactive AI chat with live system state injection and tool calling.
#[tauri::command]
#[tracing::instrument(skip_all)]
async fn ai_chat(
    app: AppHandle,
    message: String,
    provider: String,
    model: String,
    history: Vec<(String, String)>,
    cache_ttl_minutes: Option<u64>,
) -> Result<macmon_core::ai::ChatResponse, String> {
    macmon_core::rate_limit::check_rate_limit("ai_chat", &macmon_core::rate_limit::profiles::AI)?;
    macmon_core::ai::check_prompt_injection(&message).map_err(|e| e.to_string())?;
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;

    // Ollama doesn't need an API key
    let api_key = if ai_provider.requires_api_key() {
        get_api_key_with_fallback(&app, &provider)?
    } else {
        String::new()
    };

    // Build system prompt with live OS state + open browser tabs
    let sys_state = macmon_core::watcher::get_cached_state();
    let mut system_prompt = macmon_core::ai::build_chat_system_prompt(&sys_state);

    // Append browser tabs context so AI can make informed close_tabs decisions
    if let Ok(tabs) = get_browser_tabs() {
        if !tabs.is_empty() {
            system_prompt.push_str("\n\n## Open Browser Tabs\n");
            for tab in tabs.iter().take(30) {
                system_prompt.push_str(&format!(
                    "- [{:?}] {} | {}\n",
                    tab.browser, tab.title, tab.url
                ));
            }
            if tabs.len() > 30 {
                system_prompt.push_str(&format!("... and {} more tabs\n", tabs.len() - 30));
            }
            system_prompt.push_str("\nWhen using close_tabs, the pattern matches against tab URLs and titles. Use pipe (|) to separate multiple patterns. To close all EXCEPT certain tabs, use close_tabs with patterns matching the tabs TO CLOSE (not the ones to keep).");
        }
    }

    // Build messages array: history + current user message
    let mut messages = history;
    messages.push(("user".to_string(), message));

    // Send to LLM with streaming — tokens are emitted as Tauri events
    let app_for_stream = app.clone();
    let (ai_text, tool_call) = macmon_core::ai::chat_with_tools_streaming(
        ai_provider,
        &model,
        &api_key,
        &messages,
        &system_prompt,
        cache_ttl_minutes.unwrap_or(5),
        move |token| {
            let _ = app_for_stream.emit("ai-stream-token", token);
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    // If AI requested a tool call, execute it
    let tool_result = tool_call.map(|call| match call.tool.as_str() {
        "add_automation_rule" => {
            if let Ok(rule) =
                serde_json::from_value::<automations::AutomationRule>(call.args.clone())
            {
                automations::add_rule(&app, rule);
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: true,
                    details: "Added automation rule successfully".into(),
                    payload: None,
                }
            } else {
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: false,
                    details: "Failed to parse rule arguments".into(),
                    payload: None,
                }
            }
        }
        "remove_automation_rule" => {
            if let Some(id) = call.args["id"].as_str() {
                automations::remove_rule(&app, id);
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: true,
                    details: "Removed automation rule successfully".into(),
                    payload: None,
                }
            } else {
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: false,
                    details: "Failed to parse rule id".into(),
                    payload: None,
                }
            }
        }
        _ => macmon_core::ai::execute_tool_call(&call.tool, &call.args, &sys_state),
    });

    // Build reply text: include tool result feedback
    let reply = if let Some(ref result) = tool_result {
        if result.success {
            format!("{}\n\n[Action executed] {}", ai_text, result.details)
        } else {
            format!("{}\n\n[Action failed] {}", ai_text, result.details)
        }
    } else {
        ai_text
    };

    Ok(macmon_core::ai::ChatResponse {
        reply,
        tool_call: tool_result,
    })
}

#[tauri::command]
#[tracing::instrument(skip_all)]
fn clear_ai_cache() -> Result<(), String> {
    macmon_core::ai::clear_ai_cache();
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = app.emit("window-visibility", true);
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        let _ = app.emit("window-visibility", false);
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            hide_main_window(app);
        } else {
            show_main_window(app);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // --- macOS Application Menu Bar ---
            let about_metadata = AboutMetadata {
                name: Some("OmniMon".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                authors: Some(vec!["Jorge Salgado Miranda".into()]),
                copyright: Some("© 2024-2026 Jorge Salgado Miranda".into()),
                website: Some("https://github.com/chochy2001/omnimon".into()),
                website_label: Some("Más información".into()),
                comments: Some(
                    "System Monitor — monitoreo avanzado de procesos, pestañas y red.".into(),
                ),
                ..Default::default()
            };
            let about_item =
                PredefinedMenuItem::about(app, Some("Acerca de OmniMon"), Some(about_metadata))?;
            let hide = PredefinedMenuItem::hide(app, None)?;
            let hide_others = PredefinedMenuItem::hide_others(app, None)?;
            let show_all = PredefinedMenuItem::show_all(app, None)?;
            let quit_item = PredefinedMenuItem::quit(app, None)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let sep3 = PredefinedMenuItem::separator(app)?;

            let app_submenu = Submenu::with_items(
                app,
                "OmniMon",
                true,
                &[
                    &about_item,
                    &sep1,
                    &hide,
                    &hide_others,
                    &show_all,
                    &sep2,
                    &sep3,
                    &quit_item,
                ],
            )?;
            let app_menu = Menu::with_items(app, &[&app_submenu])?;
            app.set_menu(app_menu)?;

            // Start the background watcher thread for system-level metrics
            macmon_core::watcher::start_watcher();
            automations::start_engine(app.handle().clone());
            let _ = plugins::start_engine(app.handle().clone());

            // Emit dynamic security alerts to frontend in real time.
            // Guard: only spawn the alert thread once, even if setup() is called multiple times.
            static ALERT_THREAD_STARTED: AtomicBool = AtomicBool::new(false);
            if ALERT_THREAD_STARTED.swap(true, Ordering::SeqCst) {
                // Already running — skip duplicate spawn.
            } else {
                let app_for_alerts = app.handle().clone();
                std::thread::spawn(move || {
                    let mut dedupe = HashMap::<String, Instant>::new();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(900));
                        let state = macmon_core::watcher::get_cached_state();
                        let now = Instant::now();
                        dedupe.retain(|_, seen| now.duration_since(*seen).as_secs() < 20);

                        for alert in state.dynamic_rule_alerts {
                            let key = format!(
                                "{}:{}:{}:{}",
                                alert.rule_id, alert.pid, alert.dst_ip, alert.dst_port
                            );
                            if dedupe.contains_key(&key) {
                                continue;
                            }
                            dedupe.insert(key, now);
                            let _ = app_for_alerts.emit("security-alert", alert);
                        }

                        for alert in state.network_alerts {
                            if dedupe.contains_key(&alert.id) {
                                continue;
                            }
                            dedupe.insert(alert.id.clone(), now);
                            let _ = app_for_alerts.emit("network-alert", alert);
                        }
                    }
                });

                let app_for_metrics = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    if let Ok(metrics) = get_metrics(Some(1.0)) {
                        let _ = app_for_metrics.emit("metrics-update", metrics);
                    }
                });

                // Emit network-update events when new snapshots are available
                let app_for_network = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(5000));
                    if let Some(snapshot) = macmon_core::watcher::get_network_snapshot() {
                        let _ = app_for_network.emit("network-update", &snapshot);
                    }
                });
            } // end ALERT_THREAD_STARTED guard

            // --- System Tray Menu ---
            let show = MenuItem::with_id(app, "show", "Dashboard", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Configuración", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &settings, &sep, &quit])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("OmniMon - System Monitor")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => show_main_window(app),
                    "settings" => {
                        show_main_window(app);
                        let _ = app.emit("open-settings", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        toggle_main_window(tray.app_handle());
                    }
                    TrayIconEvent::DoubleClick { .. } => {
                        show_main_window(tray.app_handle());
                    }
                    _ => {}
                })
                .build(app)?;

            // --- Window close intercept: hide instead of quit ---
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Prevent the window from actually closing
                        api.prevent_close();
                        hide_main_window(&app_handle);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            get_network_data,
            get_network_connections,
            get_network_history,
            get_filtered_connections,
            kill_process,
            kill_processes,
            save_ai_config,
            check_api_key,
            apply_ai_rules,
            get_ai_rules_schema,
            validate_api_key,
            analyze_processes,
            analyze_context,
            set_network_alert_rules,
            ai_chat,
            clear_ai_cache,
            get_browser_tabs,
            close_browser_tab,
            focus_browser_tab,
            get_window_visible,
            save_cloud_key,
            get_cloud_key,
            automations::get_automation_rules,
            automations::add_automation_rule,
            automations::remove_automation_rule,
            plugins::list_plugins,
            plugins::install_plugin,
            plugins::set_plugin_enabled,
            plugins::remove_plugin,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[omnimon] fatal: tauri application failed to start: {e}");
            std::process::exit(1);
        });
}
