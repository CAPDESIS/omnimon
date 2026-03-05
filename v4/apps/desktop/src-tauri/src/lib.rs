use macmon_core::browser::{BrowserKind, BrowserTab, NativeTabProvider, TabProvider};
use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessRefreshKind, System};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

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
    pub swap_used_mb: u32,
    pub total_processes: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    pub processes: Vec<ProcessEntry>,
    pub stats: SystemStats,
}

/// Persistent sysinfo instance for per-process CPU tracking.
/// CPU usage requires consecutive refreshes to produce meaningful values.
static PROCESS_SYSTEM: OnceLock<Mutex<System>> = OnceLock::new();

fn process_system() -> &'static Mutex<System> {
    PROCESS_SYSTEM.get_or_init(|| Mutex::new(System::new_all()))
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
#[tauri::command]
fn get_metrics() -> Result<Metrics, String> {
    // System-level stats from the cached watcher (O(1))
    let sys_state = macmon_core::watcher::get_cached_state();

    // Top processes sorted by memory from core
    let top_procs = macmon_core::metrics::top_processes_by_memory(100);

    // Refresh persistent System — only processes (skip disks, networks, components)
    let mut system = process_system()
        .lock()
        .map_err(|e| format!("system lock poisoned: {e}"))?;
    system.refresh_processes_specifics(ProcessRefreshKind::everything());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let processes: Vec<ProcessEntry> = top_procs
        .iter()
        .map(|entry| {
            let proc_info = system.process(Pid::from_u32(entry.pid));

            let cpu_pct = proc_info.map(|p| p.cpu_usage() as f64).unwrap_or(0.0);

            let exec_name = proc_info
                .and_then(|p| {
                    p.exe().map(|e| {
                        e.file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| entry.name.clone())
                    })
                })
                .unwrap_or_else(|| entry.name.clone());

            let start_time = proc_info.map(|p| p.start_time()).unwrap_or(now);
            let uptime = format_uptime(now.saturating_sub(start_time));

            let ram_mb = entry.memory_bytes as f64 / 1_048_576.0;
            let is_system = macmon_core::killer::is_immutable_blocked_process_name(&entry.name);
            let idle = cpu_pct < 1.0 && !is_system;

            // Tag Browser group by process name pattern — no AppleScript, instant
            let group = if exec_name.contains("Google Chrome Helper")
                || entry.name == "com.apple.WebKit.WebContent"
                || exec_name.contains("Safari")
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
        swap_used_mb: 0,
        total_processes: total_procs,
    };

    Ok(Metrics { processes, stats })
}

/// Cached browser tabs — refreshed in background, served instantly.
static TAB_CACHE: OnceLock<Mutex<(Vec<BrowserTab>, Instant)>> = OnceLock::new();

/// How often to actually run AppleScript/CDP (seconds).
const TAB_CACHE_TTL_SECS: u64 = 5;

fn tab_cache() -> &'static Mutex<(Vec<BrowserTab>, Instant)> {
    TAB_CACHE.get_or_init(|| Mutex::new((Vec::new(), Instant::now() - std::time::Duration::from_secs(TAB_CACHE_TTL_SECS + 1))))
}

fn refresh_tab_cache_if_stale() -> Vec<BrowserTab> {
    let mut cache = tab_cache().lock().unwrap_or_else(|e| e.into_inner());
    if cache.1.elapsed().as_secs() < TAB_CACHE_TTL_SECS {
        return cache.0.clone();
    }
    // Stale — refresh now
    let provider = NativeTabProvider;
    let mut tabs = provider.list_tabs(BrowserKind::Chrome).unwrap_or_default();
    #[cfg(target_os = "macos")]
    {
        tabs.extend(provider.list_tabs(BrowserKind::Safari).unwrap_or_default());
    }
    cache.0 = tabs.clone();
    cache.1 = Instant::now();
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
    let kind = match browser.as_str() {
        "Chrome" => BrowserKind::Chrome,
        "Safari" => BrowserKind::Safari,
        _ => return Err(format!("Unknown browser: {browser}")),
    };
    let provider = NativeTabProvider;
    let tab = BrowserTab {
        id: tab_id,
        title: String::new(),
        url: tab_url,
        browser: kind,
    };
    provider.close_tab(kind, &tab)
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

/// IPC: Kill multiple processes by PIDs. Returns list of actually killed PIDs.
#[tauri::command]
fn kill_processes(pids: Vec<u32>) -> Result<Vec<u32>, String> {
    let mut killed = Vec::new();
    for pid in pids {
        match macmon_core::killer::kill_process_safe(pid as i32, &[]) {
            Ok(_) => killed.push(pid),
            Err(e) => eprintln!("[kill_processes] PID {pid}: {e}"),
        }
    }
    Ok(killed)
}

/// IPC: Save AI Configuration to OS Keyring
#[tauri::command]
fn save_ai_config(_provider: String, _model: String, key: String) -> Result<(), String> {
    macmon_core::ai::save_api_key(&key).map_err(|e| e.to_string())
}

/// IPC: Analyze processes using AI
#[tauri::command]
async fn analyze_processes(
    profile: String,
) -> Result<Vec<macmon_core::ai::ProcessSuggestion>, String> {
    let top_procs = macmon_core::metrics::top_processes_by_memory(30);
    let mut procs_to_send = Vec::new();

    for p in top_procs {
        if !macmon_core::killer::is_immutable_blocked_process_name(&p.name) {
            procs_to_send.push(serde_json::json!({
                "pid": p.pid,
                "name": p.name,
                "memory_mb": p.memory_bytes / 1_048_576
            }));
        }
    }

    let processes_json = serde_json::to_string(&procs_to_send).map_err(|e| e.to_string())?;

    let provider = "openrouter";
    let model = "google/gemini-flash-1.5-8b";

    let mut suggestions =
        macmon_core::ai::analyze_with_ai(provider, model, &processes_json, &profile)
            .await
            .map_err(|e| e.to_string())?;

    suggestions.retain(|s| !macmon_core::killer::is_immutable_blocked_process_name(&s.name));

    Ok(suggestions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Start the background watcher thread for system-level metrics
            macmon_core::watcher::start_watcher();

            let quit = MenuItem::with_id(app, "quit", "Quit macmon", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Monitor", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("macmon - System Monitor")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_metrics,
            kill_process,
            kill_processes,
            save_ai_config,
            analyze_processes,
            get_browser_tabs,
            close_browser_tab,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
