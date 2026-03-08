//! Fuzz target for the rules evaluation engine.
//!
//! Feeds arbitrary JSON as both rules and connection events to ensure the
//! evaluation pipeline never panics, even with corrupted or adversarial data.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        // Try to parse input as a JSON object with "rules" and "events" keys.
        // If parsing fails, that's fine — we just want to ensure no panic.
        let Ok(val) = serde_json::from_str::<serde_json::Value>(input) else {
            return;
        };

        // Attempt to load rules from a subsection of the input
        if let Some(rules_str) = val.get("rules").and_then(|v| v.as_str()) {
            let _ = core::rules_engine::upsert_rules_from_ai_json(rules_str);
        }

        // Build synthetic connection events from input
        let mut events = Vec::new();
        if let Some(arr) = val.get("events").and_then(|v| v.as_array()) {
            for ev in arr.iter().take(64) {
                let pid = ev.get("pid").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let dst_ip = ev
                    .get("dst_ip")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0.0")
                    .to_string();
                let dst_port = ev.get("dst_port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                events.push(core::network::ProcessConnectionEvent {
                    pid,
                    protocol: core::network::TransportProtocol::Tcp,
                    direction: core::network::TrafficDirection::Outbound,
                    src_ip: "10.0.0.1".to_string(),
                    dst_ip,
                    src_port: 50000,
                    dst_port,
                    bytes: 100,
                });
            }
        }

        let runtime = vec![core::rules_engine::ProcessRuntime {
            pid: 1,
            process_name: "fuzz-proc".to_string(),
            memory_bytes: 100 * 1_048_576,
        }];

        // Must never panic.
        let _ = core::rules_engine::evaluate_events(&events, &runtime);
    }
});
