use crate::network::{NetworkFlowSample, ProcessConnectionEvent};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::{OnceLock, RwLock};

pub type NetworkSnapshot = NetworkFlowSample;

const REQUIRED_CONSECUTIVE_MATCHES: u32 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Upload,
    Download,
    Both,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkAlertRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub cooldown_seconds: u32,
    pub notify_ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlertCondition {
    HighBandwidth {
        threshold_mbps: f64,
        direction: Direction,
        process: Option<String>,
    },
    NewExternalConnection {
        exclude_known: bool,
    },
    UnusualPort {
        suspicious_ports: Vec<u16>,
    },
    ProcessNetworkSpike {
        process_name: String,
        multiplier: f64,
    },
    ConnectionCountExceeded {
        max_connections: usize,
        process: Option<String>,
    },
    SuspiciousDestination {
        patterns: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NetworkAlert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub condition_kind: String,
    pub message: String,
    pub triggered_at_unix_ms: u128,
    pub notify_ai: bool,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
    pub destination: Option<String>,
    pub bandwidth_mbps: Option<f64>,
    pub connection_count: Option<usize>,
    pub details: Vec<String>,
}

/// Mutable per-tick state for the alert evaluator. Holds debounce counters
/// and cooldown timestamps, plus the running set of destinations we've seen.
///
/// This used to be a process-global `OnceLock<RwLock<EvaluatorState>>`, which
/// made tests contaminate each other's counters whenever they ran in parallel.
/// It is now owned by the caller (watcher tick in production, each test in
/// test code), which completely eliminates that class of flake.
#[derive(Default, Debug)]
pub struct EvaluatorState {
    consecutive_matches: HashMap<String, u32>,
    last_triggered_ms: HashMap<String, u128>,
    known_destinations: HashSet<String>,
}

impl EvaluatorState {
    /// Build a fresh evaluator with empty counters and no known destinations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop every accumulated counter and destination memory.
    pub fn clear(&mut self) {
        self.consecutive_matches.clear();
        self.last_triggered_ms.clear();
        self.known_destinations.clear();
    }
}

static RULES: OnceLock<RwLock<Vec<NetworkAlertRule>>> = OnceLock::new();

fn rules_state() -> &'static RwLock<Vec<NetworkAlertRule>> {
    RULES.get_or_init(|| RwLock::new(Vec::new()))
}

pub fn set_active_rules(rules: Vec<NetworkAlertRule>) {
    if let Ok(mut guard) = rules_state().write() {
        *guard = rules;
    }
}

