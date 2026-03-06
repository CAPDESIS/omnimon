use serde::{Deserialize, Serialize};
use sysinfo::System;

/// A single process entry with its PID, name, and memory usage in bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryEntry {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
}

/// Snapshot of system-wide memory: total, free, and used bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMemory {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
}

/// Returns the top `limit` processes sorted by memory usage in descending order.
pub fn top_processes_by_memory(limit: usize) -> Vec<ProcessMemoryEntry> {
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

/// Collects a snapshot of system memory, preferring native OS APIs over sysinfo fallback.
pub fn free_system_memory() -> SystemMemory {
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
}
