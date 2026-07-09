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

fn metric_label(metric: &str, locale: UiLocale) -> &'static str {
    match (locale, metric) {
        (_, METRIC_RAM) => "RAM",
        (UiLocale::Es, _) => "CPU",
        (UiLocale::En, _) => "CPU",
    }
}

fn notification_title(locale: UiLocale, is_kill: bool) -> &'static str {
    match (locale, is_kill) {
        (UiLocale::Es, true) => "Motor de automatizaciones",
        (UiLocale::En, true) => NOTIFICATION_TITLE_KILLED,
        (UiLocale::Es, false) => "Alerta del motor de automatizaciones",
        (UiLocale::En, false) => NOTIFICATION_TITLE_ALERT,
    }
}

fn notification_body(
    locale: UiLocale,
    is_kill: bool,
    process: &str,
    pid: u32,
    threshold: f64,
    metric: &str,
) -> String {
    let metric = metric_label(metric, locale);
    match (locale, is_kill) {
        (UiLocale::Es, true) => format!(
            "Se terminó {} (PID {}) por superar {} {}.",
            process, pid, threshold, metric
        ),
        (UiLocale::En, true) => format!(
            "Killed {} (PID {}) for exceeding {} {}.",
            process, pid, threshold, metric
        ),
        (UiLocale::Es, false) => format!(
            "El proceso {} (PID {}) superó {} {}.",
            process, pid, threshold, metric
        ),
        (UiLocale::En, false) => format!(
            "Process {} (PID {}) exceeded {} {}.",
            process, pid, threshold, metric
        ),
    }
}

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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    // --- AutomationRule serde ---

    fn make_rule(id: &str, process: &str, metric: &str, threshold: f64) -> AutomationRule {
        AutomationRule {
            id: id.to_string(),
            process_pattern: process.to_string(),
            metric: metric.to_string(),
            threshold,
            duration_secs: 30,
            action: "alert".to_string(),
        }
    }

    #[test]
    fn rule_serializes_to_json() {
        let rule = make_rule("r1", "chrome", "cpu", 80.0);
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"id\":\"r1\""));
        assert!(json.contains("\"process_pattern\":\"chrome\""));
        assert!(json.contains("\"metric\":\"cpu\""));
        assert!(json.contains("\"threshold\":80.0"));
    }

    #[test]
    fn rule_deserializes_from_json() {
        let json = r#"{
            "id": "r2",
            "process_pattern": "node",
            "metric": "ram",
            "threshold": 2048.0,
            "duration_secs": 60,
            "action": "kill"
        }"#;
        let rule: AutomationRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.id, "r2");
        assert_eq!(rule.process_pattern, "node");
        assert_eq!(rule.metric, "ram");
        assert_eq!(rule.threshold, 2048.0);
        assert_eq!(rule.duration_secs, 60);
        assert_eq!(rule.action, "kill");
    }

    #[test]
    fn rule_roundtrip_vec() {
        let rules = vec![
            make_rule("a", "chrome", "cpu", 50.0),
            make_rule("b", "node", "ram", 1024.0),
        ];
        let json = serde_json::to_string(&rules).unwrap();
        let restored: Vec<AutomationRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].id, "a");
        assert_eq!(restored[1].id, "b");
    }

    // --- get_rules ---

    #[test]
    fn get_rules_returns_shared_lock() {
        let arc1 = get_rules();
        let arc2 = get_rules();
        // Both references point to the same lock
        assert!(std::sync::Arc::ptr_eq(&arc1, &arc2));
    }

    // --- lock recovery ---

    #[test]
    fn write_lock_recovers_from_poison() {
        let lock = RwLock::new(42u32);
        // Poison the lock by panicking while holding a write guard
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("intentional panic to poison lock");
        }));
        // Should recover without panic
        let guard = write_lock_or_recover(&lock);
        assert_eq!(*guard, 42);
    }

    #[test]
    fn read_lock_recovers_from_poison() {
        let lock = RwLock::new(99u32);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = lock.write().unwrap();
            panic!("intentional panic to poison lock");
        }));
        let guard = read_lock_or_recover(&lock);
        assert_eq!(*guard, 99);
    }

    // --- constants ---

    #[test]
    fn bytes_per_mb_is_correct() {
        assert_eq!(BYTES_PER_MB, 1_048_576.0);
    }

    #[test]
    fn default_interval_is_5_seconds() {
        assert_eq!(DEFAULT_AUTOMATION_INTERVAL_SECS, 5);
    }

    // --- rule matching logic (isolated) ---

    #[test]
    fn process_pattern_matches_name_substring() {
        let rule = make_rule("test", "Chrome", "cpu", 50.0);
        let name = "Google Chrome Helper";
        assert!(name.contains(&rule.process_pattern));
    }

    #[test]
    fn metric_ram_computes_mb_correctly() {
        let memory_bytes: u64 = 2_147_483_648; // 2 GB
        let mb = (memory_bytes as f64) / BYTES_PER_MB;
        assert!((mb - 2048.0).abs() < 0.01);
    }

    #[test]
    fn threshold_comparison_works() {
        let rule = make_rule("test", "node", "ram", 1024.0);
        let value_above = 1500.0;
        let value_below = 512.0;
        assert!(value_above > rule.threshold);
        assert!(value_below <= rule.threshold);
    }

    #[test]
    fn detect_system_locale_returns_valid_variant() {
        assert!(matches!(
            detect_system_locale(),
            UiLocale::En | UiLocale::Es
        ));
    }

    #[test]
    fn metric_label_and_notification_copy() {
        assert_eq!(metric_label(METRIC_RAM, UiLocale::En), "RAM");
        assert_eq!(metric_label("cpu", UiLocale::Es), "CPU");
        assert!(!notification_title(UiLocale::Es, true).is_empty());
        assert!(!notification_title(UiLocale::En, false).is_empty());
        let body = notification_body(UiLocale::En, true, "node", 42, 90.0, "cpu");
        assert!(body.contains("node"));
        assert!(body.contains("42"));
        let body_es = notification_body(UiLocale::Es, false, "chrome", 7, 80.0, METRIC_RAM);
        assert!(body_es.contains("chrome"));
        assert!(body_es.contains("RAM"));
    }

    #[test]
    fn add_and_remove_rule_mutates_shared_store() {
        // Use unique ids so parallel/serial runs do not collide with leftover state.
        let id = format!("cov-rule-{}", std::process::id());
        let rule = make_rule(&id, "coverage-proc", "cpu", 99.0);
        {
            let arc = get_rules();
            let mut rules = write_lock_or_recover(&arc);
            rules.retain(|r| r.id != id);
            rules.push(rule.clone());
        }
        {
            let arc = get_rules();
            let rules = read_lock_or_recover(&arc);
            assert!(rules.iter().any(|r| r.id == id));
        }
        {
            let arc = get_rules();
            let mut rules = write_lock_or_recover(&arc);
            rules.retain(|r| r.id != id);
        }
        let arc = get_rules();
        let rules = read_lock_or_recover(&arc);
        assert!(!rules.iter().any(|r| r.id == id));
    }

    #[test]
    fn evaluate_rule_hits_matches_cpu_and_ram() {
        let rules = vec![
            make_rule("cpu-rule", "node", "cpu", 10.0),
            make_rule("ram-rule", "chrome", METRIC_RAM, 100.0),
        ];
        let procs = vec![
            macmon_core::watcher::CachedProcessInfo {
                pid: 1,
                name: "node helper".into(),
                exec_name: "node".into(),
                cpu_pct: 55.0,
                memory_bytes: 10 * 1024 * 1024,
                ..Default::default()
            },
            macmon_core::watcher::CachedProcessInfo {
                pid: 2,
                name: "Google Chrome".into(),
                exec_name: "chrome".into(),
                cpu_pct: 1.0,
                memory_bytes: 512 * 1024 * 1024,
                ..Default::default()
            },
            macmon_core::watcher::CachedProcessInfo {
                pid: 3,
                name: "safe".into(),
                exec_name: "safe".into(),
                cpu_pct: 1.0,
                memory_bytes: 1,
                ..Default::default()
            },
        ];
        let hits = evaluate_rule_hits(&rules, &procs);
        assert_eq!(hits.len(), 2);
        assert!(hits
            .iter()
            .any(|(r, pid, _, _)| r.id == "cpu-rule" && *pid == 1));
        assert!(hits
            .iter()
            .any(|(r, pid, _, _)| r.id == "ram-rule" && *pid == 2));
    }
}

