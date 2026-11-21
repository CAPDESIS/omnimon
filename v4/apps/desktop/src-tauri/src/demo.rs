//! Development-only native host for screenshots. No watcher, capture, AI,
//! automation, process control, keyring or real browser integration is started.
use serde_json::{json, Value};
use tauri::Emitter;
use tauri_plugin_store::StoreExt;

fn response(command: &str) -> Result<Value, String> {
    let data: Value =
        serde_json::from_str(include_str!("demo-data.json")).expect("valid demo fixture");
    match command {
        "get_metrics" => Ok(data["metrics"].clone()),
        "get_network_data" => Ok(data["network"].clone()),
        "get_zombie_killer_config" => Ok(data["zombie_config"].clone()),
        "list_zombie_candidates" => Ok(data["zombies"].clone()),
        "get_browser_tabs" | "list_plugins" => Ok(json!([])),
        "get_window_visible" => Ok(json!(true)),
        "check_api_key" => Ok(json!(false)),
        "set_network_alert_rules" => Ok(json!(0)),
        _ => Err("Unavailable in screenshot demo".into()),
    }
}

pub fn run(mut context: tauri::Context<tauri::Wry>) {
    context.config_mut().identifier = "com.omnimon.screenshot-demo".into();
    for window in &mut context.config_mut().app.windows {
        window.incognito = true;
        window.title = format!(
            "OmniMon {} | Demo (synthetic data)",
            env!("CARGO_PKG_VERSION")
        );
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let store = app.store("preferences.json")?;
            store.clear();
            store.set("localePreference", json!("en"));
            store.set("theme", json!("dark"));
            store.set("userMode", json!("pro"));
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if handle
                    .emit("metrics-update", response("get_metrics").unwrap())
                    .is_err()
                {
                    break;
                }
            });
            Ok(())
        })
        .invoke_handler(|invoke| {
            match response(invoke.message.command()) {
                Ok(value) => invoke.resolver.resolve(value),
                Err(error) => invoke.resolver.reject(error),
            }
            true
        })
        .run(context)
        .expect("failed to run screenshot demo");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_is_valid_and_network_capture_is_off() {
        let metrics = response("get_metrics").unwrap();
        assert_eq!(metrics["processes"].as_array().unwrap().len(), 22);
        assert_eq!(metrics["processes"][0]["name"], "Safari");
        for process in metrics["processes"].as_array().unwrap() {
            assert!(process["exe_path"].is_null());
        }
        let network = response("get_network_data").unwrap();
        assert_eq!(network["dpi_active"], false);
        for connection in network["recent_connections"].as_array().unwrap() {
            assert_eq!(connection["src_ip"], "192.0.2.10");
            assert!(["198.51.100.20", "203.0.113.30", "192.0.2.80"]
                .contains(&connection["dst_ip"].as_str().unwrap()));
        }
        assert_eq!(response("get_browser_tabs").unwrap(), json!([]));
    }
    #[test]
    fn commands_cannot_fall_through_to_live_operations() {
        for command in [
            "kill_process",
            "kill_zombie",
            "kill_all_zombies",
            "set_zombie_killer_config",
            "ai_chat",
            "save_cloud_key",
            "run_security_scan",
            "unknown",
        ] {
            assert!(response(command).is_err(), "{command}");
        }
    }
}
