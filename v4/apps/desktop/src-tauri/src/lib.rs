use macmon_core::browser::{
    sanitize_tab_id, sanitize_tab_url, BrowserKind, BrowserTab, NativeTabProvider, TabProvider,
};
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub exec_name: String,
    pub ram_mb: f64,
    pub cpu_pct: f64,
    pub uptime: String,
    pub group: String,
    pub is_system: bool,
    pub idle: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
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
fn get_metrics(idle_threshold: Option<f64>) -> Result<Metrics, String> {
    // All data from the background watcher cache (RwLock read, O(1) — no syscalls)
    let sys_state = macmon_core::watcher::get_cached_state();

    // Sort cached processes by memory descending, take top 100
    let mut top_procs = sys_state.cached_process_info.clone();
    top_procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    top_procs.truncate(100);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let processes: Vec<ProcessEntry> = top_procs
        .iter()
        .map(|entry| {
            let cpu_pct = entry.cpu_pct as f64;
            let exec_name = entry.exec_name.clone();
            let uptime = format_uptime(now.saturating_sub(entry.start_time));

            let ram_mb = entry.memory_bytes as f64 / 1_048_576.0;
            let is_system = macmon_core::killer::is_immutable_blocked_process_name(&entry.name);
            let threshold = idle_threshold.unwrap_or(1.0);
            let idle = cpu_pct < threshold && !is_system;

            // Tag Browser group by process name pattern — no AppleScript, instant
            let name = &entry.name;
            let group = if exec_name.contains("Google Chrome Helper")
                || exec_name.contains("Google Chrome")
                || name == "com.apple.WebKit.WebContent"
                || exec_name.contains("Safari")
                || name.contains("Safari")
                || exec_name.contains("Brave Browser Helper")
                || exec_name.contains("Brave Browser")
                || exec_name.contains("Microsoft Edge Helper")
                || exec_name.contains("Microsoft Edge")
                || exec_name.contains("Arc Helper")
                || exec_name == "Arc"
                || exec_name.contains("firefox")
                || name.contains("firefox")
            {
                "Browser".to_string()
            } else {
                String::new()
            };

            ProcessEntry {
                pid: entry.pid,
                name: entry.name.clone(),
                exec_name,
                ram_mb: (ram_mb * 10.0).round() / 10.0,
                cpu_pct: (cpu_pct * 10.0).round() / 10.0,
                uptime,
                group,
                is_system,
                idle,
                state: if idle { "S".into() } else { "R".into() },
            }
        })
        .collect();

    let total_procs = processes.len() as u32;

    let stats = SystemStats {
        ram_total_gb: (sys_state.total_memory_bytes as f64 / 1_073_741_824.0 * 10.0).round() / 10.0,
        ram_used_pct: if sys_state.total_memory_bytes > 0 {
            ((sys_state.used_memory_bytes as f64 / sys_state.total_memory_bytes as f64) * 100.0)
                as u32
        } else {
            0
        },
        swap_used_mb: sys_state.swap_used_mb,
        total_processes: total_procs,
        net_rx_bytes_per_sec: sys_state.net_rx_bytes_per_sec,
        net_tx_bytes_per_sec: sys_state.net_tx_bytes_per_sec,
    };

    Ok(Metrics { processes, stats })
}

/// Cached browser tabs — refreshed in background, served instantly.
static TAB_CACHE: OnceLock<Mutex<(Vec<BrowserTab>, Instant)>> = OnceLock::new();

/// How often to actually run AppleScript/CDP (seconds).
const TAB_CACHE_TTL_SECS: u64 = 5;

fn tab_cache() -> &'static Mutex<(Vec<BrowserTab>, Instant)> {
    TAB_CACHE.get_or_init(|| {
        Mutex::new((
            Vec::new(),
            Instant::now() - std::time::Duration::from_secs(TAB_CACHE_TTL_SECS + 1),
        ))
    })
}

/// Prevents multiple concurrent tab refreshes.
static TAB_REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn refresh_tab_cache_if_stale() -> Vec<BrowserTab> {
    // Check staleness under lock, then drop lock before expensive work.
    {
        let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        if cache.1.elapsed().as_secs() < TAB_CACHE_TTL_SECS {
            return cache.0.clone();
        }
    }

    // Prevent multiple concurrent refreshes — if another thread is already
    // refreshing, return the (stale) cached data instead of blocking.
    if TAB_REFRESH_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        return cache.0.clone();
    }

    // Expensive AppleScript/CDP work happens outside the Mutex.
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

    // Re-acquire lock to update cache.
    {
        let mut cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
        cache.0 = tabs.clone();
        cache.1 = Instant::now();
    }

    TAB_REFRESH_IN_PROGRESS.store(false, Ordering::SeqCst);
    tabs
}

