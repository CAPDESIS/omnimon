//! Background monitoring daemon. Periodically aggregates system metrics, network flows, and dynamically evaluates AI-driven security rules.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, System};

static CACHED_STATE: OnceLock<Arc<RwLock<SystemState>>> = OnceLock::new();
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

/// Per-process info cached by the watcher thread (memory, CPU%, executable name, start time).
/// Avoids the need for IPC handlers to create `System` instances on the main thread.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CachedProcessInfo {
    pub pid: u32,
    pub name: String,
    pub group_name: String,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub cpu_pct: f32,
    pub exec_name: String,
    pub exe_path: Option<String>,
    pub bundle_id: Option<String>,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub energy_impact_score: Option<f32>,
    pub start_time: u64,
}

/// Periodically refreshed snapshot of system health: memory, CPU, swap, and network I/O.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub free_percent: u32,
    pub swap_used_mb: u64,
    pub cpu_usage_percent: f32,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub net_capture_backend: String,
    pub net_dpi_active: bool,
    pub top_network_processes: Vec<crate::network::ProcessNetworkThroughput>,
    pub recent_network_connections: Vec<crate::network::ProcessConnectionEvent>,
    pub mitre_network_alerts: Vec<crate::security::ProcessThreatLabel>,
    pub dynamic_rule_alerts: Vec<crate::rules_engine::DynamicAlert>,
    pub security_heartbeat: Option<crate::audit::SecurityHeartbeat>,
    pub cached_process_info: Vec<CachedProcessInfo>,
    pub updated_at_unix_ms: u128,
}

fn state_handle() -> Arc<RwLock<SystemState>> {
    Arc::clone(CACHED_STATE.get_or_init(|| Arc::new(RwLock::new(SystemState::default()))))
}

fn collect_state(system: &mut System) -> SystemState {
    system.refresh_memory();
    system.refresh_cpu();
    system.refresh_processes_specifics(ProcessRefreshKind::everything());

    let fallback_total = system.total_memory();
    let fallback_free = system.available_memory();
    let fallback_used = fallback_total.saturating_sub(fallback_free);
    let fallback_free_pct = if fallback_total > 0 {
        ((fallback_free as f64 / fallback_total as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32
    } else {
        0
    };
    let native = crate::os_native::collect_native_memory_snapshot();
    let total_memory_bytes = native
        .map(|m| m.total_memory_bytes)
        .unwrap_or(fallback_total);
    let free_memory_bytes = native.map(|m| m.free_memory_bytes).unwrap_or(fallback_free);
    let used_memory_bytes = native.map(|m| m.used_memory_bytes).unwrap_or(fallback_used);
    let free_percent = native.map(|m| m.free_percent).unwrap_or(fallback_free_pct);
    let swap_used_mb = native.map(|m| m.swap_used_mb).unwrap_or(0);
    let cpu_usage_percent = system.global_cpu_info().cpu_usage();
    let updated_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let cached_process_info: Vec<CachedProcessInfo> = system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let name = process.name().to_string();
            let exe_path = process.exe().map(|e| e.display().to_string());
            let exec_name = process
                .exe()
                .and_then(|e| e.file_name())
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.clone());
            let bundle_id = crate::process_identity::detect_bundle_id(process.exe());
            let disk = process.disk_usage();
            let is_system = crate::killer::is_immutable_blocked_process_name(&name);
            let group_name = crate::process_identity::classify_group(
                &name,
                &exec_name,
                exe_path.as_deref(),
                is_system,
            );
            CachedProcessInfo {
                pid: pid.as_u32(),
                name,
                group_name,
                memory_bytes: process.memory(),
                virtual_memory_bytes: process.virtual_memory(),
                cpu_pct: process.cpu_usage(),
                exec_name,
                exe_path,
                bundle_id,
                disk_read_bytes: disk.total_read_bytes,
                disk_write_bytes: disk.total_written_bytes,
                net_rx_bytes_per_sec: 0,
                net_tx_bytes_per_sec: 0,
                energy_impact_score: None,
                start_time: process.start_time(),
            }
        })
        .collect();

    SystemState {
        total_memory_bytes,
        free_memory_bytes,
        used_memory_bytes,
        free_percent,
        swap_used_mb,
        cpu_usage_percent,
        net_rx_bytes_per_sec: 0,
        net_tx_bytes_per_sec: 0,
        net_capture_backend: "Unknown".to_string(),
        net_dpi_active: false,
        top_network_processes: Vec::new(),
        recent_network_connections: Vec::new(),
        mitre_network_alerts: Vec::new(),
        dynamic_rule_alerts: Vec::new(),
        security_heartbeat: None,
        cached_process_info,
        updated_at_unix_ms,
    }
}

