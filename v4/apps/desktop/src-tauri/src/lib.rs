pub mod automations;
pub mod plugins;
pub mod zombie_killer;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiLocale {
    En,
    Es,
}

fn detect_system_locale() -> UiLocale {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if value.to_ascii_lowercase().starts_with("es") {
                return UiLocale::Es;
            }
        }
    }
    UiLocale::En
}

fn read_ui_locale(app: &AppHandle) -> UiLocale {
    if let Ok(store) = app.store("preferences.json") {
        if let Some(value) = store.get("localePreference") {
            if let Some(locale) = value.as_str() {
                return match locale {
                    "es" => UiLocale::Es,
                    "en" => UiLocale::En,
                    _ => detect_system_locale(),
                };
            }
        }
    }
    detect_system_locale()
}

fn tr(locale: UiLocale, key: &str) -> &'static str {
    match (locale, key) {
        (UiLocale::Es, "about.more_info") => "Más información",
        (UiLocale::En, "about.more_info") => "More information",
        (UiLocale::Es, "about.comments") => {
            "System Monitor — monitoreo avanzado de procesos, pestañas y red."
        }
        (UiLocale::En, "about.comments") => {
            "System Monitor - advanced monitoring for processes, tabs, and network."
        }
        (UiLocale::Es, "about.title") => "Acerca de OmniMon",
        (UiLocale::En, "about.title") => "About OmniMon",
        (UiLocale::Es, "tray.dashboard") => "Dashboard",
        (UiLocale::En, "tray.dashboard") => "Dashboard",
        (UiLocale::Es, "tray.settings") => "Configuración",
        (UiLocale::En, "tray.settings") => "Settings",
        (UiLocale::Es, "tray.quit") => "Salir",
        (UiLocale::En, "tray.quit") => "Quit",
        (UiLocale::Es, "tray.tooltip_idle") => "OmniMon - Monitor del sistema",
        (UiLocale::En, "tray.tooltip_idle") => "OmniMon - System Monitor",
        _ => "OmniMon",
    }
}

fn tray_tooltip(locale: UiLocale, cpu_pct: f64, ram_used_gb: f64, ram_used_pct: u32) -> String {
    match locale {
        UiLocale::Es => format!(
            "OmniMon - CPU: {:.1}% | RAM: {:.1}GB ({:.0}%)",
            cpu_pct, ram_used_gb, ram_used_pct
        ),
        UiLocale::En => format!(
            "OmniMon - CPU: {:.1}% | RAM: {:.1}GB ({:.0}%)",
            cpu_pct, ram_used_gb, ram_used_pct
        ),
    }
}

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
                .min(100.0) as u32
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

#[inline]
fn acquire_tab_cache() -> std::sync::MutexGuard<'static, (Arc<Vec<BrowserTab>>, Instant)> {
    tab_cache().lock().unwrap_or_else(|e| e.into_inner())
}

/// Prevents multiple concurrent tab refreshes.
static TAB_REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

fn refresh_tab_cache_if_stale() -> Arc<Vec<BrowserTab>> {
    // Check staleness under lock, then drop lock before expensive work.
    {
        let cache = acquire_tab_cache();
        if cache.1.elapsed().as_secs() < TAB_CACHE_TTL_SECS {
            return Arc::clone(&cache.0);
        }
    }

    // Prevent multiple concurrent refreshes — if another thread is already
    // refreshing, return the (stale) cached data instead of blocking.
    if TAB_REFRESH_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        let cache = acquire_tab_cache();
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
                Err(e) => tracing::error!(
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
            tracing::error!("[tab-cache] panic during tab refresh — returning stale cache");
            TAB_REFRESH_IN_PROGRESS.store(false, Ordering::SeqCst);
            let cache = acquire_tab_cache();
            return Arc::clone(&cache.0);
        }
    };

    let arc_tabs = Arc::new(tabs);

    // Re-acquire lock to update cache.
    {
        let mut cache = acquire_tab_cache();
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
    let cache = acquire_tab_cache();
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

/// Validates and prepares a browser tab for IPC operations.
fn prepare_browser_tab(
    command: &'static str,
    tab_id: &str,
    tab_url: &str,
    browser: &str,
) -> Result<(BrowserTab, BrowserKind), String> {
    macmon_core::rate_limit::check_rate_limit(
        command,
        &macmon_core::rate_limit::profiles::BROWSER,
    )?;
    sanitize_tab_id(tab_id)?;
    sanitize_tab_url(tab_url)?;
    let kind = BrowserKind::from_str(browser)?;
    let tab = BrowserTab {
        id: tab_id.to_string(),
        title: String::new(),
        url: tab_url.to_string(),
        browser: kind,
    };
    Ok((tab, kind))
}

/// IPC: Gracefully close a browser tab via AppleScript/CDP (not process kill).
#[tauri::command]
#[tracing::instrument(skip_all)]
fn close_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
    let (tab, kind) = prepare_browser_tab("close_browser_tab", &tab_id, &tab_url, &browser)?;
    NativeTabProvider.close_tab(kind, &tab)
}

