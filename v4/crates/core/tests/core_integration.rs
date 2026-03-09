use core::{
    ai, audit, audit_trail, cloud, crypto, killer, metrics, network, network_alerts,
    process_identity, rate_limit, rules_engine, security, telemetry, watcher,
};
use std::sync::{Mutex, OnceLock};

fn rules_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD.get_or_init(|| Mutex::new(())).lock().unwrap()
}

// ===========================================================================
// Metrics Module
// ===========================================================================

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
fn integration_top_processes_zero_limit() {
    let top = metrics::top_processes_by_memory(0);
    assert!(top.is_empty());
}

#[test]
fn integration_top_processes_large_limit_does_not_exceed_total() {
    let top = metrics::top_processes_by_memory(100_000);
    // Should return all processes but not crash
    assert!(!top.is_empty());
}

#[test]
fn integration_super_process_aggregation_is_available() {
    let grouped = metrics::aggregate_super_processes(Some(20));
    assert!(grouped.len() <= 20);
    assert!(grouped.iter().all(|sp| sp.process_count >= 1));
}

#[test]
fn integration_super_processes_pids_match_count() {
    let grouped = metrics::aggregate_super_processes(Some(50));
    for sp in &grouped {
        assert_eq!(
            sp.pids.len(),
            sp.process_count,
            "PIDs vec length should match process_count for {}",
            sp.display_name
        );
    }
}

#[test]
fn integration_super_processes_with_network_data() {
    let network = vec![network::ProcessNetworkThroughput {
        pid: std::process::id(),
        process_name: Some("self".to_string()),
        rx_bytes_per_sec: 2048,
        tx_bytes_per_sec: 4096,
        tcp_packets_per_sec: 5,
        udp_packets_per_sec: 1,
    }];
    let grouped = metrics::aggregate_super_processes_with_network(&network, Some(100));
    assert!(!grouped.is_empty());
}

#[test]
fn integration_energy_impact_zero_activity_returns_none() {
    let score = metrics::estimate_energy_impact(0.0, 0, 0, 0, 0, 0);
    assert!(score.is_none());
}

#[test]
fn integration_energy_impact_capped_at_1000() {
    let score = metrics::estimate_energy_impact(
        10000.0,
        u64::MAX / 2,
        u64::MAX / 2,
        u64::MAX / 2,
        u64::MAX / 2,
        u64::MAX / 2,
    );
    assert!(score.unwrap_or(0.0) <= 1000.0);
}

#[test]
fn integration_process_telemetry_snapshot_has_entries() {
    let telemetry = metrics::snapshot_process_telemetry();
    assert!(!telemetry.is_empty());
    // Every process should have a non-empty name
    assert!(telemetry.iter().all(|p| !p.name.is_empty()));
}

// ===========================================================================
// Network Module
// ===========================================================================

#[test]
fn integration_network_engine_observes_local_traffic() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");

    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 4096];
        let _ = socket.read(&mut buf);
    });

    let mut engine = network::NetworkTelemetryEngine::new();
    std::thread::sleep(std::time::Duration::from_millis(300));

    let mut client = TcpStream::connect(addr).expect("connect to listener");
    client.write_all(&vec![7u8; 4096]).expect("write traffic");
    let _ = server.join();

    std::thread::sleep(std::time::Duration::from_millis(350));
    let sample = engine.sample();
    assert!(sample.observed_interval_ms >= 1);
    assert!(!sample.backend_label.is_empty());
}

#[test]
fn integration_network_engine_multiple_samples_do_not_panic() {
    let mut engine = network::NetworkTelemetryEngine::new();
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let sample = engine.sample();
        assert!(sample.observed_interval_ms >= 1);
    }
}

#[test]
fn integration_network_capture_backend_labels() {
    assert_eq!(network::NetworkCaptureBackend::Ebpf.as_str(), "eBPF");
    assert_eq!(
        network::NetworkCaptureBackend::PacketFilter.as_str(),
        "Packet Filter (libpcap)"
    );
    assert_eq!(
        network::NetworkCaptureBackend::WinDivert.as_str(),
        "WinDivert"
    );
    assert_eq!(
        network::NetworkCaptureBackend::Unsupported.as_str(),
        "Unsupported"
    );
}

#[test]
fn integration_network_flow_sample_serializes() {
    let mut engine = network::NetworkTelemetryEngine::new();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let sample = engine.sample();
    let json = serde_json::to_string(&sample).expect("serialize NetworkFlowSample");
    assert!(json.contains("backend_label"));
    assert!(json.contains("observed_interval_ms"));
}

