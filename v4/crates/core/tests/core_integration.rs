use core::{audit, crypto, killer, metrics, network, rules_engine, security, watcher};

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
fn integration_super_process_aggregation_is_available() {
    let grouped = metrics::aggregate_super_processes(Some(20));
    assert!(grouped.len() <= 20);
    assert!(grouped.iter().all(|sp| sp.process_count >= 1));
}

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
fn integration_killer_rejects_invalid_pid() {
    let result = killer::kill_process_safe(0, &[]);
    assert!(matches!(result, Err(killer::KillError::InvalidPid(0))));
}

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
fn integration_crypto_roundtrip_works() {
    let key = [42u8; 32];
    let payload = serde_json::json!({"telemetry": true, "interval": 2});

    let encrypted = crypto::encrypt_json(&key, &payload).expect("encrypt payload");
    let decrypted: serde_json::Value =
        crypto::decrypt_json(&key, &encrypted).expect("decrypt payload");

    assert_eq!(decrypted, payload);
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

#[test]
fn integration_security_audit_trail_rotates_encrypted_logs() {
    let dir = std::env::temp_dir().join(format!("omnimon-int-audit-{}", std::process::id()));
    let trail = core::audit_trail::EncryptedAuditTrail::new(&dir, 200, 3);
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