/// Pure evaluation of which (rule_id, pid) pairs currently exceed thresholds.
fn evaluate_rule_hits(
    rules: &[AutomationRule],
    procs: &[macmon_core::watcher::CachedProcessInfo],
) -> Vec<(AutomationRule, u32, String, f64)> {
    let mut hits = Vec::new();
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
                    hits.push((rule.clone(), proc.pid, proc.name.clone(), value));
                }
            }
        }
    }
    hits
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

            for (rule, pid, proc_name, _value) in evaluate_rule_hits(&rules, procs) {
                let key = (rule.id.clone(), pid);
                let first_seen = violations.get(&key).copied().unwrap_or(now);
                new_violations.insert(key.clone(), first_seen);

                if now.duration_since(first_seen).as_secs() >= rule.duration_secs {
                    if rule.action == ACTION_KILL {
                        if macmon_core::killer::kill_process_safe(pid as i32, &[]).is_ok() {
                            let locale = read_ui_locale(&app);
                            let _ = app
                                .notification()
                                .builder()
                                .title(notification_title(locale, true))
                                .body(notification_body(
                                    locale,
                                    true,
                                    &proc_name,
                                    pid,
                                    rule.threshold,
                                    &rule.metric,
                                ))
                                .show();
                        }
                    } else {
                        let locale = read_ui_locale(&app);
                        let _ = app
                            .notification()
                            .builder()
                            .title(notification_title(locale, false))
                            .body(notification_body(
                                locale,
                                false,
                                &proc_name,
                                pid,
                                rule.threshold,
                                &rule.metric,
                            ))
                            .show();
                    }
                    new_violations.remove(&key);
                }
            }
            violations = new_violations;
        }
    });
}