#[test]
fn integration_network_snapshot_filter_alert_pipeline() {
    use core::network_alerts::{
        evaluate_network_alerts, AlertCondition, AlertSeverity, Direction, NetworkAlertRule,
    };
    use core::network_analysis::{
        ConnectionState, NetworkConnection, NetworkFilter, NetworkSnapshot, ProcessNetworkSummary,
        Protocol,
    };
    use std::net::{IpAddr, Ipv4Addr};

    let connection = NetworkConnection {
        pid: 4242,
        process_name: "chrome".to_string(),
        protocol: Protocol::TCP,
        local_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10)),
        local_port: 51_234,
        remote_addr: IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        remote_port: 443,
        remote_hostname: Some("example.com".to_string()),
        state: ConnectionState::Established,
        bytes_sent: 10_000,
        bytes_received: 20_000,
        bytes_per_sec_up: 2_000_000.0,
        bytes_per_sec_down: 1_000_000.0,
        established_at: 1,
        country: Some("US".to_string()),
        is_encrypted: Some(true),
    };

    let analysis_snapshot = NetworkSnapshot {
        timestamp: 100,
        connections: vec![connection.clone()],
        total_bytes_up: connection.bytes_sent,
        total_bytes_down: connection.bytes_received,
        total_bytes_per_sec_up: connection.bytes_per_sec_up,
        total_bytes_per_sec_down: connection.bytes_per_sec_down,
        active_connections: 1,
        per_process_summary: vec![ProcessNetworkSummary {
            pid: 4242,
            name: "chrome".to_string(),
            connection_count: 1,
            total_up: connection.bytes_per_sec_up,
            total_down: connection.bytes_per_sec_down,
            top_remote: Some("example.com".to_string()),
            protocols: vec![Protocol::TCP],
        }],
    };

    let filter = NetworkFilter {
        protocols: Some(vec![Protocol::TCP]),
        ports: Some(vec![443]),
        process_names: Some(vec!["chrome".to_string()]),
        remote_hosts: Some(vec!["example".to_string()]),
        only_established: true,
        ..Default::default()
    };

    let filtered = filter.apply(&analysis_snapshot.connections);
    assert_eq!(filtered.len(), 1);

    let mut flow_snapshot = network::NetworkFlowSample {
        backend: network::NetworkCaptureBackend::Unsupported,
        backend_label: "Unsupported".to_string(),
        privileged_path_available: false,
        deep_packet_inspection_active: false,
        net_rx_bytes_per_sec: 1_000_000,
        net_tx_bytes_per_sec: 2_000_000,
        observed_interval_ms: 2_000,
        process_throughput: vec![network::ProcessNetworkThroughput {
            pid: 4242,
            process_name: Some("chrome".to_string()),
            rx_bytes_per_sec: 1_000_000,
            tx_bytes_per_sec: 2_000_000,
            tcp_packets_per_sec: 10,
            udp_packets_per_sec: 0,
        }],
        recent_connections: vec![network::ProcessConnectionEvent {
            pid: 4242,
            protocol: network::TransportProtocol::Tcp,
            direction: network::TrafficDirection::Outbound,
            src_ip: "10.0.0.10".to_string(),
            dst_ip: "93.184.216.34".to_string(),
            src_port: 51_234,
            dst_port: 443,
            bytes: 4_096,
        }],
        capture_windows_dropped: 0,
        captured_at_unix_ms: 10_000,
    };

    let rules = vec![NetworkAlertRule {
        id: "pipeline-bandwidth".to_string(),
        name: "Pipeline bandwidth".to_string(),
        enabled: true,
        condition: AlertCondition::HighBandwidth {
            threshold_mbps: 10.0,
            direction: Direction::Both,
            process: Some("chrome".to_string()),
        },
        severity: AlertSeverity::Warning,
        cooldown_seconds: 0,
        notify_ai: false,
    }];

    assert!(evaluate_network_alerts(&flow_snapshot, None, &rules, &[]).is_empty());
    flow_snapshot.captured_at_unix_ms += 2_000;
    assert!(evaluate_network_alerts(&flow_snapshot, None, &rules, &[]).is_empty());
    flow_snapshot.captured_at_unix_ms += 2_000;
    let alerts = evaluate_network_alerts(&flow_snapshot, None, &rules, &[]);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].process_name.as_deref(), Some("chrome"));
    assert!(alerts[0].message.contains("chrome"));
}

// ===========================================================================
// Rules Engine
// ===========================================================================

#[test]
fn integration_rules_engine_matches_country_rule() {
    let rules_json = r#"{
        "schema_version": 1,
        "rules": [
            {
                "id": "cn-rule",
                "name": "CN outbound",
                "enabled": true,
                "kind": "process_country",
                "process_contains": "chrome",
                "country_code": "CN",
                "destination_ip": null,
                "destination_cidr": null,
                "destination_port": null,
                "protocol": "tcp",
                "process_memory_mb_gt": null,
                "mitre_technique_id": "T1571"
            }
        ]
    }"#;
    let loaded = rules_engine::upsert_rules_from_ai_json(rules_json).expect("load rules");
    assert_eq!(loaded, 1);

    let events = vec![network::ProcessConnectionEvent {
        pid: 777,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.1".to_string(),
        dst_ip: "36.5.10.2".to_string(),
        src_port: 51515,
        dst_port: 443,
        bytes: 256,
    }];

    let runtime = vec![rules_engine::ProcessRuntime {
        pid: 777,
        process_name: "chrome renderer".to_string(),
        memory_bytes: 400 * 1_048_576,
    }];
    let alerts = rules_engine::evaluate_events(&events, &runtime);
    assert_eq!(alerts.len(), 1);
}

#[test]
fn integration_rules_engine_ip_rule() {
    let payload = r#"{"schema_version":1,"rules":[{"id":"ip-rule","name":"IP match","enabled":true,"kind":"process_ip","process_contains":null,"country_code":null,"destination_ip":"1.2.3.4","destination_cidr":null,"destination_port":null,"protocol":"any","process_memory_mb_gt":null,"mitre_technique_id":"T1071"}]}"#;
    rules_engine::upsert_rules_from_ai_json(payload).expect("load IP rule");

    let events = vec![network::ProcessConnectionEvent {
        pid: 80,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "192.168.1.1".to_string(),
        dst_ip: "1.2.3.4".to_string(),
        src_port: 50000,
        dst_port: 80,
        bytes: 100,
    }];
    let runtime = vec![rules_engine::ProcessRuntime {
        pid: 80,
        process_name: "curl".to_string(),
        memory_bytes: 10 * 1_048_576,
    }];
    let alerts = rules_engine::evaluate_events(&events, &runtime);
    assert!(
        alerts.iter().any(|a| a.rule_id == "ip-rule"),
        "IP rule should match exact destination IP"
    );
}