/// IPC: Focus (navigate to) a browser tab via AppleScript/CDP.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn focus_browser_tab(tab_id: String, tab_url: String, browser: String) -> Result<bool, String> {
    let (tab, kind) = prepare_browser_tab("focus_browser_tab", &tab_id, &tab_url, &browser)?;
    NativeTabProvider.focus_tab(kind, &tab)
}

/// IPC: Check if CDP (Chrome DevTools Protocol) is available for supported browsers.
/// Returns a map of browser names to availability status.
#[tauri::command]
fn check_cdp_availability() -> std::collections::HashMap<String, bool> {
    use macmon_core::browser::{cdp_is_available, BrowserKind};

    let mut status = std::collections::HashMap::new();

    for browser in BrowserKind::all() {
        if browser.supports_cdp() {
            let port = browser.cdp_port();
            let base = format!("http://localhost:{}", port);
            let available = cdp_is_available(&base);
            status.insert(browser.display_name().to_string(), available);
        }
    }

    status
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

fn kill_processes_with<F>(pids: Vec<u32>, mut kill_one: F) -> Result<KillProcessesResult, String>
where
    F: FnMut(u32) -> Result<(), String>,
{
    if pids.len() > MAX_KILL_BATCH {
        return Err(format!("error_batch_limit:{}", MAX_KILL_BATCH));
    }

    let mut killed = Vec::new();
    let mut failed = Vec::new();
    for pid in pids {
        match kill_one(pid) {
            Ok(()) => killed.push(pid),
            Err(err) => failed.push((pid, err)),
        }
    }
    Ok(KillProcessesResult { killed, failed })
}

/// IPC: Kill multiple processes by PIDs. Returns killed and failed PIDs with error messages.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn kill_processes(pids: Vec<u32>) -> Result<KillProcessesResult, String> {
    kill_processes_with(pids, |pid| {
        macmon_core::rate_limit::check_rate_limit(
            "kill_processes",
            &macmon_core::rate_limit::profiles::KILL,
        )?;
        macmon_core::killer::kill_process_safe(pid as i32, &[])
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
}

/// Legacy store filename — only used for migrating existing plain-text keys.
const LEGACY_STORE_FILENAME: &str = "ai_keys.json";

/// Retrieve an API key from the OS keyring.
///
/// If a legacy Tauri Store key exists, it is migrated to the keyring.
/// Plain-text storage is never used for new keys.
///
/// # Safety contract
///
/// The legacy plain-text value is **erased from disk before any keyring
/// operation is attempted**. This guarantees there is no window in which
/// both the plain-text copy on disk *and* a keyring entry coexist, and
/// that a mid-flight crash (or a keyring that hangs forever) cannot leave
/// the secret readable on the filesystem.
///
/// Failure modes:
/// - Keyring save succeeds → key is returned; plain-text is already gone.
/// - Keyring save fails → the caller still receives the key in memory
///   for this session (so the AI call can proceed), but the plain-text
///   copy on disk has already been wiped. The user must re-enter the key
///   next session if the keyring remains unavailable. There is never a
///   state in which the disk copy survives a failed migration.
fn get_api_key_with_fallback(app: &AppHandle, provider: &str) -> Result<String, String> {
    let ai_provider = macmon_core::ai::AiProvider::from_str(provider)?;

    // 1) Try OS keyring (secure path) first.
    if let Ok(key) = macmon_core::ai::get_api_key(ai_provider) {
        return Ok(key);
    }

    // 2) Legacy plain-text migration: delete-first, then attempt secure save.
    let store = match app.store(LEGACY_STORE_FILENAME) {
        Ok(store) => store,
        Err(_) => return Err(format!("error_no_api_key:{}", provider)),
    };

    let service = ai_provider.keyring_service();
    let legacy_key = match store
        .get(service)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        Some(key) => key,
        None => return Err(format!("error_no_api_key:{}", provider)),
    };

    // Wipe plain-text from the store and persist the deletion BEFORE we touch
    // the keyring. If the app crashes between here and the save call below,
    // the worst case is that the user must re-enter the key — the secret is
    // never left on disk after this function has observed it.
    store.delete(service);
    if let Err(e) = store.save() {
        tracing::error!(
            "Failed to persist legacy-store deletion for {} ({}). Continuing with in-memory key; \
             the disk copy may survive and must be removed manually if this error repeats.",
            provider,
            e
        );
    }

    match macmon_core::ai::save_api_key(ai_provider, &legacy_key) {
        Ok(()) => {
            tracing::info!(
                "Migrated legacy plain-text API key for {} to OS keyring (plain-text wiped)",
                provider
            );
            Ok(legacy_key)
        }
        Err(e) => {
            // Keyring unavailable. The disk copy is already erased (see above),
            // so there is no persistent plain-text leak. Return the in-memory
            // copy for this session only; next session the user will be asked
            // to re-enter the key if the keyring is still broken.
            tracing::error!(
                "OS keyring unavailable during migration for {} ({}). Plain-text was wiped from \
                 disk; user must re-enter the key next session if keyring remains broken.",
                provider,
                e
            );
            Ok(legacy_key)
        }
    }
}

/// IPC: Save AI Configuration — keyring only, no plain-text fallback.
fn save_ai_config_with<F>(provider: &str, key: &str, mut save_key: F) -> Result<(), String>
where
    F: FnMut(macmon_core::ai::AiProvider, &str) -> Result<(), String>,
{
    let trimmed_key = key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("error_api_key_empty".to_string());
    }
    let ai_provider = macmon_core::ai::AiProvider::from_str(provider)?;
    save_key(ai_provider, &trimmed_key)
}

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
    save_ai_config_with(&provider, &key, |ai_provider, trimmed_key| {
        macmon_core::ai::save_api_key(ai_provider, trimmed_key).map_err(|e| {
            format!(
                "Failed to save API key to OS keyring: {e}. \
                 Ensure your OS keyring service is available."
            )
        })
    })
}

