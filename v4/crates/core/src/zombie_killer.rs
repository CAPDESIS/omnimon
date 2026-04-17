//! Zombie process detection. Classifies long-running processes consuming
//! excessive CPU or RAM as candidates that likely have no active user
//! interaction.
//!
//! The scoring is stateless and cheap. Callers (service layer) are
//! responsible for confirming a candidate only after it has remained
//! "hot" continuously for [`ZombieKillerConfig::sustained_secs`].

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::watcher::CachedProcessInfo;

const DEFAULT_CPU_THRESHOLD_PCT: f32 = 50.0;
const DEFAULT_MIN_UPTIME_SECS: u64 = 7 * 24 * 60 * 60;
const DEFAULT_SUSTAINED_SECS: u64 = 60 * 60;
const MIN_CPU_THRESHOLD_PCT: f32 = 1.0;
const MAX_CPU_THRESHOLD_PCT: f32 = 10_000.0;
const MIN_UPTIME_SECS: u64 = 60;
const MAX_UPTIME_SECS: u64 = 365 * 24 * 60 * 60;
const MIN_SUSTAINED_SECS: u64 = 60;
const MAX_SUSTAINED_SECS: u64 = 24 * 60 * 60;

/// Runtime configuration for the zombie killer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZombieKillerConfig {
    pub enabled: bool,
    pub cpu_threshold_pct: f32,
    pub ram_threshold_bytes: u64,
    pub min_uptime_secs: u64,
    pub sustained_secs: u64,
    pub auto_kill: bool,
    pub never_kill: Vec<String>,
}

impl Default for ZombieKillerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_threshold_pct: DEFAULT_CPU_THRESHOLD_PCT,
            ram_threshold_bytes: 0,
            min_uptime_secs: DEFAULT_MIN_UPTIME_SECS,
            sustained_secs: DEFAULT_SUSTAINED_SECS,
            auto_kill: false,
            never_kill: Vec::new(),
        }
    }
}

/// Clamp every numeric field so that broken or malicious stored config cannot
/// push the engine into a pathological state (e.g. sustained = 0 → instant kill).
pub fn sanitize_config(mut config: ZombieKillerConfig) -> ZombieKillerConfig {
    if !config.cpu_threshold_pct.is_finite() {
        config.cpu_threshold_pct = DEFAULT_CPU_THRESHOLD_PCT;
    }
    config.cpu_threshold_pct = config
        .cpu_threshold_pct
        .clamp(MIN_CPU_THRESHOLD_PCT, MAX_CPU_THRESHOLD_PCT);
    config.min_uptime_secs = config
        .min_uptime_secs
        .clamp(MIN_UPTIME_SECS, MAX_UPTIME_SECS);
    config.sustained_secs = config
        .sustained_secs
        .clamp(MIN_SUSTAINED_SECS, MAX_SUSTAINED_SECS);
    config.never_kill.retain(|name| !name.trim().is_empty());
    for name in &mut config.never_kill {
        *name = name.trim().to_string();
    }
    config
}

/// Reason a process was flagged. Serialized as a snake_case string so the
/// frontend can map it to a localized label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZombieReason {
    CpuSustained,
    RamSustained,
    CpuAndRamSustained,
}

/// A process that currently satisfies the zombie criteria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZombieCandidate {
    pub pid: u32,
    pub name: String,
    pub exec_name: String,
    pub exe_path: Option<String>,
    pub cpu_pct: f32,
    pub memory_bytes: u64,
    pub age_secs: u64,
    pub reason: ZombieReason,
    /// Unix seconds when the process was spawned. Used by the service layer to
    /// key sustained-violation tracking by `(pid, start_time)`, which survives
    /// PID reuse: a new process with the same PID gets a fresh entry because
    /// its `start_time` differs from the dead process.
    pub start_time: u64,
}

/// Current Unix timestamp in seconds. Returns 0 if the system clock is
/// misconfigured.
pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Stateless per-tick scoring. Returns every process currently exceeding
/// the configured thresholds.
///
/// Protected system processes (see [`crate::killer::is_immutable_blocked_process_name`])
/// and anything listed in `config.never_kill` are always excluded.
pub fn identify_candidates(
    processes: &[CachedProcessInfo],
    config: &ZombieKillerConfig,
    now_unix_secs: u64,
) -> Vec<ZombieCandidate> {
    if !config.enabled {
        return Vec::new();
    }

    processes
        .iter()
        .filter_map(|proc| classify_process(proc, config, now_unix_secs))
        .collect()
}

