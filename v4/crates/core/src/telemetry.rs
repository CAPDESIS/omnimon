use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTelemetryView {
    pub pid: u32,
    pub name: String,
    pub exec_name: String,
    pub group: String,
    pub group_key: String,
    pub group_identity_type: String,
    pub grouped_display_name: String,
    pub process_count: usize,
    pub bundle_id: Option<String>,
    pub exe_path: Option<String>,
    pub icon_data_url: Option<String>,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub energy_impact_score: Option<f32>,
    pub cpu_usage_percent: f32,
    pub start_time: u64,
    pub is_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub free_percent: u32,
    pub swap_used_mb: u64,
    pub cpu_usage_percent: f32,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub total_processes: u32,
    pub processes: Vec<ProcessTelemetryView>,
    pub super_processes: Vec<crate::metrics::SuperProcess>,
}

pub fn telemetry_snapshot(limit: Option<usize>) -> TelemetrySnapshot {
    let state = crate::watcher::get_cached_state();
    let super_processes = super_processes_from_state(&state, limit);

    let mut groups_by_pid = HashMap::new();
    for super_process in &super_processes {
        for pid in &super_process.pids {
            groups_by_pid.insert(*pid, super_process.clone());
        }
    }

    let mut processes = state
        .cached_process_info
        .iter()
        .map(|entry| {
            let grouped = groups_by_pid.get(&entry.pid).cloned().unwrap_or_else(|| {
                let identity = crate::process_identity::resolve_group_identity(
                    &entry.name,
                    &entry.exec_name,
                    entry.exe_path.as_deref(),
                    entry.bundle_id.as_deref(),
                    entry.group_name == "System",
                );
                crate::metrics::SuperProcess {
                    binary_key: identity.key,
                    identity_type: identity.identity_type,
                    display_name: identity.display_name,
                    group: identity.group,
                    bundle_id: entry.bundle_id.clone(),
                    executable_path: entry.exe_path.clone(),
                    process_count: 1,
                    pids: vec![entry.pid],
                    total_memory_bytes: entry.memory_bytes,
                    total_virtual_memory_bytes: entry.virtual_memory_bytes,
                    total_disk_read_bytes: entry.disk_read_bytes,
                    total_disk_write_bytes: entry.disk_write_bytes,
                    total_cpu_usage_percent: entry.cpu_pct,
                    total_net_rx_bytes_per_sec: entry.net_rx_bytes_per_sec,
                    total_net_tx_bytes_per_sec: entry.net_tx_bytes_per_sec,
                    energy_impact_score: entry.energy_impact_score,
                }
            });

            ProcessTelemetryView {
                pid: entry.pid,
                name: entry.name.clone(),
                exec_name: entry.exec_name.clone(),
                group: grouped.group.clone(),
                group_key: grouped.binary_key.clone(),
                group_identity_type: grouped.identity_type.clone(),
                grouped_display_name: grouped.display_name.clone(),
                process_count: grouped.process_count,
                bundle_id: entry.bundle_id.clone(),
                exe_path: entry.exe_path.clone(),
                icon_data_url: crate::app_icons::resolve_process_icon_data_url(
                    entry.exe_path.as_deref(),
                    entry.bundle_id.as_deref(),
                    &entry.name,
                    &entry.exec_name,
                ),
                memory_bytes: entry.memory_bytes,
                virtual_memory_bytes: entry.virtual_memory_bytes,
                disk_read_bytes: entry.disk_read_bytes,
                disk_write_bytes: entry.disk_write_bytes,
                net_rx_bytes_per_sec: entry.net_rx_bytes_per_sec,
                net_tx_bytes_per_sec: entry.net_tx_bytes_per_sec,
                energy_impact_score: entry.energy_impact_score,
                cpu_usage_percent: entry.cpu_pct,
                start_time: entry.start_time,
                is_system: crate::killer::is_immutable_blocked_process_name(&entry.name),
            }
        })
        .collect::<Vec<_>>();

    processes.sort_by_key(|p| std::cmp::Reverse(p.memory_bytes));
    if let Some(max) = limit {
        processes.truncate(max);
    }

    TelemetrySnapshot {
        total_memory_bytes: state.total_memory_bytes,
        free_memory_bytes: state.free_memory_bytes,
        used_memory_bytes: state.used_memory_bytes,
        free_percent: state.free_percent,
        swap_used_mb: state.swap_used_mb,
        cpu_usage_percent: state.cpu_usage_percent,
        net_rx_bytes_per_sec: state.net_rx_bytes_per_sec,
        net_tx_bytes_per_sec: state.net_tx_bytes_per_sec,
        total_processes: state.cached_process_info.len() as u32,
        processes,
        super_processes,
    }
}

fn super_processes_from_state(
    state: &crate::watcher::SystemState,
    limit: Option<usize>,
) -> Vec<crate::metrics::SuperProcess> {
    let mut grouped: HashMap<String, crate::metrics::SuperProcess> = HashMap::new();

    for process in &state.cached_process_info {
        let identity = crate::process_identity::resolve_group_identity(
            &process.name,
            &process.exec_name,
            process.exe_path.as_deref(),
            process.bundle_id.as_deref(),
            process.group_name == "System",
        );

        let entry =
            grouped
                .entry(identity.key.clone())
                .or_insert_with(|| crate::metrics::SuperProcess {
                    binary_key: identity.key,
                    identity_type: identity.identity_type,
                    display_name: identity.display_name,
                    group: identity.group,
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
                    energy_impact_score: None,
                });

        entry.process_count += 1;
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
        entry.total_cpu_usage_percent += process.cpu_pct;
        entry.total_net_rx_bytes_per_sec = entry
            .total_net_rx_bytes_per_sec
            .saturating_add(process.net_rx_bytes_per_sec);
        entry.total_net_tx_bytes_per_sec = entry
            .total_net_tx_bytes_per_sec
            .saturating_add(process.net_tx_bytes_per_sec);
        entry.energy_impact_score = Some(
            entry.energy_impact_score.unwrap_or(0.0)
                + process.energy_impact_score.unwrap_or_default(),
        );
    }

    let mut values: Vec<_> = grouped.into_values().collect();
    values.sort_by_key(|v| std::cmp::Reverse(v.total_memory_bytes));
    if let Some(max) = limit {
        values.truncate(max);
    }
    values
}