#[test]
fn integration_rules_engine_rejects_invalid_schema_version() {
    let _guard = rules_test_guard();
    let payload = r#"{"schema_version":99,"rules":[]}"#;
    let result = rules_engine::upsert_rules_from_ai_json(payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unsupported schema_version"));
}

#[test]
fn integration_rules_engine_rejects_invalid_json() {
    let _guard = rules_test_guard();
    let result = rules_engine::upsert_rules_from_ai_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn integration_rules_engine_schema_contains_all_kinds() {
    let _guard = rules_test_guard();
    let schema = rules_engine::ai_rules_schema_json();
    assert!(schema.contains("process_country"));
    assert!(schema.contains("process_ip"));
    assert!(schema.contains("process_cidr"));
    assert!(schema.contains("process_port"));
    assert!(schema.contains("process_memory"));
}

#[test]
fn integration_rules_engine_active_rules_returns_loaded() {
    let _guard = rules_test_guard();
    let payload = r#"{"schema_version":1,"rules":[{"id":"active-test","name":"Active Test","enabled":true,"kind":"process_port","process_contains":null,"country_code":null,"destination_ip":null,"destination_cidr":null,"destination_port":8080,"protocol":"any","process_memory_mb_gt":null,"mitre_technique_id":"T1571"}]}"#;
    rules_engine::upsert_rules_from_ai_json(payload).expect("load rule");
    let rules = rules_engine::active_rules();
    assert!(rules.iter().any(|r| r.id == "active-test"));
}

#[test]
fn integration_rules_engine_remove_rule() {
    let _guard = rules_test_guard();
    let payload = r#"{"schema_version":1,"rules":[{"id":"removable","name":"To Remove","enabled":true,"kind":"process_port","process_contains":null,"country_code":null,"destination_ip":null,"destination_cidr":null,"destination_port":9999,"protocol":"any","process_memory_mb_gt":null,"mitre_technique_id":"T1571"}]}"#;
    rules_engine::upsert_rules_from_ai_json(payload).expect("load rule");
    let removed = rules_engine::remove_rule_by_id("removable").expect("remove rule");
    assert!(removed);
    // Removing again should return false
    let removed_again = rules_engine::remove_rule_by_id("removable").expect("remove again");
    assert!(!removed_again);
}

#[test]
fn integration_rules_geoip_replace_db() {
    let _guard = rules_test_guard();
    let geo_json = r#"[{"cidr":"100.0.0.0/8","country_code":"RU"}]"#;
    let count = rules_engine::replace_geoip_db_from_json(geo_json).expect("replace geo db");
    assert_eq!(count, 1);
}

#[test]
fn integration_rules_geoip_invalid_json() {
    let _guard = rules_test_guard();
    let result = rules_engine::replace_geoip_db_from_json("bad json");
    assert!(result.is_err());
}

// ===========================================================================
// Watcher Module
// ===========================================================================

#[test]
fn integration_watcher_cache_is_readable() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2300));
    let state = watcher::get_cached_state();
    assert!(state.total_memory_bytes > 0);
    assert!(state.updated_at_unix_ms > 0);
    assert!(!state.net_capture_backend.is_empty());
    let _ = state.net_dpi_active;
}

#[test]
fn integration_watcher_idempotent_start() {
    // Calling start_watcher multiple times should be a no-op
    watcher::start_watcher();
    watcher::start_watcher();
    watcher::start_watcher();
    // Should still work fine
    let state = watcher::get_cached_state();
    // State may or may not be populated depending on timing, but should not panic
    let _ = state.total_memory_bytes;
}

#[test]
fn integration_watcher_state_has_processes() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let state = watcher::get_cached_state();
    assert!(
        !state.cached_process_info.is_empty(),
        "watcher should cache at least one process"
    );
}

#[test]
fn integration_watcher_state_serializes() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let state = watcher::get_cached_state();
    let json = serde_json::to_string(&state).expect("serialize SystemState");
    assert!(json.contains("total_memory_bytes"));
    assert!(json.contains("cached_process_info"));
}

// ===========================================================================
// Killer Module
// ===========================================================================

#[test]
fn integration_killer_rejects_invalid_pid() {
    let result = killer::kill_process_safe(0, &[]);
    assert!(matches!(result, Err(killer::KillError::InvalidPid(0))));
}

#[test]
fn integration_killer_rejects_pid_one() {
    let result = killer::kill_process_safe(1, &[]);
    assert!(matches!(result, Err(killer::KillError::InvalidPid(1))));
}

#[test]
fn integration_killer_rejects_negative_pid() {
    let result = killer::kill_process_safe(-5, &[]);
    assert!(matches!(result, Err(killer::KillError::InvalidPid(-5))));
}

#[test]
fn integration_killer_process_not_found() {
    // Very high PID that shouldn't exist
    let result = killer::kill_process_safe(4_000_000, &[]);
    assert!(matches!(
        result,
        Err(killer::KillError::ProcessNotFound(4_000_000))
    ));
}

#[test]
fn integration_killer_blocked_process_names() {
    assert!(killer::is_immutable_blocked_process_name("launchd"));
    assert!(killer::is_immutable_blocked_process_name("kernel_task"));
    assert!(killer::is_immutable_blocked_process_name("Launchd")); // case-insensitive
    assert!(killer::is_immutable_blocked_process_name("KERNEL_TASK"));
    assert!(!killer::is_immutable_blocked_process_name("firefox"));
    assert!(!killer::is_immutable_blocked_process_name("chrome"));
    assert!(!killer::is_immutable_blocked_process_name(""));
}

#[test]
fn integration_killer_terminates_spawned_child() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn sleep child");
    let pid = child.id() as i32;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    std::thread::sleep(std::time::Duration::from_millis(200));
    let result = killer::kill_process_safe(pid, &[]);
    assert!(result.is_ok(), "expected kill success, got: {result:?}");
}

// ===========================================================================
// Security Module
// ===========================================================================

#[test]
fn integration_security_mitre_mapping_works() {
    let observations = vec![security::ProcessBehaviorObservation {
        pid: 991,
        process_name: "injector.exe".to_string(),
        indicator: security::BehaviorIndicator::DllInjection,
        detail: Some("LoadLibrary remote thread".to_string()),
    }];

    let labels = security::label_process_observations(&observations);
    assert_eq!(labels.len(), 1);
    assert!(labels[0].mitre_techniques[0]
        .technique_id
        .starts_with("T1055"));
}