pub fn active_rules() -> Vec<NetworkAlertRule> {
    rules_state()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Evaluate alerts against the globally configured rule set.
///
/// Thin wrapper over [`evaluate_network_alerts`] that reads the current rules
/// from the shared [`RULES`] store. Callers that already own their rules
/// should prefer `evaluate_network_alerts` directly to avoid the RwLock
/// read.
pub fn evaluate_active_network_alerts(
    snapshot: &NetworkSnapshot,
    prev_snapshot: Option<&NetworkSnapshot>,
    history: &[NetworkSnapshot],
    state: &mut EvaluatorState,
) -> Vec<NetworkAlert> {
    let rules = active_rules();
    evaluate_network_alerts(snapshot, prev_snapshot, &rules, history, state)
}

/// Evaluate `rules` against `snapshot`, advancing `state`'s debounce and
/// cooldown bookkeeping in place. Safe to call from multiple threads as long
/// as each thread owns its own `state` (there is no hidden global).
pub fn evaluate_network_alerts(
    snapshot: &NetworkSnapshot,
    prev_snapshot: Option<&NetworkSnapshot>,
    rules: &[NetworkAlertRule],
    history: &[NetworkSnapshot],
    state: &mut EvaluatorState,
) -> Vec<NetworkAlert> {
    let mut alerts = Vec::new();
    let mut seen_external_destinations = HashSet::new();

    for event in &snapshot.recent_connections {
        if is_external_ip(&event.dst_ip) {
            seen_external_destinations.insert(destination_key(event));
        }
    }

    for rule in rules.iter().filter(|rule| rule.enabled) {
        let Some(candidate) = check_rule(rule, snapshot, prev_snapshot, history, state) else {
            clear_rule_debounce(rule, state);
            continue;
        };

        let consecutive = state
            .consecutive_matches
            .entry(candidate.debounce_key.clone())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);

        if *consecutive < REQUIRED_CONSECUTIVE_MATCHES {
            continue;
        }

        let cooldown_ms = u128::from(rule.cooldown_seconds) * 1000;
        if cooldown_ms > 0
            && state
                .last_triggered_ms
                .get(&candidate.cooldown_key)
                .copied()
                .map(|last_triggered| {
                    snapshot.captured_at_unix_ms.saturating_sub(last_triggered) < cooldown_ms
                })
                .unwrap_or(false)
        {
            continue;
        }

        state
            .last_triggered_ms
            .insert(candidate.cooldown_key.clone(), snapshot.captured_at_unix_ms);

        alerts.push(NetworkAlert {
            id: format!(
                "{}-{}-{}",
                rule.id, candidate.cooldown_key, snapshot.captured_at_unix_ms
            ),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            severity: rule.severity,
            condition_kind: candidate.condition_kind,
            message: candidate.message,
            triggered_at_unix_ms: snapshot.captured_at_unix_ms,
            notify_ai: rule.notify_ai,
            process_name: candidate.process_name,
            pid: candidate.pid,
            destination: candidate.destination,
            bandwidth_mbps: candidate.bandwidth_mbps,
            connection_count: candidate.connection_count,
            details: candidate.details,
        });
    }

    state.known_destinations.extend(seen_external_destinations);
    alerts
}

fn clear_rule_debounce(rule: &NetworkAlertRule, state: &mut EvaluatorState) {
    let prefix = format!("{}:", rule.id);
    state
        .consecutive_matches
        .retain(|key, _| !key.starts_with(&prefix));
}

struct RuleCandidate {
    debounce_key: String,
    cooldown_key: String,
    condition_kind: String,
    message: String,
    process_name: Option<String>,
    pid: Option<u32>,
    destination: Option<String>,
    bandwidth_mbps: Option<f64>,
    connection_count: Option<usize>,
    details: Vec<String>,
}

fn check_rule(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    _prev_snapshot: Option<&NetworkSnapshot>,
    history: &[NetworkSnapshot],
    state: &EvaluatorState,
) -> Option<RuleCandidate> {
    match &rule.condition {
        AlertCondition::HighBandwidth {
            threshold_mbps,
            direction,
            process,
        } => check_high_bandwidth(
            rule,
            snapshot,
            *threshold_mbps,
            *direction,
            process.as_deref(),
        ),
        AlertCondition::NewExternalConnection { exclude_known } => {
            check_new_external_connection(rule, snapshot, *exclude_known, state)
        }
        AlertCondition::UnusualPort { suspicious_ports } => {
            check_unusual_port(rule, snapshot, suspicious_ports)
        }
        AlertCondition::ProcessNetworkSpike {
            process_name,
            multiplier,
        } => check_process_spike(rule, snapshot, history, process_name, *multiplier),
        AlertCondition::ConnectionCountExceeded {
            max_connections,
            process,
        } => check_connection_count(rule, snapshot, *max_connections, process.as_deref()),
        AlertCondition::SuspiciousDestination { patterns } => {
            check_suspicious_destination(rule, snapshot, patterns)
        }
    }
}

