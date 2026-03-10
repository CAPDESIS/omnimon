use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

const DEFAULT_AUTOMATION_INTERVAL_SECS: u64 = 5;

// --- Constants ---

/// Metric name for RAM usage (in MB).
const METRIC_RAM: &str = "ram";

/// Action: terminate the offending process.
const ACTION_KILL: &str = "kill";

/// Notification title used when an automation kills a process.
const NOTIFICATION_TITLE_KILLED: &str = "Automations Engine";

/// Notification title used for alert-only automations.
const NOTIFICATION_TITLE_ALERT: &str = "Automations Engine Alert";

/// Bytes in one mebibyte, used for memory conversion.
const BYTES_PER_MB: f64 = 1_048_576.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub process_pattern: String,
    pub metric: String, // "cpu" or "ram"
    pub threshold: f64, // percentage or MB
    pub duration_secs: u64,
    pub action: String, // "kill" or "alert"
}

static RULES: OnceLock<Arc<RwLock<Vec<AutomationRule>>>> = OnceLock::new();
static RULES_INITIALIZED: OnceLock<Arc<RwLock<bool>>> = OnceLock::new();

/// Acquire a write lock on an `RwLock`, recovering from poisoned state.
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| {
        eprintln!("[automations] RwLock poisoned (write), recovering: {e}");
        e.into_inner()
    })
}

/// Acquire a read lock on an `RwLock`, recovering from poisoned state.
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| {
        eprintln!("[automations] RwLock poisoned (read), recovering: {e}");
        e.into_inner()
    })
}

pub fn get_rules() -> Arc<RwLock<Vec<AutomationRule>>> {
    RULES
        .get_or_init(|| Arc::new(RwLock::new(Vec::new())))
        .clone()
}

fn save_rules(app: &AppHandle, rules: &[AutomationRule]) {
    match app.store("automations.json") {
        Ok(store) => {
            match serde_json::to_value(rules) {
                Ok(value) => store.set("rules", value),
                Err(e) => {
                    eprintln!("[automations] failed to serialize rules: {e}");
                    return;
                }
            }
            if let Err(e) = store.save() {
                eprintln!("[automations] failed to save store: {e}");
            }
        }
        Err(e) => {
            eprintln!("[automations] failed to open store 'automations.json': {e}");
        }
    }
}

pub fn add_rule(app: &AppHandle, rule: AutomationRule) {
    let arc = get_rules();
    let mut rules = write_lock_or_recover(&arc);
    rules.push(rule.clone());
    save_rules(app, &rules);
}

pub fn remove_rule(app: &AppHandle, id: &str) {
    let arc = get_rules();
    let mut rules = write_lock_or_recover(&arc);
    rules.retain(|r| r.id != id);
    save_rules(app, &rules);
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn get_automation_rules(app: AppHandle) -> Vec<AutomationRule> {
    let init_flag = RULES_INITIALIZED.get_or_init(|| Arc::new(RwLock::new(false)));
    let mut is_init = write_lock_or_recover(init_flag);
    if !*is_init {
        match app.store("automations.json") {
            Ok(store) => {
                if let Some(val) = store.get("rules") {
                    if let Ok(stored_rules) = serde_json::from_value::<Vec<AutomationRule>>(val) {
                        let arc = get_rules();
                        let mut rules = write_lock_or_recover(&arc);
                        *rules = stored_rules;
                    }
                }
            }
            Err(e) => {
                eprintln!("[automations] failed to open store for rule loading: {e}");
            }
        }
        *is_init = true;
    }

    read_lock_or_recover(&get_rules()).clone()
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn add_automation_rule(app: AppHandle, rule: AutomationRule) -> Result<(), String> {
    macmon_core::rate_limit::check_rate_limit(
        "add_automation_rule",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    let _ = get_automation_rules(app.clone()); // Ensure init
    add_rule(&app, rule);
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip_all)]
pub fn remove_automation_rule(app: AppHandle, id: String) -> Result<(), String> {
    macmon_core::rate_limit::check_rate_limit(
        "remove_automation_rule",
        &macmon_core::rate_limit::profiles::CONFIG,
    )?;
    let _ = get_automation_rules(app.clone()); // Ensure init
    remove_rule(&app, &id);
    Ok(())
}

pub fn start_engine(app: AppHandle) {
    let _ = get_automation_rules(app.clone()); // Pre-load rules

    std::thread::spawn(move || {
        let mut violations: HashMap<(String, u32), Instant> = HashMap::new();
        loop {
            let interval_secs = macmon_core::settings::read_settings()
                .automation_interval_secs
                .unwrap_or(DEFAULT_AUTOMATION_INTERVAL_SECS);
            std::thread::sleep(Duration::from_secs(interval_secs));
            let rules = read_lock_or_recover(&get_rules()).clone();
            if rules.is_empty() {
                continue;
            }

            let state = macmon_core::watcher::get_cached_state();
            let procs = &state.cached_process_info;

            let now = Instant::now();
            let mut new_violations = HashMap::new();

            for rule in rules {
                for proc in procs {
                    if proc.name.contains(&rule.process_pattern)
                        || proc.exec_name.contains(&rule.process_pattern)
                    {
                        let value = if rule.metric == METRIC_RAM {
                            (proc.memory_bytes as f64) / BYTES_PER_MB
                        } else {
                            proc.cpu_pct as f64
                        };

                        if value > rule.threshold {
                            let key = (rule.id.clone(), proc.pid);
                            let first_seen = violations.get(&key).copied().unwrap_or(now);
                            new_violations.insert(key.clone(), first_seen);

                            if now.duration_since(first_seen).as_secs() >= rule.duration_secs {
                                // Action time!
                                if rule.action == ACTION_KILL {
                                    if macmon_core::killer::kill_process_safe(proc.pid as i32, &[])
                                        .is_ok()
                                    {
                                        let _ = app
                                            .notification()
                                            .builder()
                                            .title(NOTIFICATION_TITLE_KILLED)
                                            .body(format!(
                                                "Killed {} (PID {}) for exceeding {} {}",
                                                proc.name, proc.pid, rule.threshold, rule.metric
                                            ))
                                            .show();
                                    }
                                } else {
                                    let _ = app
                                        .notification()
                                        .builder()
                                        .title(NOTIFICATION_TITLE_ALERT)
                                        .body(format!(
                                            "Process {} (PID {}) exceeded {} {}",
                                            proc.name, proc.pid, rule.threshold, rule.metric
                                        ))
                                        .show();
                                }
                                // Reset violation to avoid spamming
                                new_violations.remove(&key);
                            }
                        }
                    }
                }
            }
            violations = new_violations;
        }
    });
}