#[test]
fn integration_security_all_indicators_map_to_mitre() {
    let indicators = vec![
        security::BehaviorIndicator::DllInjection,
        security::BehaviorIndicator::RemoteThreadInjection,
        security::BehaviorIndicator::ProcessHollowing,
        security::BehaviorIndicator::SuspiciousMemoryRead,
        security::BehaviorIndicator::UnsignedModuleLoad,
        security::BehaviorIndicator::SuspiciousNetworkConnection,
    ];

    for indicator in indicators {
        let techniques = security::map_behavior_to_mitre(&indicator);
        assert!(
            !techniques.is_empty(),
            "{indicator:?} should map to at least one MITRE technique"
        );
        for t in &techniques {
            assert!(
                t.technique_id.starts_with('T'),
                "technique_id should start with T: {}",
                t.technique_id
            );
        }
    }
}

#[test]
fn integration_security_network_policy_detects_blocked_ips() {
    let policy = security::NetworkPolicy::default();
    assert!(!policy.blocked_ips.is_empty());
    assert!(!policy.unusual_ports.is_empty());

    let events = vec![network::ProcessConnectionEvent {
        pid: 42,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.1".to_string(),
        dst_ip: policy.blocked_ips[0].clone(),
        src_port: 50000,
        dst_port: 443,
        bytes: 128,
    }];

    let observations = security::evaluate_network_events(&events, &policy);
    assert_eq!(observations.len(), 1);
    assert_eq!(
        observations[0].indicator,
        security::BehaviorIndicator::SuspiciousNetworkConnection
    );
}

#[test]
fn integration_security_network_policy_detects_unusual_ports() {
    let policy = security::NetworkPolicy::default();
    let events = vec![network::ProcessConnectionEvent {
        pid: 43,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.1".to_string(),
        dst_ip: "8.8.8.8".to_string(), // not blocked
        src_port: 50000,
        dst_port: 4444, // unusual port
        bytes: 64,
    }];

    let observations = security::evaluate_network_events(&events, &policy);
    assert_eq!(observations.len(), 1);
}

#[test]
fn integration_security_clean_traffic_has_no_observations() {
    let policy = security::NetworkPolicy::default();
    let events = vec![network::ProcessConnectionEvent {
        pid: 44,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.1".to_string(),
        dst_ip: "8.8.8.8".to_string(),
        src_port: 50000,
        dst_port: 443, // normal port, normal IP
        bytes: 64,
    }];

    let observations = security::evaluate_network_events(&events, &policy);
    assert!(
        observations.is_empty(),
        "clean traffic should produce no observations"
    );
}

#[test]
fn integration_security_confidence_with_detail() {
    let observations = vec![security::ProcessBehaviorObservation {
        pid: 10,
        process_name: "test.exe".to_string(),
        indicator: security::BehaviorIndicator::ProcessHollowing,
        detail: Some("NtUnmapViewOfSection + WriteProcessMemory".to_string()),
    }];
    let labels = security::label_process_observations(&observations);
    assert!(labels[0].confidence > 0.8);
}

#[test]
fn integration_security_confidence_without_detail() {
    let observations = vec![security::ProcessBehaviorObservation {
        pid: 11,
        process_name: "test2.exe".to_string(),
        indicator: security::BehaviorIndicator::UnsignedModuleLoad,
        detail: None,
    }];
    let labels = security::label_process_observations(&observations);
    assert!(labels[0].confidence <= 0.8);
}

// ===========================================================================
// Audit Module (CVE + Heartbeat)
// ===========================================================================

#[test]
fn integration_cve_audit_detects_affected_version() {
    let db = audit::LocalCveDatabase {
        schema_version: 1,
        entries: vec![audit::CveEntry {
            cve_id: "CVE-2026-1234".to_string(),
            product: "openssl".to_string(),
            affected_version_reqs: vec!["<3.0.14".to_string()],
            severity: Some("critical".to_string()),
            summary: Some("Example CVE".to_string()),
        }],
    };

    let processes = vec![audit::ProcessVersionInfo {
        pid: 44,
        process_name: "openssl-worker".to_string(),
        product: "OpenSSL".to_string(),
        version: "3.0.12".to_string(),
    }];

    let matches = audit::audit_processes_against_cves(&processes, &db);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].cve_id, "CVE-2026-1234");
}

#[test]
fn integration_cve_audit_skips_unaffected_version() {
    let db = audit::LocalCveDatabase {
        schema_version: 1,
        entries: vec![audit::CveEntry {
            cve_id: "CVE-2026-5555".to_string(),
            product: "nginx".to_string(),
            affected_version_reqs: vec!["<1.25.0".to_string()],
            severity: Some("high".to_string()),
            summary: Some("Buffer overflow".to_string()),
        }],
    };

    let processes = vec![audit::ProcessVersionInfo {
        pid: 45,
        process_name: "nginx".to_string(),
        product: "nginx".to_string(),
        version: "1.26.0".to_string(), // not affected
    }];

    let matches = audit::audit_processes_against_cves(&processes, &db);
    assert!(matches.is_empty());
}

#[test]
fn integration_cve_audit_skips_non_semver() {
    let db = audit::LocalCveDatabase {
        schema_version: 1,
        entries: vec![audit::CveEntry {
            cve_id: "CVE-2026-9999".to_string(),
            product: "chrome".to_string(),
            affected_version_reqs: vec!["<124.0.0".to_string()],
            severity: None,
            summary: None,
        }],
    };

    let processes = vec![audit::ProcessVersionInfo {
        pid: 46,
        process_name: "Chrome".to_string(),
        product: "chrome".to_string(),
        version: "beta-channel".to_string(),
    }];

    let matches = audit::audit_processes_against_cves(&processes, &db);
    assert!(matches.is_empty());
}