fn check_high_bandwidth(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    threshold_mbps: f64,
    direction: Direction,
    process: Option<&str>,
) -> Option<RuleCandidate> {
    if let Some(process_name) = process {
        let throughput = snapshot.process_throughput.iter().find(|entry| {
            entry
                .process_name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(process_name))
                .unwrap_or(false)
        })?;
        let bandwidth = throughput_direction_mbps(
            throughput.rx_bytes_per_sec,
            throughput.tx_bytes_per_sec,
            direction,
        );
        if bandwidth < threshold_mbps {
            return None;
        }

        let actual_name = throughput
            .process_name
            .clone()
            .unwrap_or_else(|| process_name.to_string());

        return Some(RuleCandidate {
            debounce_key: format!("{}:pid:{}", rule.id, throughput.pid),
            cooldown_key: format!("{}:pid:{}", rule.id, throughput.pid),
            condition_kind: "high_bandwidth".to_string(),
            message: format!(
                "{} supero {} con {:.2} Mbps",
                actual_name,
                direction_label(direction),
                bandwidth
            ),
            process_name: Some(actual_name),
            pid: Some(throughput.pid),
            destination: None,
            bandwidth_mbps: Some(round2(bandwidth)),
            connection_count: None,
            details: vec![format!("Threshold: {:.2} Mbps", threshold_mbps)],
        });
    }

    let bandwidth = throughput_direction_mbps(
        snapshot.net_rx_bytes_per_sec,
        snapshot.net_tx_bytes_per_sec,
        direction,
    );
    if bandwidth < threshold_mbps {
        return None;
    }

    Some(RuleCandidate {
        debounce_key: format!("{}:system:{}", rule.id, direction_label(direction)),
        cooldown_key: format!("{}:system:{}", rule.id, direction_label(direction)),
        condition_kind: "high_bandwidth".to_string(),
        message: format!(
            "El trafico total supero {} con {:.2} Mbps",
            direction_label(direction),
            bandwidth
        ),
        process_name: None,
        pid: None,
        destination: None,
        bandwidth_mbps: Some(round2(bandwidth)),
        connection_count: None,
        details: vec![format!("Threshold: {:.2} Mbps", threshold_mbps)],
    })
}

fn check_new_external_connection(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    exclude_known: bool,
    state: &EvaluatorState,
) -> Option<RuleCandidate> {
    let event = snapshot
        .recent_connections
        .iter()
        .find(|event| is_external_ip(&event.dst_ip))?;
    let destination = destination_key(event);
    if exclude_known && state.known_destinations.contains(&destination) {
        return None;
    }

    Some(RuleCandidate {
        debounce_key: format!("{}:{}", rule.id, destination),
        cooldown_key: format!("{}:{}", rule.id, destination),
        condition_kind: "new_external_connection".to_string(),
        message: format!("Nueva conexion externa observada hacia {}", destination),
        process_name: None,
        pid: Some(event.pid),
        destination: Some(destination.clone()),
        bandwidth_mbps: None,
        connection_count: None,
        details: vec![format!("Source port: {}", event.src_port)],
    })
}

fn check_unusual_port(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    suspicious_ports: &[u16],
) -> Option<RuleCandidate> {
    let event = snapshot
        .recent_connections
        .iter()
        .find(|event| suspicious_ports.contains(&event.dst_port))?;
    let destination = destination_key(event);

    Some(RuleCandidate {
        debounce_key: format!("{}:{}", rule.id, destination),
        cooldown_key: format!("{}:{}", rule.id, destination),
        condition_kind: "unusual_port".to_string(),
        message: format!(
            "Conexion a puerto sospechoso {} en {}",
            event.dst_port, destination
        ),
        process_name: None,
        pid: Some(event.pid),
        destination: Some(destination),
        bandwidth_mbps: None,
        connection_count: None,
        details: vec![format!("Ports: {:?}", suspicious_ports)],
    })
}

