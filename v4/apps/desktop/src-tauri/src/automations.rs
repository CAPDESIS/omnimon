use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

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

pub fn get_rules() -> Arc<RwLock<Vec<AutomationRule>>> {
    RULES
        .get_or_init(|| Arc::new(RwLock::new(Vec::new())))
        .clone()
}

fn save_rules(app: &AppHandle, rules: &[AutomationRule]) {
    if let Ok(store) = app.store("automations.json") {
        store.set("rules", serde_json::to_value(rules).unwrap());
        let _ = store.save();
    }
}

pub fn add_rule(app: &AppHandle, rule: AutomationRule) {
    let arc = get_rules();
    let mut rules = arc.write().unwrap();
    rules.push(rule.clone());
    save_rules(app, &rules);
}

pub fn remove_rule(app: &AppHandle, id: &str) {
    let arc = get_rules();
    let mut rules = arc.write().unwrap();
    rules.retain(|r| r.id != id);
    save_rules(app, &rules);
}

#[tauri::command]
pub fn get_automation_rules(app: AppHandle) -> Vec<AutomationRule> {
    let init_flag = RULES_INITIALIZED.get_or_init(|| Arc::new(RwLock::new(false)));
    let mut is_init = init_flag.write().unwrap();
    if !*is_init {
        if let Ok(store) = app.store("automations.json") {
            if let Some(val) = store.get("rules") {
                if let Ok(stored_rules) = serde_json::from_value::<Vec<AutomationRule>>(val) {
                    let arc = get_rules();
                    let mut rules = arc.write().unwrap();
                    *rules = stored_rules;
                }
            }
        }
        *is_init = true;
    }

    get_rules().read().unwrap().clone()
}

#[tauri::command]
pub fn add_automation_rule(app: AppHandle, rule: AutomationRule) {
    let _ = get_automation_rules(app.clone()); // Ensure init
    add_rule(&app, rule);
}

#[tauri::command]
pub fn remove_automation_rule(app: AppHandle, id: String) {
    let _ = get_automation_rules(app.clone()); // Ensure init
    remove_rule(&app, &id);
}

pub fn start_engine(app: AppHandle) {
    let _ = get_automation_rules(app.clone()); // Pre-load rules

    std::thread::spawn(move || {
        let mut violations: HashMap<(String, u32), Instant> = HashMap::new();
        loop {
            std::thread::sleep(Duration::from_secs(5));
            let rules = get_rules().read().unwrap().clone();
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
                        let value = if rule.metric == "ram" {
                            (proc.memory_bytes as f64) / 1_048_576.0
                        } else {
                            proc.cpu_pct as f64
                        };

                        if value > rule.threshold {
                            let key = (rule.id.clone(), proc.pid);
                            let first_seen = violations.get(&key).copied().unwrap_or(now);
                            new_violations.insert(key.clone(), first_seen);

                            if now.duration_since(first_seen).as_secs() >= rule.duration_secs {
                                // Action time!
                                if rule.action == "kill" {
                                    if macmon_core::killer::kill_process_safe(proc.pid as i32, &[])
                                        .is_ok()
                                    {
                                        let _ = app
                                            .notification()
                                            .builder()
                                            .title("Automations Engine")
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
                                        .title("Automations Engine Alert")
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