#[test]
fn integration_cve_db_from_json_str() {
    let json = r#"{"schema_version":1,"entries":[{"cve_id":"CVE-2026-0001","product":"test","affected_version_reqs":["<1.0.0"],"severity":"low","summary":"test cve"}]}"#;
    let db = audit::LocalCveDatabase::from_json_str(json).expect("parse CVE db");
    assert_eq!(db.entries.len(), 1);
    assert_eq!(db.entries[0].cve_id, "CVE-2026-0001");
}

#[test]
fn integration_cve_db_from_invalid_json() {
    let result = audit::LocalCveDatabase::from_json_str("not json");
    assert!(result.is_err());
}

#[test]
fn integration_heartbeat_nist_compliance() {
    let heartbeat = audit::build_security_heartbeat(42, 1, true, 2, 3, true, "investigating");
    assert!(heartbeat.generated_at_unix_ms > 0);
    assert_eq!(
        heartbeat.nist_control_family,
        "NIST 800-53 (Identification, Monitoring, Response)"
    );
    assert_eq!(heartbeat.identification.tracked_processes, 42);
    assert!(heartbeat.identification.asset_inventory_complete);
    assert!(heartbeat.monitoring.dpi_active);
    assert_eq!(heartbeat.monitoring.suspicious_connection_count, 2);
    assert!(heartbeat.response.encrypted_audit_trail_enabled);
}

#[test]
fn integration_heartbeat_json_serialization() {
    let heartbeat = audit::build_security_heartbeat(10, 0, false, 0, 0, true, "idle");
    let json = audit::security_heartbeat_json(&heartbeat).expect("heartbeat json");
    assert!(json.contains("NIST 800-53"));
    assert!(json.contains("idle"));
}

#[test]
fn integration_persists_encrypted_security_heartbeat() {
    let heartbeat = audit::build_security_heartbeat(42, 1, true, 2, 3, true, "investigating");
    let key = [11u8; 32];
    let path =
        std::env::temp_dir().join(format!("omnimon-heartbeat-int-{}.enc", std::process::id()));

    audit::persist_encrypted_security_heartbeat(&path, &key, &heartbeat)
        .expect("persist heartbeat");

    let data = std::fs::read_to_string(&path).expect("read heartbeat file");
    assert!(!data.trim().is_empty());

    let json = audit::security_heartbeat_json(&heartbeat).expect("heartbeat json");
    assert!(json.contains("NIST 800-53"));
}

// ===========================================================================
// Audit Trail
// ===========================================================================

#[test]
fn integration_security_audit_trail_rotates_encrypted_logs() {
    let dir = std::env::temp_dir().join(format!("omnimon-int-audit-{}", std::process::id()));
    let trail = audit_trail::EncryptedAuditTrail::new(&dir, 200, 3);
    let key = [88u8; 32];

    let observation = security::ProcessBehaviorObservation {
        pid: 1001,
        process_name: "injector.exe".to_string(),
        indicator: security::BehaviorIndicator::DllInjection,
        detail: Some("CreateRemoteThread + LoadLibrary".to_string()),
    };

    for _ in 0..8 {
        let labels = security::label_and_record_observations(
            std::slice::from_ref(&observation),
            &trail,
            &key,
        )
        .expect("label and record");
        assert_eq!(labels.len(), 1);
    }

    let active = dir.join("mitre-alerts.log.enc");
    assert!(active.exists());
}

#[test]
fn integration_audit_trail_writes_and_reads_records() {
    let dir = std::env::temp_dir().join(format!("omnimon-int-trail-rw-{}", std::process::id()));
    let trail = audit_trail::EncryptedAuditTrail::new(&dir, 4096, 5);
    let key = [55u8; 32];

    let label = security::ProcessThreatLabel {
        pid: 500,
        process_name: "test-proc".to_string(),
        indicator: security::BehaviorIndicator::SuspiciousMemoryRead,
        mitre_techniques: vec![security::MitreTechnique {
            technique_id: "T1003".to_string(),
            tactic: "Credential Access".to_string(),
            name: "OS Credential Dumping".to_string(),
        }],
        confidence: 0.85,
        context: Some("ReadProcessMemory on lsass.exe".to_string()),
    };

    trail.append_label(&key, &label).expect("append label");
    let active = dir.join("mitre-alerts.log.enc");
    assert!(active.exists());
    let content = std::fs::read_to_string(active).expect("read file");
    assert!(!content.trim().is_empty());
}

// ===========================================================================
// Crypto Module
// ===========================================================================

#[test]
fn integration_crypto_roundtrip_works() {
    let key = [42u8; 32];
    let payload = serde_json::json!({"telemetry": true, "interval": 2});

    let encrypted = crypto::encrypt_json(&key, &payload).expect("encrypt payload");
    let decrypted: serde_json::Value =
        crypto::decrypt_json(&key, &encrypted).expect("decrypt payload");

    assert_eq!(decrypted, payload);
}

