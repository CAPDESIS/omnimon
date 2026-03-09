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

#[derive(Default)]
struct EvaluatorState {
    consecutive_matches: HashMap<String, u32>,
    last_triggered_ms: HashMap<String, u128>,
    known_destinations: HashSet<String>,
}

static RULES: OnceLock<RwLock<Vec<NetworkAlertRule>>> = OnceLock::new();
static EVALUATOR_STATE: OnceLock<RwLock<EvaluatorState>> = OnceLock::new();

fn rules_state() -> &'static RwLock<Vec<NetworkAlertRule>> {
    RULES.get_or_init(|| RwLock::new(Vec::new()))
}

fn evaluator_state() -> &'static RwLock<EvaluatorState> {
    EVALUATOR_STATE.get_or_init(|| RwLock::new(EvaluatorState::default()))
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

pub fn evaluate_active_network_alerts(
    snapshot: &NetworkSnapshot,
    prev_snapshot: Option<&NetworkSnapshot>,
    history: &[NetworkSnapshot],
) -> Vec<NetworkAlert> {
    let rules = active_rules();
    evaluate_network_alerts(snapshot, prev_snapshot, &rules, history)
}

pub fn evaluate_network_alerts(
    snapshot: &NetworkSnapshot,
    prev_snapshot: Option<&NetworkSnapshot>,
    rules: &[NetworkAlertRule],
    history: &[NetworkSnapshot],
) -> Vec<NetworkAlert> {
    let Ok(mut state) = evaluator_state().write() else {
        return Vec::new();
    };

    let mut alerts = Vec::new();
    let mut seen_external_destinations = HashSet::new();

    for event in &snapshot.recent_connections {
        if is_external_ip(&event.dst_ip) {
            seen_external_destinations.insert(destination_key(event));
        }
    }

    for rule in rules.iter().filter(|rule| rule.enabled) {
        let Some(candidate) = check_rule(rule, snapshot, prev_snapshot, history, &state) else {
            clear_rule_debounce(rule, &mut state);
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
            let Some(name) = entry.process_name.as_deref() else {
                return None;
            };
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

pub fn reset_network_alert_state_for_tests() {
    if let Ok(mut guard) = evaluator_state().write() {
        *guard = EvaluatorState::default();
    }
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
        reset_network_alert_state_for_tests();
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

        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
        snapshot.captured_at_unix_ms += 2_000;
        let alerts = evaluate_network_alerts(&snapshot, None, &rules, &[]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "port-watch");
        assert!(alerts[0].notify_ai);
    }

    #[test]
    fn cooldown_suppresses_duplicate_alerts() {
        reset_network_alert_state_for_tests();
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
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
        snapshot.captured_at_unix_ms = 12_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
        snapshot.captured_at_unix_ms = 14_000;
        assert_eq!(
            evaluate_network_alerts(&snapshot, None, &rules, &[]).len(),
            1
        );
        snapshot.captured_at_unix_ms = 16_000;
        assert!(evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
    }
}