fn classify_process(
    proc: &CachedProcessInfo,
    config: &ZombieKillerConfig,
    now_unix_secs: u64,
) -> Option<ZombieCandidate> {
    if crate::killer::is_immutable_blocked_process_name(&proc.name) {
        return None;
    }

    if config.never_kill.iter().any(|entry| {
        entry.eq_ignore_ascii_case(&proc.name) || entry.eq_ignore_ascii_case(&proc.exec_name)
    }) {
        return None;
    }

    let age_secs = now_unix_secs.saturating_sub(proc.start_time);
    if age_secs < config.min_uptime_secs {
        return None;
    }

    let cpu_hot = proc.cpu_pct >= config.cpu_threshold_pct;
    let ram_hot = config.ram_threshold_bytes > 0 && proc.memory_bytes >= config.ram_threshold_bytes;

    let reason = match (cpu_hot, ram_hot) {
        (true, true) => ZombieReason::CpuAndRamSustained,
        (true, false) => ZombieReason::CpuSustained,
        (false, true) => ZombieReason::RamSustained,
        (false, false) => return None,
    };

    Some(ZombieCandidate {
        pid: proc.pid,
        name: proc.name.clone(),
        exec_name: proc.exec_name.clone(),
        exe_path: proc.exe_path.clone(),
        cpu_pct: proc.cpu_pct,
        memory_bytes: proc.memory_bytes,
        age_secs,
        reason,
        start_time: proc.start_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proc(
        pid: u32,
        name: &str,
        cpu_pct: f32,
        memory_bytes: u64,
        start_time: u64,
    ) -> CachedProcessInfo {
        CachedProcessInfo {
            pid,
            name: name.to_string(),
            group_name: String::new(),
            memory_bytes,
            virtual_memory_bytes: 0,
            cpu_pct,
            exec_name: name.to_string(),
            exe_path: None,
            bundle_id: None,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            net_rx_bytes_per_sec: 0,
            net_tx_bytes_per_sec: 0,
            energy_impact_score: None,
            start_time,
        }
    }

    #[test]
    fn default_config_is_conservative_and_alert_only() {
        let c = ZombieKillerConfig::default();
        assert!(c.enabled);
        assert_eq!(c.cpu_threshold_pct, 50.0);
        assert_eq!(c.min_uptime_secs, 7 * 24 * 60 * 60);
        assert_eq!(c.sustained_secs, 3600);
        assert!(!c.auto_kill);
        assert!(c.never_kill.is_empty());
    }

    #[test]
    fn disabled_config_returns_empty() {
        let config = ZombieKillerConfig {
            enabled: false,
            ..Default::default()
        };
        let procs = vec![make_proc(1, "greedy", 99.0, 1_000_000_000, 0)];
        assert!(identify_candidates(&procs, &config, 10_000_000).is_empty());
    }

    #[test]
    fn protected_system_process_is_never_flagged() {
        let config = ZombieKillerConfig::default();
        let procs = vec![make_proc(1, "launchd", 99.0, 10_000_000_000, 0)];
        assert!(identify_candidates(&procs, &config, 10_000_000).is_empty());
    }

    #[test]
    fn user_blocklist_prevents_flagging_case_insensitive() {
        let config = ZombieKillerConfig {
            never_kill: vec!["MyDaemon".to_string()],
            ..Default::default()
        };
        let procs = vec![make_proc(1, "mydaemon", 99.0, 0, 0)];
        assert!(identify_candidates(&procs, &config, 10_000_000).is_empty());
    }

    #[test]
    fn young_process_is_not_flagged() {
        let config = ZombieKillerConfig::default();
        let now = 1_000_000;
        let start = now - 60;
        let procs = vec![make_proc(1, "fresh", 99.0, 0, start)];
        assert!(identify_candidates(&procs, &config, now).is_empty());
    }

    #[test]
    fn cool_old_process_is_not_flagged() {
        let config = ZombieKillerConfig::default();
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        let procs = vec![make_proc(1, "idle", 5.0, 0, start)];
        assert!(identify_candidates(&procs, &config, now).is_empty());
    }

    #[test]
    fn hot_old_process_is_flagged_as_cpu_sustained() {
        let config = ZombieKillerConfig::default();
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        let procs = vec![make_proc(1337, "adobe", 107.7, 50_000_000, start)];
        let candidates = identify_candidates(&procs, &config, now);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].pid, 1337);
        assert_eq!(candidates[0].reason, ZombieReason::CpuSustained);
        assert_eq!(candidates[0].age_secs, 10 * 24 * 60 * 60);
    }

    #[test]
    fn hot_cpu_and_ram_marks_combined_reason() {
        let config = ZombieKillerConfig {
            ram_threshold_bytes: 100_000_000,
            ..Default::default()
        };
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        let procs = vec![make_proc(2, "hog", 75.0, 500_000_000, start)];
        let candidates = identify_candidates(&procs, &config, now);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, ZombieReason::CpuAndRamSustained);
    }

    #[test]
    fn ram_only_threshold_flags_ram_sustained() {
        let config = ZombieKillerConfig {
            cpu_threshold_pct: 10_000.0,
            ram_threshold_bytes: 1_000_000_000,
            ..Default::default()
        };
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        let procs = vec![make_proc(3, "mem_hog", 10.0, 2_000_000_000, start)];
        let candidates = identify_candidates(&procs, &config, now);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, ZombieReason::RamSustained);
    }

    #[test]
    fn ram_threshold_zero_means_ignore_ram() {
        let config = ZombieKillerConfig::default();
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        // Lots of RAM, low CPU → should NOT be flagged because RAM is disabled.
        let procs = vec![make_proc(4, "big_but_idle", 5.0, 50_000_000_000, start)];
        assert!(identify_candidates(&procs, &config, now).is_empty());
    }

    #[test]
    fn start_time_in_the_future_is_not_flagged() {
        // saturating_sub clamps age to 0, which is below any uptime threshold.
        let config = ZombieKillerConfig::default();
        let now = 1_000_000;
        let start = now + 1_000;
        let procs = vec![make_proc(1, "future", 99.0, 0, start)];
        assert!(identify_candidates(&procs, &config, now).is_empty());
    }

    #[test]
    fn sanitize_clamps_sustained_secs_floor() {
        let config = ZombieKillerConfig {
            sustained_secs: 0,
            ..Default::default()
        };
        let sanitized = sanitize_config(config);
        assert!(sanitized.sustained_secs >= MIN_SUSTAINED_SECS);
    }

    #[test]
    fn sanitize_clamps_cpu_threshold_nan_to_default() {
        let config = ZombieKillerConfig {
            cpu_threshold_pct: f32::NAN,
            ..Default::default()
        };
        let sanitized = sanitize_config(config);
        assert_eq!(sanitized.cpu_threshold_pct, DEFAULT_CPU_THRESHOLD_PCT);
    }

    #[test]
    fn sanitize_strips_blank_blocklist_entries() {
        let config = ZombieKillerConfig {
            never_kill: vec![
                "   ".to_string(),
                "Adobe".to_string(),
                "".to_string(),
                " Finder ".to_string(),
            ],
            ..Default::default()
        };
        let sanitized = sanitize_config(config);
        assert_eq!(sanitized.never_kill, vec!["Adobe", "Finder"]);
    }

    #[test]
    fn candidate_serializes_reason_as_snake_case() {
        let candidate = ZombieCandidate {
            pid: 5,
            name: "test".to_string(),
            exec_name: "test".to_string(),
            exe_path: None,
            cpu_pct: 80.0,
            memory_bytes: 123,
            age_secs: 100,
            reason: ZombieReason::CpuSustained,
            start_time: 1_000_000,
        };
        let json = serde_json::to_string(&candidate).unwrap();
        assert!(json.contains("\"reason\":\"cpu_sustained\""));
        assert!(json.contains("\"pid\":5"));
        assert!(json.contains("\"execName\":\"test\""));
        assert!(json.contains("\"startTime\":1000000"));
    }

    #[test]
    fn candidate_exposes_start_time_for_pid_reuse_detection() {
        let config = ZombieKillerConfig::default();
        let now = 10_000_000;
        let start = now - (10 * 24 * 60 * 60);
        let procs = vec![make_proc(1, "old", 99.0, 0, start)];
        let candidates = identify_candidates(&procs, &config, now);
        assert_eq!(candidates[0].start_time, start);
    }

    #[test]
    fn config_roundtrips_through_json() {
        let original = ZombieKillerConfig {
            enabled: true,
            cpu_threshold_pct: 75.0,
            ram_threshold_bytes: 2_000_000_000,
            min_uptime_secs: 3 * 24 * 60 * 60,
            sustained_secs: 1800,
            auto_kill: true,
            never_kill: vec!["Xcode".to_string()],
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ZombieKillerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }
}