#[test]
fn integration_crypto_bytes_roundtrip() {
    let key = [33u8; 32];
    let plaintext = b"hello world from OmniMon";

    let encrypted = crypto::encrypt_bytes(&key, plaintext).expect("encrypt bytes");
    assert_eq!(encrypted.algorithm, "AES-256-GCM");

    let decrypted = crypto::decrypt_bytes(&key, &encrypted).expect("decrypt bytes");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn integration_crypto_wrong_key_fails() {
    let key_ok = [1u8; 32];
    let key_bad = [2u8; 32];
    let encrypted = crypto::encrypt_bytes(&key_ok, b"secret").expect("encrypt");
    assert!(crypto::decrypt_bytes(&key_bad, &encrypted).is_err());
}

#[test]
fn integration_crypto_ed25519_sign_verify() {
    let (signing_key, verifying_key) = crypto::generate_ed25519_keypair();
    let payload = b"OmniMon v5.2.0 release binary";

    let sig = crypto::sign_release(&signing_key, payload, "v5.2.0");
    assert_eq!(sig.version, "v5.2.0");

    let result = crypto::verify_release(payload, &sig, &verifying_key);
    assert!(result.is_ok());
}

#[test]
fn integration_crypto_tampered_release_fails() {
    let (signing_key, verifying_key) = crypto::generate_ed25519_keypair();
    let payload = b"legit binary";
    let sig = crypto::sign_release(&signing_key, payload, "v1.0.0");

    let tampered = b"malicious binary";
    let result = crypto::verify_release(tampered, &sig, &verifying_key);
    assert!(result.is_err());
}

#[test]
fn integration_crypto_sha256_known_hash() {
    let hash = crypto::sha256_hex(b"hello world");
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn integration_crypto_public_key_export_import() {
    let (_, verifying_key) = crypto::generate_ed25519_keypair();
    let exported = crypto::export_public_key(&verifying_key);
    let imported = crypto::import_public_key(&exported).expect("import");
    assert_eq!(verifying_key.to_bytes(), imported.to_bytes());
}

#[test]
fn integration_crypto_invalid_public_key() {
    assert!(crypto::import_public_key("not-valid!!!").is_err());
}

#[test]
fn integration_crypto_update_manifest_verify() {
    let (signing_key, verifying_key) = crypto::generate_ed25519_keypair();
    let payload = b"update payload";
    let sig = crypto::sign_release(&signing_key, payload, "v5.2.0");

    let manifest = crypto::UpdateManifest {
        version: sig.version,
        sha256: sig.sha256,
        signature_b64: sig.signature_b64,
        download_url: "https://example.com/v5.2.0".to_string(),
    };

    assert!(crypto::verify_update(payload, &manifest, &verifying_key).is_ok());
}

// ===========================================================================
// Process Identity Module
// ===========================================================================

#[test]
fn integration_process_identity_normalize_removes_parenthetical() {
    assert_eq!(
        process_identity::normalize_process_name("Chrome Helper (Renderer)"),
        "chrome helper"
    );
}

#[test]
fn integration_process_identity_normalize_handles_special_chars() {
    assert_eq!(
        process_identity::normalize_process_name("my-daemon_v2.3"),
        "my daemon v2 3"
    );
}

#[test]
fn integration_process_identity_normalize_empty() {
    assert_eq!(process_identity::normalize_process_name(""), "");
}

#[test]
fn integration_process_identity_browser_detection() {
    assert_eq!(
        process_identity::browser_family("Google Chrome", "chrome", None),
        Some("Chrome")
    );
    assert_eq!(
        process_identity::browser_family("Safari", "safari", None),
        Some("Safari")
    );
    assert_eq!(
        process_identity::browser_family("firefox", "firefox", None),
        Some("Firefox")
    );
    assert_eq!(process_identity::browser_family("node", "node", None), None);
}

#[test]
fn integration_process_identity_classify_group_browser() {
    let group = process_identity::classify_group("Chrome Helper", "chrome", None, false);
    assert_eq!(group, "Browser");
}

#[test]
fn integration_process_identity_classify_group_system() {
    let group = process_identity::classify_group("my-daemon", "my-daemon", None, true);
    assert_eq!(group, "System");
}

#[test]
fn integration_process_identity_classify_group_empty() {
    let group = process_identity::classify_group("my-app", "my-app", None, false);
    assert_eq!(group, "");
}

#[test]
fn integration_process_identity_resolve_with_bundle_id() {
    let identity = process_identity::resolve_group_identity(
        "Chrome Helper",
        "Chrome Helper",
        Some("/Applications/Google Chrome.app/Contents/MacOS/Chrome Helper"),
        Some("/applications/google chrome.app"),
        false,
    );
    assert_eq!(identity.key, "browser:chrome");
    assert_eq!(identity.identity_type, "browser_family");
}

#[test]
fn integration_process_identity_resolve_with_exe_path_only() {
    let identity = process_identity::resolve_group_identity(
        "my-daemon",
        "my-daemon",
        Some("/usr/local/bin/my-daemon"),
        None,
        false,
    );
    assert_eq!(identity.identity_type, "exec_name");
    assert!(identity.key.starts_with("exec:"));
}

#[test]
fn integration_process_identity_resolve_name_only() {
    let identity =
        process_identity::resolve_group_identity("mystery", "mystery", None, None, false);
    assert_eq!(identity.identity_type, "normalized_name");
    assert!(identity.key.starts_with("name:"));
}

// ===========================================================================
// Telemetry Module
// ===========================================================================

#[test]
fn integration_telemetry_snapshot_returns_data() {
    // Ensure watcher is started so cache is populated
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let snapshot = telemetry::telemetry_snapshot(Some(10));
    assert!(snapshot.total_memory_bytes > 0);
    assert!(snapshot.processes.len() <= 10);
}

#[test]
fn integration_telemetry_snapshot_no_limit() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));
    let snapshot = telemetry::telemetry_snapshot(None);
    assert!(snapshot.total_memory_bytes > 0);
}

#[test]
fn integration_telemetry_processes_have_group_keys() {
    let snapshot = telemetry::telemetry_snapshot(Some(5));
    for p in &snapshot.processes {
        assert!(!p.group_key.is_empty(), "group_key should not be empty");
        assert!(
            !p.group_identity_type.is_empty(),
            "group_identity_type should not be empty"
        );
    }
}

#[test]
fn integration_telemetry_super_processes_consistent() {
    let snapshot = telemetry::telemetry_snapshot(Some(100));
    for sp in &snapshot.super_processes {
        assert!(sp.process_count >= 1);
        assert!(!sp.binary_key.is_empty());
        assert!(!sp.display_name.is_empty());
    }
}

// ===========================================================================
// Cloud Module (CrabNebula)
// ===========================================================================

