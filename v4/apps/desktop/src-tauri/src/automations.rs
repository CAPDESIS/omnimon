use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

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

pub fn get_rules() -> Arc<RwLock<Vec<AutomationRule>>> {
    RULES
        .get_or_init(|| Arc::new(RwLock::new(Vec::new())))
        .clone()
}

pub fn add_rule(rule: AutomationRule) {
    let arc = get_rules();
    let mut rules = arc.write().unwrap();
    rules.push(rule);
}

pub fn remove_rule(id: &str) {
    let arc = get_rules();
    let mut rules = arc.write().unwrap();
    rules.retain(|r| r.id != id);
}

use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[tauri::command]
pub fn get_automation_rules() -> Vec<AutomationRule> {
    get_rules().read().unwrap().clone()
}

#[tauri::command]
pub fn add_automation_rule(rule: AutomationRule) {
    add_rule(rule);
}

#[tauri::command]
pub fn remove_automation_rule(id: String) {
    remove_rule(&id);
}

pub fn start_engine(app: AppHandle) {
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
                                            .body(&format!(
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
                                        .body(&format!(
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
