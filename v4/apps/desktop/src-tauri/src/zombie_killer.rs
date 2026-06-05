//! Stateful engine around `macmon_core::zombie_killer`. Tracks how long
//! each candidate has been "hot", promotes to confirmed only after
//! `sustained_secs`, and exposes IPC commands for the UI.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

use macmon_core::zombie_killer::{self, ZombieCandidate, ZombieKillerConfig};

const STORE_FILE: &str = "zombie_killer.json";
const CONFIG_KEY: &str = "config";
const TICK_INTERVAL_SECS: u64 = 10;
const EVENT_ZOMBIES_UPDATED: &str = "zombie-killer-update";
/// Upper bound on notifications emitted in a single tick. Protects against
/// UI spam if many processes cross the threshold at once.
const MAX_NOTIFICATIONS_PER_TICK: usize = 5;
// Compile-time invariants for the throttle. Prevents a regression that would
// either silently disable notifications (0) or flood the user (very large).
const _: () = assert!(MAX_NOTIFICATIONS_PER_TICK > 0);
const _: () = assert!(MAX_NOTIFICATIONS_PER_TICK <= 10);

static CONFIG: OnceLock<Arc<RwLock<ZombieKillerConfig>>> = OnceLock::new();
static CURRENT_ZOMBIES: OnceLock<Arc<RwLock<Vec<ZombieCandidate>>>> = OnceLock::new();
static CONFIG_LOADED: OnceLock<Arc<RwLock<bool>>> = OnceLock::new();
/// Reentry guard: calling `start_engine` twice (e.g. on setup re-run) is a no-op.
static ENGINE_STARTED: AtomicBool = AtomicBool::new(false);

/// Key for per-process tracking that survives PID reuse. Pairs the PID with
/// the process's start_time — a recycled PID belonging to a different process
/// has a different start_time, so it gets a fresh entry instead of inheriting
/// the dead process's "first seen" timestamp.
type ProcessKey = (u32, u64);

fn config_handle() -> Arc<RwLock<ZombieKillerConfig>> {
    CONFIG
        .get_or_init(|| Arc::new(RwLock::new(ZombieKillerConfig::default())))
        .clone()
}

fn zombies_handle() -> Arc<RwLock<Vec<ZombieCandidate>>> {
    CURRENT_ZOMBIES
        .get_or_init(|| Arc::new(RwLock::new(Vec::new())))
        .clone()
}

fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| {
        eprintln!("[zombie_killer] RwLock poisoned (write), recovering: {e}");
        e.into_inner()
    })
}

fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| {
        eprintln!("[zombie_killer] RwLock poisoned (read), recovering: {e}");
        e.into_inner()
    })
}

fn load_config_once(app: &AppHandle) {
    let flag = CONFIG_LOADED.get_or_init(|| Arc::new(RwLock::new(false)));
    let mut loaded = write_lock_or_recover(flag);
    if *loaded {
        return;
    }
    match app.store(STORE_FILE) {
        Ok(store) => {
            if let Some(value) = store.get(CONFIG_KEY) {
                match serde_json::from_value::<ZombieKillerConfig>(value) {
                    Ok(parsed) => {
                        let sanitized = zombie_killer::sanitize_config(parsed);
                        *write_lock_or_recover(&config_handle()) = sanitized;
                    }
                    Err(e) => eprintln!("[zombie_killer] stored config parse failed: {e}"),
                }
            }
        }
        Err(e) => eprintln!("[zombie_killer] store open failed on load: {e}"),
    }
    *loaded = true;
}

fn save_config(app: &AppHandle, config: &ZombieKillerConfig) {
    match app.store(STORE_FILE) {
        Ok(store) => {
            match serde_json::to_value(config) {
                Ok(value) => store.set(CONFIG_KEY, value),
                Err(e) => {
                    eprintln!("[zombie_killer] serialize config failed: {e}");
                    return;
                }
            }
            if let Err(e) = store.save() {
                eprintln!("[zombie_killer] store save failed: {e}");
            }
        }
        Err(e) => eprintln!("[zombie_killer] store open failed on save: {e}"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiLocale {
    En,
    Es,
}

fn detect_system_locale() -> UiLocale {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            if v.to_ascii_lowercase().starts_with("es") {
                return UiLocale::Es;
            }
        }
    }
    UiLocale::En
}

fn read_ui_locale(app: &AppHandle) -> UiLocale {
    if let Ok(store) = app.store("preferences.json") {
        if let Some(v) = store.get("localePreference") {
            if let Some(s) = v.as_str() {
                return match s {
                    "es" => UiLocale::Es,
                    "en" => UiLocale::En,
                    _ => detect_system_locale(),
                };
            }
        }
    }
    detect_system_locale()
}

fn notification_title(locale: UiLocale, killed: bool) -> &'static str {
    match (locale, killed) {
        (UiLocale::Es, true) => "Zombie eliminado",
        (UiLocale::Es, false) => "Zombie detectado",
        (UiLocale::En, true) => "Zombie killed",
        (UiLocale::En, false) => "Zombie detected",
    }
}