/// IPC: Check whether an API key exists (keyring or store).
#[tauri::command]
#[tracing::instrument(skip_all)]
fn check_api_key(app: AppHandle, provider: String) -> Result<bool, String> {
    Ok(get_api_key_with_fallback(&app, &provider).is_ok())
}

fn apply_ai_rules_with<F>(payload: &str, mut upsert_rules: F) -> Result<usize, String>
where
    F: FnMut(&str) -> Result<usize, String>,
{
    if payload.len() > MAX_AI_RULES_PAYLOAD_BYTES {
        return Err(format!(
            "payload exceeds {}KB limit",
            MAX_AI_RULES_PAYLOAD_BYTES / 1024
        ));
    }
    upsert_rules(payload)
}

/// IPC: Apply AI-generated rules payload directly into core rules engine.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn apply_ai_rules(payload: String) -> Result<usize, String> {
    macmon_core::rate_limit::check_rate_limit(
        "apply_ai_rules",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    apply_ai_rules_with(
        &payload,
        macmon_core::rules_engine::upsert_rules_from_ai_json,
    )
}

/// IPC: Return JSON schema contract for AI rules payload.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_ai_rules_schema() -> String {
    macmon_core::rules_engine::ai_rules_schema_json()
}

/// Effective per-UTC-day AI call ceiling for the current user.
///
/// All remote AI commands (`ai_chat`, `analyze_processes`, `analyze_context`,
/// `validate_api_key`) share the same `ai_daily` bucket, so a user that spent
/// their budget on chat cannot spend another copy of it on analysis. This
/// mirrors how cost on the LLM-provider side works — tokens are fungible.
fn ai_daily_limit_effective() -> u32 {
    macmon_core::settings::read_settings()
        .ai_daily_limit
        .unwrap_or(macmon_core::rate_limit::DEFAULT_AI_DAILY_LIMIT)
}

