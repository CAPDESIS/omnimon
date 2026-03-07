//! System telemetry and metrics collection. Gathers real-time data on CPU, memory, swap, and identifies the top resource-consuming processes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::System;

/// A single process entry with its PID, name, and memory usage in bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryEntry {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTelemetry {
    pub pid: u32,
    pub name: String,
    pub bundle_id: Option<String>,
    pub exe_path: Option<String>,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub cpu_usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperProcess {
    pub binary_key: String,
    pub identity_type: String,
    pub display_name: String,
    pub bundle_id: Option<String>,
    pub executable_path: Option<String>,
    pub process_count: usize,
    pub pids: Vec<u32>,
    pub total_memory_bytes: u64,
    pub total_virtual_memory_bytes: u64,
    pub total_disk_read_bytes: u64,
    pub total_disk_write_bytes: u64,
    pub total_cpu_usage_percent: f32,
    pub total_net_rx_bytes_per_sec: u64,
    pub total_net_tx_bytes_per_sec: u64,
}

/// Snapshot of system-wide memory: total, free, and used bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMemory {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
}

/// Returns the top `limit` processes sorted by memory usage in descending order.
///
/// Reads from the watcher cache when available, falling back to a fresh
/// `System` scan only if the watcher has not been started yet.
pub fn top_processes_by_memory(limit: usize) -> Vec<ProcessMemoryEntry> {
    let state = crate::watcher::get_cached_state();
    if !state.cached_process_info.is_empty() {
        let mut refs: Vec<&crate::watcher::CachedProcessInfo> =
            state.cached_process_info.iter().collect();
        refs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
        return refs
            .into_iter()
            .take(limit)
            .map(|p| ProcessMemoryEntry {
                pid: p.pid,
                name: p.name.clone(),
                memory_bytes: p.memory_bytes,
            })
            .collect();
    }

    // Fallback: watcher not started (CLI cold-start, tests)
    let mut system = System::new_all();
    system.refresh_all();

    let mut entries: Vec<ProcessMemoryEntry> = system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessMemoryEntry {
            pid: pid.as_u32(),
            name: process.name().to_string(),
            memory_bytes: process.memory(),
        })
        .collect();

    entries.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    entries.truncate(limit);
    entries
}

pub async fn top_processes_by_memory_async(limit: usize) -> Vec<ProcessMemoryEntry> {
    tokio::task::spawn_blocking(move || top_processes_by_memory(limit))
        .await
        .unwrap_or_default()
}

pub fn snapshot_process_telemetry() -> Vec<ProcessTelemetry> {
    let mut system = System::new_all();
    system.refresh_all();

    system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let disk = process.disk_usage();
            ProcessTelemetry {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                bundle_id: detect_bundle_id(process.exe()),
                exe_path: process.exe().map(|p| p.display().to_string()),
                memory_bytes: process.memory(),
                virtual_memory_bytes: process.virtual_memory(),
                disk_read_bytes: disk.total_read_bytes,
                disk_write_bytes: disk.total_written_bytes,
                cpu_usage_percent: process.cpu_usage(),
            }
        })
        .collect()
}

pub fn aggregate_super_processes(limit: Option<usize>) -> Vec<SuperProcess> {
    aggregate_super_processes_with_network(&[], limit)
}

pub fn aggregate_super_processes_with_network(
    network: &[crate::network::ProcessNetworkThroughput],
    limit: Option<usize>,
) -> Vec<SuperProcess> {
    let mut grouped: HashMap<String, SuperProcess> = HashMap::new();
    let mut pid_to_network: HashMap<u32, (u64, u64)> = HashMap::new();
    for item in network {
        pid_to_network.insert(item.pid, (item.rx_bytes_per_sec, item.tx_bytes_per_sec));
    }

    for process in snapshot_process_telemetry() {
        let group_identity = build_super_process_identity(&process);
        let group_key = group_identity.key;

        let entry = grouped
            .entry(group_key.clone())
            .or_insert_with(|| SuperProcess {
                binary_key: group_key,
                identity_type: group_identity.identity_type,
                display_name: process.name.clone(),
                bundle_id: process.bundle_id.clone(),
                executable_path: process.exe_path.clone(),
                process_count: 0,
                pids: Vec::new(),
                total_memory_bytes: 0,
                total_virtual_memory_bytes: 0,
                total_disk_read_bytes: 0,
                total_disk_write_bytes: 0,
                total_cpu_usage_percent: 0.0,
                total_net_rx_bytes_per_sec: 0,
                total_net_tx_bytes_per_sec: 0,
            });

        entry.process_count = entry.process_count.saturating_add(1);
        entry.pids.push(process.pid);
        entry.total_memory_bytes = entry
            .total_memory_bytes
            .saturating_add(process.memory_bytes);
        entry.total_virtual_memory_bytes = entry
            .total_virtual_memory_bytes
            .saturating_add(process.virtual_memory_bytes);
        entry.total_disk_read_bytes = entry
            .total_disk_read_bytes
            .saturating_add(process.disk_read_bytes);
        entry.total_disk_write_bytes = entry
            .total_disk_write_bytes
            .saturating_add(process.disk_write_bytes);
        entry.total_cpu_usage_percent += process.cpu_usage_percent;
        let (rx, tx) = pid_to_network.get(&process.pid).copied().unwrap_or((0, 0));
        entry.total_net_rx_bytes_per_sec = entry.total_net_rx_bytes_per_sec.saturating_add(rx);
        entry.total_net_tx_bytes_per_sec = entry.total_net_tx_bytes_per_sec.saturating_add(tx);
    }

    let mut super_processes: Vec<SuperProcess> = grouped.into_values().collect();
    super_processes.sort_by(|a, b| b.total_memory_bytes.cmp(&a.total_memory_bytes));

    if let Some(max) = limit {
        super_processes.truncate(max);
    }

    super_processes
}

