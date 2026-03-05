use core::{killer, metrics, watcher};

#[test]
fn integration_metrics_are_consistent() {
    let memory = metrics::free_system_memory();
    assert!(memory.total_memory_bytes >= memory.used_memory_bytes);
    assert!(memory.total_memory_bytes >= memory.free_memory_bytes);
}

#[test]
fn integration_top_processes_respects_limit_and_order() {
    let top = metrics::top_processes_by_memory(15);
    assert!(top.len() <= 15);
    for pair in top.windows(2) {
        assert!(pair[0].memory_bytes >= pair[1].memory_bytes);
    }
}

#[test]
fn integration_watcher_cache_is_readable() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2300));
    let state = watcher::get_cached_state();
    assert!(state.total_memory_bytes > 0);
    assert!(state.updated_at_unix_ms > 0);
}

#[test]
fn integration_killer_rejects_invalid_pid() {
    let result = killer::kill_process_safe(0, &[]);
    assert!(matches!(result, Err(killer::KillError::InvalidPid(0))));
}