/// Shared bucket name for the daily AI budget. Every remote AI command
/// spends one token from this bucket. Local-only providers (Ollama) should
/// configure `ai_daily_limit = Some(0)` to disable the cap.
const AI_DAILY_BUCKET: &str = "ai_daily";

/// IPC: Return `(used_today, configured_limit)` for the shared AI budget.
/// The UI can render "X / Y calls today" without mutating the counter.
#[tauri::command]
#[tracing::instrument(skip_all)]
fn get_ai_daily_usage() -> (u32, u32) {
    let (used, _stored_limit) = macmon_core::rate_limit::daily_usage(AI_DAILY_BUCKET);
    (used, ai_daily_limit_effective())
}

/// IPC: Validate AI API key by making a test request
#[tauri::command]
#[tracing::instrument(skip_all)]
async fn validate_api_key(provider: String, key: String) -> Result<bool, String> {
    macmon_core::rate_limit::check_rate_limit(
        "validate_api_key",
        &macmon_core::rate_limit::profiles::AI,
    )?;
    macmon_core::rate_limit::check_daily_limit(AI_DAILY_BUCKET, ai_daily_limit_effective())?;
    let trimmed_key = key.trim().to_string();
    if trimmed_key.is_empty() {
        return Err("error_api_key_empty".to_string());
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
    macmon_core::rate_limit::check_daily_limit(AI_DAILY_BUCKET, ai_daily_limit_effective())?;
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;
    let api_key = get_api_key_with_fallback(&app, &provider)?;

    let sys_state = macmon_core::watcher::get_cached_state();
    let mut top_procs = sys_state.cached_process_info;
    top_procs.sort_by_key(|p| std::cmp::Reverse(p.memory_bytes));
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
    macmon_core::rate_limit::check_daily_limit(AI_DAILY_BUCKET, ai_daily_limit_effective())?;
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
    set_network_alert_rules_with(&payload_json, macmon_core::network_alerts::set_active_rules)
}

fn parse_network_alert_rules_payload(
    payload_json: &str,
) -> Result<Vec<macmon_core::network_alerts::NetworkAlertRule>, String> {
    if payload_json.len() > MAX_NETWORK_ALERT_RULES_PAYLOAD_BYTES {
        return Err("error_payload_too_large".to_string());
    }
    serde_json::from_str(payload_json).map_err(|e| format!("error_invalid_json:{}", e))
}

fn set_network_alert_rules_with<F>(payload_json: &str, mut set_rules: F) -> Result<usize, String>
where
    F: FnMut(Vec<macmon_core::network_alerts::NetworkAlertRule>),
{
    let rules = parse_network_alert_rules_payload(payload_json)?;
    let count = rules.len();
    set_rules(rules);
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
    macmon_core::rate_limit::check_daily_limit(AI_DAILY_BUCKET, ai_daily_limit_effective())?;
    macmon_core::ai::check_prompt_injection(&message).map_err(|e| e.to_string())?;
    let ai_provider = macmon_core::ai::AiProvider::from_str(&provider)?;

    // Ollama doesn't need an API key
    let api_key = if ai_provider.requires_api_key() {
        get_api_key_with_fallback(&app, &provider)?
    } else {
        String::new()
    };

    // Build system prompt with live OS state + open browser tabs.
    //
    // Privacy: if the user has enabled `ai_privacy_mode` in their settings,
    // process names are replaced by stable pseudonymous tokens before being
    // sent to the remote LLM provider. The AI can still reason about
    // "process X keeps appearing" but cannot correlate tokens back to real
    // application identities without access to the local machine.
    let sys_state = macmon_core::watcher::get_cached_state();
    let privacy_mode = macmon_core::settings::read_settings()
        .ai_privacy_mode
        .unwrap_or(false);
    let mut system_prompt =
        macmon_core::ai::build_chat_system_prompt_with_privacy(&sys_state, privacy_mode);

    // Append browser tabs context so AI can make informed close_tabs decisions.
    //
    // Privacy: tab titles and URLs routinely contain customer names, ticket
    // IDs, internal hostnames, and session tokens. When privacy mode is on
    // we redact each to a stable token so the LLM can still issue a
    // `close_tabs` pattern (matched locally on the real title/URL) without
    // having seen the plain text.
    if let Ok(tabs) = get_browser_tabs() {
        if !tabs.is_empty() {
            system_prompt.push_str("\n\n## Open Browser Tabs\n");
            for tab in tabs.iter().take(30) {
                system_prompt.push_str(&format!(
                    "- [{:?}] {} | {}\n",
                    tab.browser,
                    macmon_core::ai::redact_tab_title(&tab.title, privacy_mode),
                    macmon_core::ai::redact_url(&tab.url, privacy_mode)
                ));
            }
            if tabs.len() > 30 {
                system_prompt.push_str(&format!("... and {} more tabs\n", tabs.len() - 30));
            }
            if privacy_mode {
                system_prompt.push_str("\nPrivacy mode is active: tab titles/URLs have been tokenized. You may still ask the user to close tabs by category (e.g. \"all youtube tabs\"), and the frontend will match your pattern against the real titles locally.");
            } else {
                system_prompt.push_str("\nWhen using close_tabs, the pattern matches against tab URLs and titles. Use pipe (|) to separate multiple patterns. To close all EXCEPT certain tabs, use close_tabs with patterns matching the tabs TO CLOSE (not the ones to keep).");
            }
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

    // If AI requested a tool call, build a plan for the frontend to confirm.
    //
    // CONFIRMATION CONTRACT: automation tools are **not** executed server-side
    // anymore — an LLM could otherwise silently register an `action: "kill"`
    // rule that later terminates processes without user consent. We validate
    // the shape here and hand the payload back to the UI, which stages it as
    // a pending action and only invokes the actual `add_automation_rule` /
    // `remove_automation_rule` IPC after the user explicitly confirms.
    let tool_result = tool_call.map(|call| match call.tool.as_str() {
        "add_automation_rule" => {
            match serde_json::from_value::<automations::AutomationRule>(call.args.clone()) {
                Ok(rule) => {
                    let summary = format!(
                        "add_automation_rule:{}:{}:{}",
                        rule.process_pattern, rule.metric, rule.action
                    );
                    macmon_core::ai::ToolResult {
                        tool: call.tool,
                        success: true,
                        details: summary,
                        payload: Some(call.args.clone()),
                    }
                }
                Err(_) => macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: false,
                    details: "automation_rule_args_invalid".into(),
                    payload: None,
                },
            }
        }
        "remove_automation_rule" => {
            if let Some(id) = call.args["id"].as_str() {
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: true,
                    details: format!("remove_automation_rule:{}", id),
                    payload: Some(serde_json::json!({ "id": id })),
                }
            } else {
                macmon_core::ai::ToolResult {
                    tool: call.tool,
                    success: false,
                    details: "automation_rule_id_invalid".into(),
                    payload: None,
                }
            }
        }
        _ => macmon_core::ai::execute_tool_call(&call.tool, &call.args, &sys_state),
    });

    // Build reply text: include tool result feedback
    let reply = ai_text;

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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let locale = read_ui_locale(app.handle());
            // --- macOS Application Menu Bar ---
            let about_metadata = AboutMetadata {
                name: Some("OmniMon".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                authors: Some(vec!["Jorge Salgado Miranda".into()]),
                copyright: Some("© 2024-2026 Jorge Salgado Miranda".into()),
                website: Some("https://github.com/chochy2001/omnimon".into()),
                website_label: Some(tr(locale, "about.more_info").into()),
                comments: Some(tr(locale, "about.comments").into()),
                ..Default::default()
            };
            let about_item = PredefinedMenuItem::about(
                app,
                Some(tr(locale, "about.title")),
                Some(about_metadata),
            )?;
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
            zombie_killer::start_engine(app.handle().clone());

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
                            // Emit to frontend for UI handling
                            let _ = app_for_alerts.emit("security-alert", alert);

                            // TODO: Add notification actions when user clicks
                            // tauri-plugin-notification supports actions on Windows/macOS
                            // but requires frontend to handle the response via event listeners
                        }

                        for alert in state.network_alerts {
                            if dedupe.contains_key(&alert.id) {
                                continue;
                            }
                            dedupe.insert(alert.id.clone(), now);
                            // Emit to frontend for UI handling
                            let _ = app_for_alerts.emit("network-alert", alert);

                            // TODO: Add notification actions when user clicks
                            // Example: Show "Terminate Process" and "Ignore" buttons
                            // The frontend should register event listeners for these actions
                        }
                    }
                });

                let app_for_metrics = app.handle().clone();
                std::thread::spawn(move || loop {
                    // Aligned to 5s to match backend watcher interval and reduce CPU usage
                    std::thread::sleep(std::time::Duration::from_millis(5000));
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
            let show = MenuItem::with_id(
                app,
                "show",
                tr(locale, "tray.dashboard"),
                true,
                None::<&str>,
            )?;
            let settings = MenuItem::with_id(
                app,
                "settings",
                tr(locale, "tray.settings"),
                true,
                None::<&str>,
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", tr(locale, "tray.quit"), true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &settings, &sep, &quit])?;

            let tray_icon = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip(tr(locale, "tray.tooltip_idle"))
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

            // Update tray tooltip every 5 seconds with CPU/RAM stats
            let tray_for_tooltip = tray_icon.clone();
            let app_for_tooltip = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                if let Ok(metrics) = get_metrics(Some(1.0)) {
                    let tooltip = tray_tooltip(
                        read_ui_locale(&app_for_tooltip),
                        metrics.stats.cpu_usage_pct,
                        metrics.stats.ram_total_gb * (metrics.stats.ram_used_pct as f64 / 100.0),
                        metrics.stats.ram_used_pct,
                    );
                    let _ = tray_for_tooltip.set_tooltip(Some(tooltip));
                }
            });

            // --- Window close behavior: Windows exits, macOS hides to tray ---
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        #[cfg(target_os = "macos")]
                        {
                            // macOS: Hide to tray for better UX
                            api.prevent_close();
                            hide_main_window(&app_handle);
                        }

                        #[cfg(not(target_os = "macos"))]
                        let _ = &api; // suppress unused warning on non-macOS

                        #[cfg(target_os = "windows")]
                        {
                            // Windows: Actually exit the application
                            // This stops all background threads and processes
                            app_handle.exit(0);
                        }

                        #[cfg(target_os = "linux")]
                        {
                            // Linux: Exit like Windows
                            app_handle.exit(0);
                        }
                    }
                });
            }

            // --- Global Hotkey: Ctrl+Alt+O (Win/Linux) / Cmd+Option+O (macOS) ---
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
                let app_handle = app.handle().clone();
                let shortcut = "CommandOrControl+Alt+O".parse::<Shortcut>();

                match shortcut {
                    Ok(s) => {
                        let register_result =
                            app.global_shortcut()
                                .on_shortcut(s, move |_app, _shortcut, _event| {
                                    toggle_main_window(&app_handle);
                                });

                        if let Err(e) = register_result {
                            tracing::error!("Failed to register global hotkey: {}", e);
                        } else {
                            #[cfg(target_os = "macos")]
                            tracing::info!("Global hotkey Cmd+Option+O registered successfully");
                            #[cfg(not(target_os = "macos"))]
                            tracing::info!("Global hotkey Ctrl+Alt+O registered successfully");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse global hotkey shortcut: {}", e);
                    }
                }
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
            check_cdp_availability,
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
            zombie_killer::get_zombie_killer_config,
            zombie_killer::set_zombie_killer_config,
            zombie_killer::list_zombie_candidates,
            zombie_killer::kill_zombie,
            zombie_killer::kill_all_zombies,
            get_ai_daily_usage,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[omnimon] fatal: tauri application failed to start: {e}");
            std::process::exit(1);
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kill_processes_with_rejects_batches_above_limit() {
        let err = kill_processes_with(vec![42; MAX_KILL_BATCH + 1], |_| {
            panic!("kill callback must not run when batch is rejected")
        })
        .unwrap_err();

        assert_eq!(err, format!("error_batch_limit:{MAX_KILL_BATCH}"));
    }

    #[test]
    fn kill_processes_with_collects_success_and_failure_per_pid() {
        let result = kill_processes_with(vec![101, 202, 303], |pid| match pid {
            202 => Err("permission_denied".to_string()),
            _ => Ok(()),
        })
        .unwrap();

        assert_eq!(result.killed, vec![101, 303]);
        assert_eq!(result.failed, vec![(202, "permission_denied".to_string())]);
    }

    #[test]
    fn save_ai_config_with_rejects_empty_key_after_trim() {
        let mut called = false;
        let err = save_ai_config_with("openai", "   ", |_, _| {
            called = true;
            Ok(())
        })
        .unwrap_err();

        assert_eq!(err, "error_api_key_empty");
        assert!(!called);
    }

    #[test]
    fn save_ai_config_with_trims_key_before_persisting() {
        let mut captured: Option<(macmon_core::ai::AiProvider, String)> = None;

        save_ai_config_with("openai", "  sk-test-value  ", |provider, key| {
            captured = Some((provider, key.to_string()));
            Ok(())
        })
        .unwrap();

        let (provider, key) = captured.expect("expected saved key capture");
        assert_eq!(provider, macmon_core::ai::AiProvider::OpenAI);
        assert_eq!(key, "sk-test-value");
    }

    #[test]
    fn save_ai_config_with_rejects_unknown_provider() {
        let err = save_ai_config_with("unknown-provider", "key", |_, _| Ok(())).unwrap_err();
        assert!(err.contains("Unknown AI provider"));
    }

    #[test]
    fn apply_ai_rules_with_rejects_payload_above_limit() {
        let payload = "x".repeat(MAX_AI_RULES_PAYLOAD_BYTES + 1);
        let err = apply_ai_rules_with(&payload, |_| Ok(0)).unwrap_err();

        assert_eq!(
            err,
            format!(
                "payload exceeds {}KB limit",
                MAX_AI_RULES_PAYLOAD_BYTES / 1024
            )
        );
    }

    #[test]
    fn apply_ai_rules_with_forwards_payload_to_rules_engine() {
        let payload = r#"{"schema_version":1,"rules":[]}"#;
        let mut seen_payload = String::new();

        let count = apply_ai_rules_with(payload, |body| {
            seen_payload = body.to_string();
            Ok(7)
        })
        .unwrap();

        assert_eq!(count, 7);
        assert_eq!(seen_payload, payload);
    }

    fn network_rules_payload() -> String {
        json!([
            {
                "id": "bandwidth-spike",
                "name": "Bandwidth spike",
                "enabled": true,
                "condition": {
                    "kind": "high_bandwidth",
                    "threshold_mbps": 180.0,
                    "direction": "both",
                    "process": "chrome"
                },
                "severity": "warning",
                "cooldown_seconds": 30,
                "notify_ai": false
            }
        ])
        .to_string()
    }

    #[test]
    fn parse_network_alert_rules_payload_rejects_too_large_payload() {
        let payload = "x".repeat(MAX_NETWORK_ALERT_RULES_PAYLOAD_BYTES + 1);
        let err = parse_network_alert_rules_payload(&payload).unwrap_err();
        assert_eq!(err, "error_payload_too_large");
    }

    #[test]
    fn set_network_alert_rules_with_parses_and_applies_rules() {
        let payload = network_rules_payload();
        let mut applied: Option<Vec<macmon_core::network_alerts::NetworkAlertRule>> = None;

        let count = set_network_alert_rules_with(&payload, |rules| {
            applied = Some(rules);
        })
        .unwrap();

        let rules = applied.expect("expected applied rules");
        assert_eq!(count, 1);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "bandwidth-spike");
    }

    #[test]
    fn set_network_alert_rules_with_rejects_invalid_json_without_applying() {
        let mut applied = false;
        let err = set_network_alert_rules_with("{not-valid-json", |_| {
            applied = true;
        })
        .unwrap_err();

        assert!(err.starts_with("error_invalid_json:"));
        assert!(!applied);
    }
}
