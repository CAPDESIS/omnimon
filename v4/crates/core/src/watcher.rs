use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::System;

static CACHED_STATE: OnceLock<Arc<RwLock<SystemState>>> = OnceLock::new();
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub free_percent: u32,
    pub swap_used_mb: u64,
    pub cpu_usage_percent: f32,
    pub updated_at_unix_ms: u128,
}

fn state_handle() -> Arc<RwLock<SystemState>> {
    Arc::clone(CACHED_STATE.get_or_init(|| Arc::new(RwLock::new(SystemState::default()))))
}

fn collect_state(system: &mut System) -> SystemState {
    system.refresh_memory();
    system.refresh_cpu();

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

    SystemState {
        total_memory_bytes,
        free_memory_bytes,
        used_memory_bytes,
        free_percent,
        swap_used_mb,
        cpu_usage_percent,
        updated_at_unix_ms,
    }
}

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

            let initial = collect_state(&mut system);
            if let Ok(mut guard) = cache.write() {
                *guard = initial;
            }

            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let snapshot = collect_state(&mut system);
                if let Ok(mut guard) = cache.write() {
                    *guard = snapshot;
                }
            }
        });
    });
}

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