fn check_process_spike(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    history: &[NetworkSnapshot],
    process_name: &str,
    multiplier: f64,
) -> Option<RuleCandidate> {
    let current = snapshot.process_throughput.iter().find(|entry| {
        entry
            .process_name
            .as_deref()
            .map(|name| name.eq_ignore_ascii_case(process_name))
            .unwrap_or(false)
    })?;

    let current_total = current
        .rx_bytes_per_sec
        .saturating_add(current.tx_bytes_per_sec);
    let current_mbps = bytes_per_sec_to_mbps(current_total);

    let mut samples = 0usize;
    let mut total = 0f64;
    for snapshot in history.iter().rev().take(10) {
        if let Some(item) = snapshot.process_throughput.iter().find(|entry| {
            entry
                .process_name
                .as_deref()
                .map(|name| name.eq_ignore_ascii_case(process_name))
                .unwrap_or(false)
        }) {
            total += item.rx_bytes_per_sec.saturating_add(item.tx_bytes_per_sec) as f64;
            samples += 1;
        }
    }
    if samples == 0 {
        return None;
    }

    let average = total / samples as f64;
    if average <= 0.0 || (current_total as f64) < average * multiplier {
        return None;
    }

    Some(RuleCandidate {
        debounce_key: format!("{}:pid:{}", rule.id, current.pid),
        cooldown_key: format!("{}:pid:{}", rule.id, current.pid),
        condition_kind: "process_network_spike".to_string(),
        message: format!(
            "{} tiene un pico de red de {:.2}x sobre su promedio",
            current
                .process_name
                .clone()
                .unwrap_or_else(|| process_name.to_string()),
            (current_total as f64 / average).max(0.0)
        ),
        process_name: current.process_name.clone(),
        pid: Some(current.pid),
        destination: None,
        bandwidth_mbps: Some(round2(current_mbps)),
        connection_count: None,
        details: vec![format!("Multiplier threshold: {:.2}", multiplier)],
    })
}

fn check_connection_count(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    max_connections: usize,
    process: Option<&str>,
) -> Option<RuleCandidate> {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for event in &snapshot.recent_connections {
        *counts.entry(event.pid).or_insert(0) += 1;
    }

    let matched = snapshot.process_throughput.iter().find_map(|entry| {
        if let Some(process_name) = process {
            let name = entry.process_name.as_deref()?;
            if !name.eq_ignore_ascii_case(process_name) {
                return None;
            }
        }
        let count = counts.get(&entry.pid).copied().unwrap_or(0);
        if count > max_connections {
            Some((entry, count))
        } else {
            None
        }
    })?;

    Some(RuleCandidate {
        debounce_key: format!("{}:pid:{}", rule.id, matched.0.pid),
        cooldown_key: format!("{}:pid:{}", rule.id, matched.0.pid),
        condition_kind: "connection_count_exceeded".to_string(),
        message: format!(
            "{} supero el limite de conexiones con {}",
            matched
                .0
                .process_name
                .clone()
                .unwrap_or_else(|| format!("pid:{}", matched.0.pid)),
            matched.1
        ),
        process_name: matched.0.process_name.clone(),
        pid: Some(matched.0.pid),
        destination: None,
        bandwidth_mbps: None,
        connection_count: Some(matched.1),
        details: vec![format!("Max allowed: {}", max_connections)],
    })
}

fn check_suspicious_destination(
    rule: &NetworkAlertRule,
    snapshot: &NetworkSnapshot,
    patterns: &[String],
) -> Option<RuleCandidate> {
    let compiled = patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect::<Vec<_>>();
    if compiled.is_empty() {
        return None;
    }

    let event = snapshot.recent_connections.iter().find(|event| {
        compiled
            .iter()
            .any(|regex| regex.is_match(&event.dst_ip) || regex.is_match(&destination_key(event)))
    })?;
    let destination = destination_key(event);

    Some(RuleCandidate {
        debounce_key: format!("{}:{}", rule.id, destination),
        cooldown_key: format!("{}:{}", rule.id, destination),
        condition_kind: "suspicious_destination".to_string(),
        message: format!("Destino sospechoso detectado: {}", destination),
        process_name: None,
        pid: Some(event.pid),
        destination: Some(destination),
        bandwidth_mbps: None,
        connection_count: None,
        details: patterns.to_vec(),
    })
}