/// IPC: List open browser tabs — returns from cache, refreshes in background if stale.
#[tauri::command]
fn get_browser_tabs() -> Result<Vec<BrowserTab>, String> {
    // Return cached data instantly
    let cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
    let tabs = cache.0.clone();
    let stale = cache.1.elapsed().as_secs() >= TAB_CACHE_TTL_SECS;
    drop(cache);

    // If stale, refresh in background thread (don't block IPC)
    if stale {
        std::thread::spawn(|| {
            refresh_tab_cache_if_stale();
        });
    }

    Ok(tabs)
}

/// IPC: Gracefully close a browser tab via AppleScript/CDP (not process kill).
#[tauri::command]
fn close_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
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
fn focus_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
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
fn kill_process(pid: u32) -> Result<bool, String> {
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
fn kill_processes(pids: Vec<u32>) -> Result<KillProcessesResult, String> {
    let mut killed = Vec::new();
    let mut failed = Vec::new();
    for pid in pids {
        match macmon_core::killer::kill_process_safe(pid as i32, &[]) {
            Ok(_) => killed.push(pid),
            Err(e) => failed.push((pid, e.to_string())),
        }
    }
    Ok(KillProcessesResult { killed, failed })
}

/// Store key name for the fallback API key file.
const STORE_FILENAME: &str = "ai_keys.json";

/// Try keyring first, then Tauri Store.
fn get_api_key_with_fallback(app: &AppHandle, provider: &str) -> Result<String, String> {
    let ai_provider = macmon_core::ai::AiProvider::from_str(provider)?;

    // 1) Try OS keyring
    if let Ok(key) = macmon_core::ai::get_api_key(ai_provider) {
        return Ok(key);
    }

    // 2) Fallback: Tauri Store
    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
    store
        .get(ai_provider.keyring_service())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| "No API key found (keyring and store both empty)".to_string())
}

/// IPC: Save AI Configuration — keyring first, Tauri Store as fallback.
#[tauri::command]
fn save_ai_config(
    app: AppHandle,
    provider: String,
    _model: String,
    key: String,
) -> Result<(), String> {
    let trimmed_key = key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("API key cannot be empty".to_string());
    }
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;

    // Try keyring first
    if macmon_core::ai::save_api_key(ai_provider, &trimmed_key).is_ok() {
        return Ok(());
    }

    // Fallback: persist in Tauri Store
    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
    store.set(
        ai_provider.keyring_service(),
        serde_json::Value::String(trimmed_key),
    );
    Ok(())
}

/// IPC: Check whether an API key exists (keyring or store).
#[tauri::command]
fn check_api_key(app: AppHandle, provider: String) -> Result<bool, String> {
    Ok(get_api_key_with_fallback(&app, &provider).is_ok())
}

/// IPC: Apply AI-generated rules payload directly into core rules engine.
#[tauri::command]
fn apply_ai_rules(payload: String) -> Result<usize, String> {
    macmon_core::rules_engine::upsert_rules_from_ai_json(&payload)
}

/// IPC: Return JSON schema contract for AI rules payload.
#[tauri::command]
fn get_ai_rules_schema() -> String {
    macmon_core::rules_engine::ai_rules_schema_json()
}

/// IPC: Validate AI API key by making a test request
#[tauri::command]
async fn validate_api_key(provider: String, key: String) -> Result<bool, String> {
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
async fn analyze_processes(
    app: AppHandle,
    profile: String,
    provider: String,
    model: String,
) -> Result<Vec<macmon_core::ai::ProcessSuggestion>, String> {
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
async fn analyze_context(
    app: AppHandle,
    context: String,
    provider: String,
    model: String,
) -> Result<String, String> {
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;
    let api_key = get_api_key_with_fallback(&app, &provider)?;
    macmon_core::ai::analyze_context_key(ai_provider, &model, &context, &api_key)
        .await
        .map_err(|e| e.to_string())
}

/// IPC: Query whether the main window is currently visible.
#[tauri::command]
fn get_window_visible(app: tauri::AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

#[tauri::command]
#[allow(dead_code)]
fn save_cloud_key(key: String) -> Result<(), String> {
    let entry =
        keyring::Entry::new("omnimon", "crabnebula_api_key").map_err(|e| e.to_string())?;
    entry.set_password(&key).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(dead_code)]
fn get_cloud_key() -> Result<String, String> {
    let entry =
        keyring::Entry::new("omnimon", "crabnebula_api_key").map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // Start the background watcher thread for system-level metrics
            macmon_core::watcher::start_watcher();

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
                    }
                });
            } // end ALERT_THREAD_STARTED guard

            // --- System Tray Menu ---
            let show = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit OmniMon", true, None::<&str>)?;

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
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_main_window(tray.app_handle());
                    }
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
            kill_process,
            kill_processes,
            save_ai_config,
            check_api_key,
            apply_ai_rules,
            get_ai_rules_schema,
            validate_api_key,
            analyze_processes,
            analyze_context,
            get_browser_tabs,
            close_browser_tab,
            focus_browser_tab,
            get_window_visible,
            save_cloud_key,
            get_cloud_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