#[test]
fn integration_cloud_key_format_validation() {
    assert!(cloud::validate_key_format("cn_live_abc123").is_ok());
    assert!(cloud::validate_key_format("").is_err());
    assert!(cloud::validate_key_format("   ").is_err());
    assert!(cloud::validate_key_format(&"x".repeat(513)).is_err());
    assert!(cloud::validate_key_format("key with spaces").is_err());
    assert!(cloud::validate_key_format("key\nnewline").is_err());
}

#[test]
fn integration_cloud_tier_as_str() {
    assert_eq!(cloud::CloudTier::Free.as_str(), "free");
    assert_eq!(cloud::CloudTier::Premium.as_str(), "premium");
    assert_eq!(cloud::CloudTier::Unknown.as_str(), "unknown");
}

#[test]
fn integration_cloud_validation_serialization() {
    let v = cloud::CloudValidation {
        valid: true,
        tier: cloud::CloudTier::Premium,
        organization: Some("TestOrg".to_string()),
        error: None,
    };
    let json = serde_json::to_string(&v).expect("serialize");
    assert!(json.contains("\"valid\":true"));
    assert!(json.contains("Premium"));

    let deserialized: cloud::CloudValidation = serde_json::from_str(&json).expect("deserialize");
    assert!(deserialized.valid);
    assert_eq!(deserialized.tier, cloud::CloudTier::Premium);
}

#[test]
fn integration_cloud_validate_empty_key() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(cloud::validate_cloud_key(""));
    assert!(!result.valid);
}

#[test]
fn integration_cloud_validate_invalid_format() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(cloud::validate_cloud_key("key with spaces"));
    assert!(!result.valid);
}

#[test]
fn integration_cloud_stored_key_graceful_fallback() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(cloud::validate_stored_cloud_key());
    // No key stored in test env — should fail gracefully
    assert!(!result.valid || result.tier != cloud::CloudTier::Unknown);
}

// ===========================================================================
// Rate Limit Module (Token Bucket IPC Protection)
// ===========================================================================

#[test]
fn integration_rate_limit_allows_within_capacity() {
    let config = rate_limit::BucketConfig::new(5, 100.0);
    for _ in 0..5 {
        assert!(rate_limit::check_rate_limit("int_cap_test", &config).is_ok());
    }
}

#[test]
fn integration_rate_limit_rejects_over_capacity() {
    let config = rate_limit::BucketConfig::new(2, 0.0); // no refill
    assert!(rate_limit::check_rate_limit("int_exhaust", &config).is_ok());
    assert!(rate_limit::check_rate_limit("int_exhaust", &config).is_ok());
    let err = rate_limit::check_rate_limit("int_exhaust", &config);
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("Rate limited"));
}

#[test]
fn integration_rate_limit_separate_buckets_independent() {
    let config = rate_limit::BucketConfig::new(1, 0.0);
    assert!(rate_limit::check_rate_limit("int_bucket_x", &config).is_ok());
    assert!(rate_limit::check_rate_limit("int_bucket_y", &config).is_ok());
    // x exhausted, y still separate
    assert!(rate_limit::check_rate_limit("int_bucket_x", &config).is_err());
}

#[test]
fn integration_rate_limit_refills_over_time() {
    let config = rate_limit::BucketConfig::new(1, 1000.0); // fast refill
    assert!(rate_limit::check_rate_limit("int_refill", &config).is_ok());
    assert!(rate_limit::check_rate_limit("int_refill", &config).is_err());
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(rate_limit::check_rate_limit("int_refill", &config).is_ok());
}

#[test]
fn integration_rate_limit_profiles_are_usable() {
    // Verify all predefined profiles can be used without panic
    assert!(rate_limit::check_rate_limit("int_kill_prof", &rate_limit::profiles::KILL).is_ok());
    assert!(rate_limit::check_rate_limit("int_ai_prof", &rate_limit::profiles::AI).is_ok());
    assert!(
        rate_limit::check_rate_limit("int_browser_prof", &rate_limit::profiles::BROWSER).is_ok()
    );
    assert!(rate_limit::check_rate_limit("int_config_prof", &rate_limit::profiles::CONFIG).is_ok());
}

#[test]
fn integration_rate_limit_error_contains_bucket_name() {
    let config = rate_limit::BucketConfig::new(0, 0.0);
    let err = rate_limit::check_rate_limit("int_named_bucket", &config).unwrap_err();
    assert!(err.contains("int_named_bucket"));
}

// ===========================================================================
// AI Module (Provider parsing, key format)
// ===========================================================================

#[test]
fn integration_ai_provider_from_str() {
    use core::ai::AiProvider;
    use std::str::FromStr;

    assert_eq!(AiProvider::from_str("openai").unwrap(), AiProvider::OpenAI);
    assert_eq!(
        AiProvider::from_str("anthropic").unwrap(),
        AiProvider::Anthropic
    );
    assert_eq!(AiProvider::from_str("gemini").unwrap(), AiProvider::Gemini);
    assert_eq!(
        AiProvider::from_str("openrouter").unwrap(),
        AiProvider::OpenRouter
    );
    assert_eq!(AiProvider::from_str("ollama").unwrap(), AiProvider::Ollama);
    assert!(AiProvider::from_str("invalid").is_err());
}

#[test]
fn integration_ai_provider_properties() {
    use core::ai::AiProvider;

    for provider in [
        AiProvider::OpenAI,
        AiProvider::Anthropic,
        AiProvider::Gemini,
        AiProvider::OpenRouter,
        AiProvider::Ollama,
    ] {
        assert!(!provider.keyring_service().is_empty());
        assert!(!provider.api_url().is_empty());
        assert!(!provider.display_name().is_empty());
    }
}

#[test]
fn integration_ai_ollama_does_not_require_key() {
    use core::ai::AiProvider;
    assert!(!AiProvider::Ollama.requires_api_key());
    assert!(AiProvider::OpenAI.requires_api_key());
    assert!(AiProvider::Anthropic.requires_api_key());
}