fn throughput_direction_mbps(
    rx_bytes_per_sec: u64,
    tx_bytes_per_sec: u64,
    direction: Direction,
) -> f64 {
    match direction {
        Direction::Upload => bytes_per_sec_to_mbps(tx_bytes_per_sec),
        Direction::Download => bytes_per_sec_to_mbps(rx_bytes_per_sec),
        Direction::Both => bytes_per_sec_to_mbps(rx_bytes_per_sec.saturating_add(tx_bytes_per_sec)),
    }
}

fn bytes_per_sec_to_mbps(bytes_per_sec: u64) -> f64 {
    (bytes_per_sec as f64 * 8.0) / 1_000_000.0
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Upload => "upload",
        Direction::Download => "download",
        Direction::Both => "trafico total",
    }
}

fn destination_key(event: &ProcessConnectionEvent) -> String {
    format!("{}:{}", event.dst_ip, event.dst_port)
}

fn is_external_ip(ip: &str) -> bool {
    let Ok(addr) = ip.parse::<IpAddr>() else {
        return false;
    };

    match addr {
        IpAddr::V4(v4) => !is_private_v4(v4),
        IpAddr::V6(v6) => !is_private_v6(v6),
    }
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
}

fn is_private_v6(ip: Ipv6Addr) -> bool {
    ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local() || ip.is_unicast_link_local()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Clear the globally configured rule set. The per-tick evaluator state is
/// now owned by the caller, so this function no longer resets counters —
/// callers create a fresh [`EvaluatorState`] for each test.
pub fn reset_network_alert_state_for_tests() {
    if let Ok(mut guard) = rules_state().write() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{
        NetworkCaptureBackend, ProcessNetworkThroughput, TrafficDirection, TransportProtocol,
    };
    use std::sync::{Mutex, OnceLock};

    /// Serializes only the tests that touch the globally configured
    /// [`RULES`] store (i.e. those that call [`set_active_rules`] or
    /// [`evaluate_active_network_alerts`]).
    ///
    /// The previous process-global `EVALUATOR_STATE` has been eliminated —
    /// each test now instantiates its own [`EvaluatorState`] and passes it
    /// to [`evaluate_network_alerts`], so counters can no longer leak
    /// across parallel test threads. Tests that build their own rule slice
    /// locally do not need this guard.
    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn sample() -> NetworkSnapshot {
        NetworkSnapshot {
            backend: NetworkCaptureBackend::Unsupported,
            backend_label: "Unsupported".to_string(),
            privileged_path_available: false,
            deep_packet_inspection_active: false,
            net_rx_bytes_per_sec: 0,
            net_tx_bytes_per_sec: 0,
            observed_interval_ms: 2000,
            process_throughput: vec![ProcessNetworkThroughput {
                pid: 42,
                process_name: Some("chrome".to_string()),
                rx_bytes_per_sec: 0,
                tx_bytes_per_sec: 0,
                tcp_packets_per_sec: 0,
                udp_packets_per_sec: 0,
            }],
            recent_connections: vec![],
            capture_windows_dropped: 0,
            captured_at_unix_ms: 1_000,
        }
    }

    #[test]
    fn emits_alert_after_three_consecutive_matches() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "port-watch".to_string(),
            name: "Puerto sospechoso".to_string(),
            enabled: true,
            condition: AlertCondition::UnusualPort {
                suspicious_ports: vec![4444],
            },
            severity: AlertSeverity::Critical,
            cooldown_seconds: 60,
            notify_ai: true,
        }];

        let mut snapshot = sample();
        snapshot.recent_connections = vec![ProcessConnectionEvent {
            pid: 42,
            protocol: TransportProtocol::Tcp,
            direction: TrafficDirection::Outbound,
            src_ip: "10.0.0.10".to_string(),
            dst_ip: "8.8.8.8".to_string(),
            src_port: 50_000,
            dst_port: 4444,
            bytes: 512,
        }];

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "port-watch");
        assert!(alerts[0].notify_ai);
    }

    #[test]
    fn cooldown_suppresses_duplicate_alerts() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "bandwidth".to_string(),
            name: "Bandwidth".to_string(),
            enabled: true,
            condition: AlertCondition::HighBandwidth {
                threshold_mbps: 10.0,
                direction: Direction::Upload,
                process: Some("chrome".to_string()),
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 30,
            notify_ai: false,
        }];

        let mut snapshot = sample();
        snapshot.process_throughput[0].tx_bytes_per_sec = 5_000_000;

        snapshot.captured_at_unix_ms = 10_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms = 12_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms = 14_000;
        assert_eq!(
            evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).len(),
            1
        );
        snapshot.captured_at_unix_ms = 16_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
    }

    #[test]
    fn active_rules_drive_evaluate_active_network_alerts() {
        // The ONLY remaining test that touches the globally configured
        // `RULES` store. The guard prevents a future test from corrupting
        // this one by writing rules concurrently; every other test in this
        // module now carries its own [`EvaluatorState`] and local rules.
        let _guard = test_guard();
        reset_network_alert_state_for_tests();
        let mut state = EvaluatorState::new();
        set_active_rules(vec![NetworkAlertRule {
            id: "new-external".to_string(),
            name: "Nueva externa".to_string(),
            enabled: true,
            condition: AlertCondition::NewExternalConnection {
                exclude_known: false,
            },
            severity: AlertSeverity::Info,
            cooldown_seconds: 0,
            notify_ai: false,
        }]);

        let mut snapshot = sample();
        snapshot.recent_connections = vec![ProcessConnectionEvent {
            pid: 99,
            protocol: TransportProtocol::Tcp,
            direction: TrafficDirection::Outbound,
            src_ip: "10.0.0.5".to_string(),
            dst_ip: "8.8.4.4".to_string(),
            src_port: 40_000,
            dst_port: 443,
            bytes: 1024,
        }];

        assert!(evaluate_active_network_alerts(&snapshot, None, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_active_network_alerts(&snapshot, None, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_active_network_alerts(&snapshot, None, &[], &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "new-external");
        reset_network_alert_state_for_tests();
    }

    #[test]
    fn known_destinations_suppress_new_external_alerts_when_requested() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "external-known".to_string(),
            name: "Destinos nuevos".to_string(),
            enabled: true,
            condition: AlertCondition::NewExternalConnection {
                exclude_known: true,
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        }];

        let mut snapshot = sample();
        snapshot.recent_connections = vec![ProcessConnectionEvent {
            pid: 7,
            protocol: TransportProtocol::Tcp,
            direction: TrafficDirection::Outbound,
            src_ip: "10.0.0.7".to_string(),
            dst_ip: "1.1.1.1".to_string(),
            src_port: 55_000,
            dst_port: 443,
            bytes: 256,
        }];

        let warmup_rules = vec![NetworkAlertRule {
            id: "warmup".to_string(),
            name: "Warmup".to_string(),
            enabled: true,
            condition: AlertCondition::NewExternalConnection {
                exclude_known: false,
            },
            severity: AlertSeverity::Info,
            cooldown_seconds: 0,
            notify_ai: false,
        }];

        assert!(
            evaluate_network_alerts(&snapshot, None, &warmup_rules, &[], &mut state).is_empty()
        );
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let first = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert!(first.is_empty());

        snapshot.captured_at_unix_ms += 2_000;
        let second = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert!(second.is_empty());
    }

    #[test]
    fn suspicious_destination_rule_matches_regex_patterns() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "dest-regex".to_string(),
            name: "Destino regex".to_string(),
            enabled: true,
            condition: AlertCondition::SuspiciousDestination {
                patterns: vec!["198\\.51\\.100\\..*".to_string()],
            },
            severity: AlertSeverity::Critical,
            cooldown_seconds: 0,
            notify_ai: true,
        }];

        let mut snapshot = sample();
        snapshot.recent_connections = vec![ProcessConnectionEvent {
            pid: 11,
            protocol: TransportProtocol::Udp,
            direction: TrafficDirection::Outbound,
            src_ip: "10.0.0.11".to_string(),
            dst_ip: "198.51.100.24".to_string(),
            src_port: 60_000,
            dst_port: 53,
            bytes: 128,
        }];

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].destination.as_deref(), Some("198.51.100.24:53"));
        assert!(alerts[0].notify_ai);
    }

    #[test]
    fn process_spike_uses_history_baseline() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "spike".to_string(),
            name: "Spike".to_string(),
            enabled: true,
            condition: AlertCondition::ProcessNetworkSpike {
                process_name: "chrome".to_string(),
                multiplier: 3.0,
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        }];

        let mut history = Vec::new();
        for offset in 0..5_u128 {
            let mut prev = sample();
            prev.captured_at_unix_ms = 1_000 + offset;
            prev.process_throughput[0].rx_bytes_per_sec = 200_000;
            prev.process_throughput[0].tx_bytes_per_sec = 200_000;
            history.push(prev);
        }

        let mut snapshot = sample();
        snapshot.process_throughput[0].rx_bytes_per_sec = 2_000_000;
        snapshot.process_throughput[0].tx_bytes_per_sec = 2_000_000;

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &history, &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &history, &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &history, &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].process_name.as_deref(), Some("chrome"));
        assert!(alerts[0].bandwidth_mbps.unwrap_or_default() > 0.0);
    }

    #[test]
    fn connection_count_rule_can_target_specific_process() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "conn-count".to_string(),
            name: "Connection count".to_string(),
            enabled: true,
            condition: AlertCondition::ConnectionCountExceeded {
                max_connections: 2,
                process: Some("chrome".to_string()),
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        }];

        let mut snapshot = sample();
        snapshot.process_throughput.push(ProcessNetworkThroughput {
            pid: 77,
            process_name: Some("curl".to_string()),
            rx_bytes_per_sec: 0,
            tx_bytes_per_sec: 0,
            tcp_packets_per_sec: 0,
            udp_packets_per_sec: 0,
        });
        snapshot.recent_connections = vec![
            make_event(42, "8.8.8.8", 443),
            make_event(42, "8.8.4.4", 443),
            make_event(42, "1.1.1.1", 80),
            make_event(77, "9.9.9.9", 53),
        ];

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].pid, Some(42));
        assert_eq!(alerts[0].connection_count, Some(3));
    }

    #[test]
    fn disabled_rules_and_rule_clear_reset_consecutive_matches() {
        let mut state = EvaluatorState::new();
        let disabled_rule = NetworkAlertRule {
            id: "disabled".to_string(),
            name: "Disabled".to_string(),
            enabled: false,
            condition: AlertCondition::UnusualPort {
                suspicious_ports: vec![4444],
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        };

        let mut snapshot = sample();
        snapshot.recent_connections = vec![make_event(42, "8.8.8.8", 4444)];
        assert!(
            evaluate_network_alerts(&snapshot, None, &[disabled_rule], &[], &mut state).is_empty()
        );

        let rule = NetworkAlertRule {
            id: "toggle".to_string(),
            name: "Toggle".to_string(),
            enabled: true,
            condition: AlertCondition::UnusualPort {
                suspicious_ports: vec![4444],
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        };

        assert!(evaluate_network_alerts(
            &snapshot,
            None,
            std::slice::from_ref(&rule),
            &[],
            &mut state
        )
        .is_empty());
        let no_match_snapshot = sample();
        assert!(evaluate_network_alerts(
            &no_match_snapshot,
            None,
            std::slice::from_ref(&rule),
            &[],
            &mut state
        )
        .is_empty());
        let second_match = evaluate_network_alerts(&snapshot, None, &[rule], &[], &mut state);
        assert!(second_match.is_empty());
    }

    #[test]
    fn connection_count_without_matching_process_or_regexless_destination_do_not_alert() {
        let mut state = EvaluatorState::new();
        let count_rule = NetworkAlertRule {
            id: "conn-specific".to_string(),
            name: "Specific process".to_string(),
            enabled: true,
            condition: AlertCondition::ConnectionCountExceeded {
                max_connections: 1,
                process: Some("firefox".to_string()),
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 0,
            notify_ai: false,
        };

        let mut snapshot = sample();
        snapshot.recent_connections = vec![
            make_event(42, "8.8.8.8", 443),
            make_event(42, "1.1.1.1", 80),
        ];
        assert!(
            evaluate_network_alerts(&snapshot, None, &[count_rule], &[], &mut state).is_empty()
        );

        let regex_rule = NetworkAlertRule {
            id: "bad-regex".to_string(),
            name: "Bad regex".to_string(),
            enabled: true,
            condition: AlertCondition::SuspiciousDestination {
                patterns: vec!["[".to_string()],
            },
            severity: AlertSeverity::Critical,
            cooldown_seconds: 0,
            notify_ai: false,
        };
        assert!(
            evaluate_network_alerts(&snapshot, None, &[regex_rule], &[], &mut state).is_empty()
        );
    }

    #[test]
    fn helpers_cover_direction_rounding_and_external_ip_logic() {
        assert_eq!(direction_label(Direction::Upload), "upload");
        assert_eq!(direction_label(Direction::Download), "download");
        assert_eq!(direction_label(Direction::Both), "trafico total");
        assert_eq!(round2(1.234), 1.23);
        assert_eq!(round2(1.235), 1.24);

        assert!(is_external_ip("8.8.8.8"));
        assert!(!is_external_ip("127.0.0.1"));
        assert!(!is_external_ip("10.0.0.1"));
        assert!(!is_external_ip("::1"));
        assert!(!is_external_ip("not-an-ip"));
    }

    #[test]
    fn cooldown_allows_retrigger_after_window_expires() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "cooldown-expire".to_string(),
            name: "Cooldown expire".to_string(),
            enabled: true,
            condition: AlertCondition::UnusualPort {
                suspicious_ports: vec![4444],
            },
            severity: AlertSeverity::Warning,
            cooldown_seconds: 5,
            notify_ai: false,
        }];

        let mut snapshot = sample();
        snapshot.recent_connections = vec![make_event(42, "8.8.8.8", 4444)];

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert_eq!(
            evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).len(),
            1
        );

        snapshot.captured_at_unix_ms += 6_000;
        assert_eq!(
            evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).len(),
            1
        );
    }

    #[test]
    fn high_bandwidth_system_rule_covers_download_path() {
        let mut state = EvaluatorState::new();
        let rules = vec![NetworkAlertRule {
            id: "system-download".to_string(),
            name: "System download".to_string(),
            enabled: true,
            condition: AlertCondition::HighBandwidth {
                threshold_mbps: 10.0,
                direction: Direction::Download,
                process: None,
            },
            severity: AlertSeverity::Info,
            cooldown_seconds: 0,
            notify_ai: false,
        }];

        let mut snapshot = sample();
        snapshot.net_rx_bytes_per_sec = 5_000_000;

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &[], &mut state);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].process_name, None);
        assert!(alerts[0].message.contains("download"));
    }

    fn make_event(pid: u32, dst_ip: &str, dst_port: u16) -> ProcessConnectionEvent {
        ProcessConnectionEvent {
            pid,
            protocol: TransportProtocol::Tcp,
            direction: TrafficDirection::Outbound,
            src_ip: "10.0.0.10".to_string(),
            dst_ip: dst_ip.to_string(),
            src_port: 50_000,
            dst_port,
            bytes: 512,
        }
    }
}