fn notification_body(locale: UiLocale, killed: bool, name: &str, pid: u32) -> String {
    match (locale, killed) {
        (UiLocale::Es, true) => format!("Se terminó {} (PID {}).", name, pid),
        (UiLocale::Es, false) => {
            format!(
                "{} (PID {}) lleva mucho tiempo consumiendo recursos.",
                name, pid
            )
        }
        (UiLocale::En, true) => format!("Killed {} (PID {}).", name, pid),
        (UiLocale::En, false) => {
            format!(
                "{} (PID {}) has been consuming resources for a long time.",
                name, pid
            )
        }
    }
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn get_zombie_killer_config(app: AppHandle) -> ZombieKillerConfig {
    load_config_once(&app);
    read_lock_or_recover(&config_handle()).clone()
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn set_zombie_killer_config(app: AppHandle, config: ZombieKillerConfig) -> Result<(), String> {
    macmon_core::rate_limit::check_rate_limit(
        "set_zombie_killer_config",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    load_config_once(&app);
    let sanitized = zombie_killer::sanitize_config(config);
    *write_lock_or_recover(&config_handle()) = sanitized.clone();
    save_config(&app, &sanitized);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn list_zombie_candidates() -> Vec<ZombieCandidate> {
    read_lock_or_recover(&zombies_handle()).clone()
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn kill_zombie(pid: u32) -> Result<macmon_core::killer::KillResult, String> {
    macmon_core::rate_limit::check_rate_limit(
        "kill_zombie",
        &macmon_core::rate_limit::profiles::KILL,
    )?;
    let config_arc = config_handle();
    let never_kill = read_lock_or_recover(&config_arc).never_kill.clone();
    drop(config_arc);
    let result = macmon_core::killer::kill_process_safe(pid as i32, &never_kill)
        .map_err(|e| e.to_string())?;
    // Drop it from the current list so UI state stays in sync immediately.
    let zombies_arc = zombies_handle();
    let mut zombies = write_lock_or_recover(&zombies_arc);
    zombies.retain(|z| z.pid != pid);
    Ok(result)
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn kill_all_zombies() -> Result<Vec<macmon_core::killer::KillResult>, String> {
    macmon_core::rate_limit::check_rate_limit(
        "kill_all_zombies",
        &macmon_core::rate_limit::profiles::KILL,
    )?;
    let (zombies, never_kill) = {
        let zombies_arc = zombies_handle();
        let config_arc = config_handle();
        let z = read_lock_or_recover(&zombies_arc).clone();
        let nk = read_lock_or_recover(&config_arc).never_kill.clone();
        (z, nk)
    };
    let mut results = Vec::with_capacity(zombies.len());
    let mut killed_pids: HashSet<u32> = HashSet::new();
    for z in &zombies {
        match macmon_core::killer::kill_process_safe(z.pid as i32, &never_kill) {
            Ok(r) => {
                killed_pids.insert(r.pid);
                results.push(r);
            }
            Err(e) => eprintln!("[zombie_killer] kill_all: pid {} failed: {}", z.pid, e),
        }
    }
    if !killed_pids.is_empty() {
        let zombies_arc = zombies_handle();
        let mut current = write_lock_or_recover(&zombies_arc);
        current.retain(|z| !killed_pids.contains(&z.pid));
    }
    Ok(results)
}

/// Start the background engine. The thread ticks every [`TICK_INTERVAL_SECS`],
/// tracks how long each candidate has been continuously flagged (keyed by
/// `(pid, start_time)` so PID reuse never inherits a dead process's clock),
/// and promotes a process to a confirmed zombie once it has been hot
/// continuously for `config.sustained_secs`. Auto-kill fires only if
/// `config.auto_kill` is true.
///
/// Calling this more than once is a no-op.
pub fn start_engine(app: AppHandle) {
    if ENGINE_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let _ = get_zombie_killer_config(app.clone());

    std::thread::spawn(move || {
        // `first_seen[(pid, start_time)]` = unix seconds when the process first
        // crossed the thresholds. The value is tiny (u64) so we clone freely.
        let mut first_seen: HashMap<ProcessKey, u64> = HashMap::new();
        // Confirmed PIDs already notified. Pruned each tick to match `first_seen`
        // so it cannot grow unbounded.
        let mut notified: HashSet<ProcessKey> = HashSet::new();

        loop {
            std::thread::sleep(Duration::from_secs(TICK_INTERVAL_SECS));

            // Run the tick body under catch_unwind. If rules evaluation, a kill,
            // or a store write panics, we log and skip this cycle instead of
            // dying silently like the pre-fix version would have done.
            let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_tick(&app, &mut first_seen, &mut notified);
            }));
            if tick.is_err() {
                eprintln!("[zombie_killer] panic in engine tick — skipping this cycle");
            }
        }
    });
}

fn run_tick(
    app: &AppHandle,
    first_seen: &mut HashMap<ProcessKey, u64>,
    notified: &mut HashSet<ProcessKey>,
) {
    let config_arc = config_handle();
    let config = read_lock_or_recover(&config_arc).clone();
    drop(config_arc);

    if !config.enabled {
        first_seen.clear();
        notified.clear();
        let zombies_arc = zombies_handle();
        let mut zombies = write_lock_or_recover(&zombies_arc);
        if !zombies.is_empty() {
            zombies.clear();
            if let Err(e) = app.emit(EVENT_ZOMBIES_UPDATED, Vec::<ZombieCandidate>::new()) {
                eprintln!("[zombie_killer] emit clear failed: {e}");
            }
        }
        return;
    }

    let state = macmon_core::watcher::get_cached_state();
    let now = zombie_killer::now_unix_secs();
    let candidates = zombie_killer::identify_candidates(&state.cached_process_info, &config, now);

    let mut next_seen: HashMap<ProcessKey, u64> = HashMap::with_capacity(candidates.len());
    let mut confirmed: Vec<ZombieCandidate> = Vec::new();

    for cand in candidates {
        let key: ProcessKey = (cand.pid, cand.start_time);
        let start = *first_seen.get(&key).unwrap_or(&now);
        next_seen.insert(key, start);
        if now.saturating_sub(start) >= config.sustained_secs {
            confirmed.push(cand);
        }
    }
    *first_seen = next_seen;
    notified.retain(|key| first_seen.contains_key(key));

    let locale = read_ui_locale(app);
    let mut notifications_sent = 0usize;
    for z in &confirmed {
        let key: ProcessKey = (z.pid, z.start_time);
        if notified.contains(&key) {
            continue;
        }

        let is_kill = config.auto_kill;
        if is_kill {
            if let Err(e) = macmon_core::killer::kill_process_safe(z.pid as i32, &config.never_kill)
            {
                eprintln!("[zombie_killer] auto-kill failed for {}: {}", z.pid, e);
                // Leave it un-notified so we try again next tick.
                continue;
            }
        }

        if notifications_sent < MAX_NOTIFICATIONS_PER_TICK {
            if let Err(e) = app
                .notification()
                .builder()
                .title(notification_title(locale, is_kill))
                .body(notification_body(locale, is_kill, &z.name, z.pid))
                .show()
            {
                eprintln!("[zombie_killer] notification show failed: {e}");
            }
            notifications_sent += 1;
        }
        notified.insert(key);
    }

    {
        let zombies_arc = zombies_handle();
        let mut zombies = write_lock_or_recover(&zombies_arc);
        *zombies = confirmed.clone();
    }
    if let Err(e) = app.emit(EVENT_ZOMBIES_UPDATED, &confirmed) {
        eprintln!("[zombie_killer] emit update failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_title_switches_by_locale() {
        assert_eq!(notification_title(UiLocale::Es, true), "Zombie eliminado");
        assert_eq!(notification_title(UiLocale::En, true), "Zombie killed");
        assert_eq!(notification_title(UiLocale::Es, false), "Zombie detectado");
        assert_eq!(notification_title(UiLocale::En, false), "Zombie detected");
    }

    #[test]
    fn notification_body_mentions_name_and_pid() {
        let body = notification_body(UiLocale::Es, true, "Chrome", 1234);
        assert!(body.contains("Chrome"));
        assert!(body.contains("1234"));
    }

    #[test]
    fn config_handle_is_shared() {
        let a = config_handle();
        let b = config_handle();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn zombies_handle_is_shared() {
        let a = zombies_handle();
        let b = zombies_handle();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn write_lock_recovers_from_poison() {
        let lock = RwLock::new(7u32);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("poison");
        }));
        let guard = write_lock_or_recover(&lock);
        assert_eq!(*guard, 7);
    }

    #[test]
    fn engine_started_guard_is_set_after_swap() {
        // Reset so this test is independent of others.
        ENGINE_STARTED.store(false, Ordering::SeqCst);
        assert!(!ENGINE_STARTED.swap(true, Ordering::SeqCst));
        // Second swap returns true = already started = no-op guard fires.
        assert!(ENGINE_STARTED.swap(true, Ordering::SeqCst));
        ENGINE_STARTED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn process_key_differs_when_start_time_differs() {
        // Simulates PID reuse: same PID, different process (different start_time).
        let a: ProcessKey = (100, 1_700_000_000);
        let b: ProcessKey = (100, 1_700_000_500);
        assert_ne!(a, b);
    }
}