// ===========================================================================
// Cross-module Integration
// ===========================================================================

#[test]
fn integration_full_pipeline_watcher_to_telemetry() {
    // Start watcher, wait for data, then query telemetry
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));

    let snapshot = telemetry::telemetry_snapshot(Some(50));
    assert!(snapshot.total_memory_bytes > 0);
    assert!(!snapshot.processes.is_empty());
    assert!(snapshot.total_processes > 0);
}

#[test]
fn integration_security_to_audit_trail_pipeline() {
    let dir = std::env::temp_dir().join(format!("omnimon-int-pipe-{}", std::process::id()));
    let trail = audit_trail::EncryptedAuditTrail::new(&dir, 4096, 3);
    let key = [77u8; 32];

    // Create observations from network events
    let policy = security::NetworkPolicy::default();
    let events = vec![network::ProcessConnectionEvent {
        pid: 300,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.5".to_string(),
        dst_ip: policy.blocked_ips[0].clone(),
        src_port: 52000,
        dst_port: 4444,
        bytes: 256,
    }];

    let observations = security::evaluate_network_events(&events, &policy);
    assert!(!observations.is_empty());

    let labels =
        security::label_and_record_observations(&observations, &trail, &key).expect("record");
    assert!(!labels.is_empty());

    // Verify the trail file exists
    let active = dir.join("mitre-alerts.log.enc");
    assert!(active.exists());
}

#[test]
fn integration_crypto_to_audit_heartbeat_pipeline() {
    let key = [99u8; 32];
    let heartbeat = audit::build_security_heartbeat(100, 5, true, 10, 8, true, "active-response");

    // Encrypt the heartbeat
    let encrypted = crypto::encrypt_json(&key, &heartbeat).expect("encrypt heartbeat");
    assert_eq!(encrypted.algorithm, "AES-256-GCM");

    // Decrypt and verify
    let decrypted: audit::SecurityHeartbeat =
        crypto::decrypt_json(&key, &encrypted).expect("decrypt heartbeat");
    assert_eq!(decrypted.identification.tracked_processes, 100);
    assert_eq!(decrypted.monitoring.mitre_alert_count, 8);
}

#[test]
fn integration_watcher_metrics_to_ai_prompt_pipeline() {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2500));

    let state = watcher::get_cached_state();
    let prompt = ai::build_chat_system_prompt(&state);

    assert!(state.total_memory_bytes > 0);
    assert!(prompt.contains("System State"));
    assert!(prompt.contains("Top Memory/CPU processes"));
}

#[test]
fn integration_security_scan_to_mitre_to_report_pipeline() {
    let policy = security::NetworkPolicy::default();
    let events = vec![network::ProcessConnectionEvent {
        pid: 808,
        protocol: network::TransportProtocol::Tcp,
        direction: network::TrafficDirection::Outbound,
        src_ip: "10.0.0.8".to_string(),
        dst_ip: policy.blocked_ips[0].clone(),
        src_port: 50_808,
        dst_port: policy.unusual_ports[0],
        bytes: 512,
    }];

    let observations = security::evaluate_network_events(&events, &policy);
    let labels = security::label_process_observations(&observations);
    let heartbeat = audit::build_security_heartbeat(
        10,
        0,
        true,
        observations.len(),
        labels.len(),
        true,
        "reported",
    );
    let json = audit::security_heartbeat_json(&heartbeat).expect("heartbeat json");

    assert_eq!(observations.len(), 1);
    assert_eq!(labels.len(), 1);
    assert!(labels[0]
        .mitre_techniques
        .iter()
        .any(|t| t.technique_id == "T1071"));
    assert!(json.contains("NIST 800-53"));
    assert!(json.contains("reported"));
}

#[test]
fn integration_network_alerts_pipeline_emits_after_three_snapshots() {
    network_alerts::reset_network_alert_state_for_tests();
    let rules = vec![network_alerts::NetworkAlertRule {
        id: "suspicious-port".to_string(),
        name: "Puerto sospechoso".to_string(),
        enabled: true,
        condition: network_alerts::AlertCondition::UnusualPort {
            suspicious_ports: vec![4444],
        },
        severity: network_alerts::AlertSeverity::Critical,
        cooldown_seconds: 30,
        notify_ai: true,
    }];

    let mut snapshot = network::NetworkFlowSample {
        backend: network::NetworkCaptureBackend::Unsupported,
        backend_label: "Unsupported".to_string(),
        privileged_path_available: false,
        deep_packet_inspection_active: false,
        net_rx_bytes_per_sec: 0,
        net_tx_bytes_per_sec: 0,
        observed_interval_ms: 2_000,
        process_throughput: vec![network::ProcessNetworkThroughput {
            pid: 77,
            process_name: Some("chrome".to_string()),
            rx_bytes_per_sec: 0,
            tx_bytes_per_sec: 0,
            tcp_packets_per_sec: 0,
            udp_packets_per_sec: 0,
        }],
        recent_connections: vec![network::ProcessConnectionEvent {
            pid: 77,
            protocol: network::TransportProtocol::Tcp,
            direction: network::TrafficDirection::Outbound,
            src_ip: "10.0.0.2".to_string(),
            dst_ip: "8.8.8.8".to_string(),
            src_port: 50_000,
            dst_port: 4444,
            bytes: 128,
        }],
        capture_windows_dropped: 0,
        captured_at_unix_ms: 1_000,
    };

    assert!(network_alerts::evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
    snapshot.captured_at_unix_ms += 2_000;
    assert!(network_alerts::evaluate_network_alerts(&snapshot, None, &rules, &[]).is_empty());
    snapshot.captured_at_unix_ms += 2_000;
    let alerts = network_alerts::evaluate_network_alerts(&snapshot, None, &rules, &[]);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].rule_name, "Puerto sospechoso");
}