struct SuperProcessIdentity {
    key: String,
    identity_type: String,
}

fn build_super_process_identity(process: &ProcessTelemetry) -> SuperProcessIdentity {
    if let Some(bundle) = process.bundle_id.as_ref() {
        return SuperProcessIdentity {
            key: format!("bundle:{bundle}"),
            identity_type: "bundle_id".to_string(),
        };
    }

    if let Some(path) = process.exe_path.as_ref() {
        return SuperProcessIdentity {
            key: format!("path:{}", path.to_ascii_lowercase()),
            identity_type: "path".to_string(),
        };
    }

    SuperProcessIdentity {
        key: format!(
            "name:{}:pid:{}",
            process.name.to_ascii_lowercase(),
            process.pid
        ),
        identity_type: "name_pid_fallback".to_string(),
    }
}

#[cfg(target_os = "macos")]
fn detect_bundle_id(exe: Option<&std::path::Path>) -> Option<String> {
    let exe = exe?;
    let path = exe.to_string_lossy();
    let marker = ".app/";
    let pos = path.find(marker)?;
    let bundle_root = &path[..pos + 4];
    Some(bundle_root.to_ascii_lowercase())
}

#[cfg(not(target_os = "macos"))]
fn detect_bundle_id(_exe: Option<&std::path::Path>) -> Option<String> {
    None
}

pub async fn aggregate_super_processes_async(limit: Option<usize>) -> Vec<SuperProcess> {
    tokio::task::spawn_blocking(move || aggregate_super_processes(limit))
        .await
        .unwrap_or_default()
}

pub async fn aggregate_super_processes_from_watcher_async(
    limit: Option<usize>,
) -> Vec<SuperProcess> {
    let state = crate::watcher::get_cached_state();
    let network = state.top_network_processes;
    tokio::task::spawn_blocking(move || aggregate_super_processes_with_network(&network, limit))
        .await
        .unwrap_or_default()
}

/// Collects a snapshot of system memory.
///
/// Reads from the watcher cache when available, falling back to native OS
/// APIs or sysinfo only if the watcher has not been started yet.
pub fn free_system_memory() -> SystemMemory {
    let state = crate::watcher::get_cached_state();
    if state.total_memory_bytes > 0 {
        return SystemMemory {
            total_memory_bytes: state.total_memory_bytes,
            free_memory_bytes: state.free_memory_bytes,
            used_memory_bytes: state.used_memory_bytes,
        };
    }

    // Fallback: watcher not started
    if let Some(native) = crate::os_native::collect_native_memory_snapshot() {
        return SystemMemory {
            total_memory_bytes: native.total_memory_bytes,
            free_memory_bytes: native.free_memory_bytes,
            used_memory_bytes: native.used_memory_bytes,
        };
    }

    let mut system = System::new_all();
    system.refresh_memory();

    let total_memory_bytes = system.total_memory();
    let free_memory_bytes = system.available_memory();
    let used_memory_bytes = total_memory_bytes.saturating_sub(free_memory_bytes);

    SystemMemory {
        total_memory_bytes,
        free_memory_bytes,
        used_memory_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn top_processes_by_memory_is_sorted_desc() {
        let top = top_processes_by_memory(25);
        for pair in top.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            assert!(
                left.memory_bytes >= right.memory_bytes,
                "process list is not sorted desc: {} < {}",
                left.memory_bytes,
                right.memory_bytes
            );
        }
    }

    #[test]
    fn process_memory_entry_serializes_as_expected() {
        let entry = ProcessMemoryEntry {
            pid: 42,
            name: "chrome".to_string(),
            memory_bytes: 1_048_576,
        };

        let encoded = serde_json::to_value(&entry).expect("serialize ProcessMemoryEntry");
        let expected = json!({
            "pid": 42,
            "name": "chrome",
            "memory_bytes": 1_048_576
        });

        assert_eq!(encoded, expected);
    }

    #[test]
    fn free_system_memory_values_are_consistent() {
        let memory = free_system_memory();
        assert!(memory.total_memory_bytes >= memory.used_memory_bytes);
        assert!(memory.total_memory_bytes >= memory.free_memory_bytes);
    }

    #[test]
    fn process_telemetry_contains_ids() {
        let telemetry = snapshot_process_telemetry();
        assert!(telemetry.iter().all(|p| p.pid > 0));
    }

    #[test]
    fn super_processes_are_aggregated() {
        let grouped = aggregate_super_processes(Some(50));
        assert!(grouped.len() <= 50);
        for super_process in grouped {
            assert!(super_process.process_count >= 1);
            assert_eq!(super_process.pids.len(), super_process.process_count);
        }
    }

    #[test]
    fn super_processes_absorb_network_throughput() {
        let network = vec![crate::network::ProcessNetworkThroughput {
            pid: std::process::id(),
            rx_bytes_per_sec: 1024,
            tx_bytes_per_sec: 2048,
            tcp_packets_per_sec: 2,
            udp_packets_per_sec: 1,
        }];

        let grouped = aggregate_super_processes_with_network(&network, Some(200));
        assert!(!grouped.is_empty());
    }

    #[test]
    fn identity_prefers_path_or_bundle_over_name() {
        let process = ProcessTelemetry {
            pid: 900,
            name: "svchost.exe".to_string(),
            bundle_id: None,
            exe_path: Some("C:/Users/bad/svchost.exe".to_string()),
            memory_bytes: 1,
            virtual_memory_bytes: 1,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            cpu_usage_percent: 0.0,
        };

        let identity = build_super_process_identity(&process);
        assert_eq!(identity.identity_type, "path");
        assert!(identity.key.starts_with("path:"));
    }
}