/// Spawns a background thread that refreshes the cached [`SystemState`] every 2 seconds.
///
/// Calling this more than once is a no-op.
pub fn start_watcher() {
    if WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let cache = state_handle();

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return,
        };

        runtime.block_on(async move {
            let mut system = System::new_all();
            let mut network_engine = crate::network::NetworkTelemetryEngine::new();

            let initial = collect_state(&mut system);
            if let Ok(mut guard) = cache.write() {
                *guard = initial;
            }

            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;

                // Wrap tick body in catch_unwind so a transient panic
                // (e.g. in network sampling or rules evaluation) doesn't
                // kill the watcher thread and freeze all metrics forever.
                let tick_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let sample = network_engine.sample();
                    let policy = crate::security::NetworkPolicy::default();
                    let network_observations = crate::security::evaluate_network_events(
                        &sample.recent_connections,
                        &policy,
                    );
                    let mitre_labels =
                        crate::security::label_process_observations(&network_observations);

                    let mut snapshot = collect_state(&mut system);

                    let runtime: Vec<crate::rules_engine::ProcessRuntime> = snapshot
                        .cached_process_info
                        .iter()
                        .map(|p| crate::rules_engine::ProcessRuntime {
                            pid: p.pid,
                            process_name: p.name.clone(),
                            memory_bytes: p.memory_bytes,
                        })
                        .collect();
                    let dynamic_alerts =
                        crate::rules_engine::evaluate_events(&sample.recent_connections, &runtime);
                    let heartbeat = crate::audit::build_security_heartbeat(
                        sample.process_throughput.len(),
                        0,
                        sample.deep_packet_inspection_active,
                        network_observations.len() + dynamic_alerts.len(),
                        mitre_labels.len() + dynamic_alerts.len(),
                        true,
                        "monitoring",
                    );

                    snapshot.net_rx_bytes_per_sec = sample.net_rx_bytes_per_sec;
                    snapshot.net_tx_bytes_per_sec = sample.net_tx_bytes_per_sec;
                    snapshot.net_capture_backend = sample.backend_label;
                    snapshot.net_dpi_active = sample.deep_packet_inspection_active;
                    let process_throughput = sample.process_throughput;
                    let mut throughput_by_pid = std::collections::HashMap::new();
                    for item in &process_throughput {
                        throughput_by_pid
                            .insert(item.pid, (item.rx_bytes_per_sec, item.tx_bytes_per_sec));
                    }
                    for process in &mut snapshot.cached_process_info {
                        let (rx, tx) = throughput_by_pid
                            .get(&process.pid)
                            .copied()
                            .unwrap_or((0, 0));
                        process.net_rx_bytes_per_sec = rx;
                        process.net_tx_bytes_per_sec = tx;
                        process.energy_impact_score = crate::metrics::estimate_energy_impact(
                            process.cpu_pct,
                            process.memory_bytes,
                            process.disk_read_bytes,
                            process.disk_write_bytes,
                            rx,
                            tx,
                        );
                    }
                    snapshot.top_network_processes = process_throughput;
                    snapshot.recent_network_connections = sample.recent_connections;
                    snapshot.mitre_network_alerts = mitre_labels;
                    snapshot.dynamic_rule_alerts = dynamic_alerts;
                    snapshot.security_heartbeat = Some(heartbeat);
                    snapshot
                }));

                match tick_result {
                    Ok(snapshot) => {
                        if let Ok(mut guard) = cache.write() {
                            *guard = snapshot;
                        }
                    }
                    Err(_) => {
                        eprintln!("[watcher] panic in monitoring tick — skipping this cycle");
                    }
                }
            }
        });
    });
}

/// Returns the most recent [`SystemState`] snapshot from the watcher.
///
/// If the watcher has not been started, returns a zeroed default state.
pub fn get_cached_state() -> SystemState {
    let cache = state_handle();
    cache
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_state_is_readable_without_starting_watcher() {
        let state = get_cached_state();
        assert_eq!(state.total_memory_bytes, 0);
        assert_eq!(state.used_memory_bytes, 0);
    }

    #[test]
    fn watcher_updates_cached_state() {
        start_watcher();
        std::thread::sleep(Duration::from_millis(2500));
        let state = get_cached_state();
        assert!(state.total_memory_bytes > 0);
        assert!(state.total_memory_bytes >= state.used_memory_bytes);
        assert!(state.total_memory_bytes >= state.free_memory_bytes);
        assert!(state.free_percent <= 100);
        assert!(state.updated_at_unix_ms > 0);
    }
}
