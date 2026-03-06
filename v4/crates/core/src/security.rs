use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BehaviorIndicator {
    DllInjection,
    RemoteThreadInjection,
    ProcessHollowing,
    SuspiciousMemoryRead,
    UnsignedModuleLoad,
    SuspiciousNetworkConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MitreTechnique {
    pub technique_id: String,
    pub tactic: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessBehaviorObservation {
    pub pid: u32,
    pub process_name: String,
    pub indicator: BehaviorIndicator,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessThreatLabel {
    pub pid: u32,
    pub process_name: String,
    pub indicator: BehaviorIndicator,
    pub mitre_techniques: Vec<MitreTechnique>,
    pub confidence: f32,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub blocked_ips: Vec<String>,
    pub unusual_ports: Vec<u16>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            blocked_ips: vec![
                "185.220.101.1".to_string(),
                "45.9.148.17".to_string(),
                "103.27.202.89".to_string(),
            ],
            unusual_ports: vec![4444, 1337, 31337, 5555, 6667, 9001],
        }
    }
}

pub fn map_behavior_to_mitre(indicator: &BehaviorIndicator) -> Vec<MitreTechnique> {
    match indicator {
        BehaviorIndicator::DllInjection => vec![MitreTechnique {
            technique_id: "T1055.001".to_string(),
            tactic: "Defense Evasion / Privilege Escalation".to_string(),
            name: "Dynamic-link Library Injection".to_string(),
        }],
        BehaviorIndicator::RemoteThreadInjection => vec![MitreTechnique {
            technique_id: "T1055.003".to_string(),
            tactic: "Defense Evasion / Privilege Escalation".to_string(),
            name: "Thread Execution Hijacking".to_string(),
        }],
        BehaviorIndicator::ProcessHollowing => vec![MitreTechnique {
            technique_id: "T1055.012".to_string(),
            tactic: "Defense Evasion".to_string(),
            name: "Process Hollowing".to_string(),
        }],
        BehaviorIndicator::SuspiciousMemoryRead => vec![MitreTechnique {
            technique_id: "T1003".to_string(),
            tactic: "Credential Access".to_string(),
            name: "OS Credential Dumping".to_string(),
        }],
        BehaviorIndicator::UnsignedModuleLoad => vec![MitreTechnique {
            technique_id: "T1574".to_string(),
            tactic: "Persistence / Privilege Escalation".to_string(),
            name: "Hijack Execution Flow".to_string(),
        }],
        BehaviorIndicator::SuspiciousNetworkConnection => vec![
            MitreTechnique {
                technique_id: "T1043".to_string(),
                tactic: "Command and Control".to_string(),
                name: "Commonly Used Port".to_string(),
            },
            MitreTechnique {
                technique_id: "T1571".to_string(),
                tactic: "Command and Control".to_string(),
                name: "Non-Standard Port".to_string(),
            },
        ],
    }
}

pub fn evaluate_network_events(
    events: &[crate::network::ProcessConnectionEvent],
    policy: &NetworkPolicy,
) -> Vec<ProcessBehaviorObservation> {
    let mut observations = Vec::new();
    for event in events {
        let hits_blocklist = policy.blocked_ips.iter().any(|ip| ip == &event.dst_ip);
        let unusual_port = policy.unusual_ports.contains(&event.dst_port);
        if !hits_blocklist && !unusual_port {
            continue;
        }

        let mut detail = Vec::new();
        if hits_blocklist {
            detail.push("destination_ip_blocked");
        }
        if unusual_port {
            detail.push("unusual_destination_port");
        }

        observations.push(ProcessBehaviorObservation {
            pid: event.pid,
            process_name: format!("pid-{}", event.pid),
            indicator: BehaviorIndicator::SuspiciousNetworkConnection,
            detail: Some(format!(
                "{} {}:{} -> {}:{} proto={:?}",
                detail.join("+"),
                event.src_ip,
                event.src_port,
                event.dst_ip,
                event.dst_port,
                event.protocol
            )),
        });
    }
    observations
}

pub fn label_process_observations(
    observations: &[ProcessBehaviorObservation],
) -> Vec<ProcessThreatLabel> {
    observations
        .iter()
        .map(|obs| {
            let techniques = map_behavior_to_mitre(&obs.indicator);
            let confidence = if obs.detail.as_ref().map(|d| !d.is_empty()).unwrap_or(false) {
                0.9
            } else {
                0.7
            };

            ProcessThreatLabel {
                pid: obs.pid,
                process_name: obs.process_name.clone(),
                indicator: obs.indicator.clone(),
                mitre_techniques: techniques,
                confidence,
                context: obs.detail.clone(),
            }
        })
        .collect()
}

pub fn label_and_record_observations(
    observations: &[ProcessBehaviorObservation],
    trail: &crate::audit_trail::EncryptedAuditTrail,
    key: &[u8; 32],
) -> Result<Vec<ProcessThreatLabel>, String> {
    let labels = label_process_observations(observations);
    for label in &labels {
        trail.append_label(key, label)?;
    }
    Ok(labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_dll_injection_to_t1055() {
        let mapped = map_behavior_to_mitre(&BehaviorIndicator::DllInjection);
        assert_eq!(mapped.len(), 1);
        assert!(mapped[0].technique_id.starts_with("T1055"));
    }

    #[test]
    fn labels_observations_with_confidence() {
        let observations = vec![ProcessBehaviorObservation {
            pid: 10,
            process_name: "suspicious.exe".to_string(),
            indicator: BehaviorIndicator::RemoteThreadInjection,
            detail: Some("CreateRemoteThread after WriteProcessMemory".to_string()),
        }];

        let labels = label_process_observations(&observations);
        assert_eq!(labels.len(), 1);
        assert!(labels[0].confidence > 0.8);
        assert!(labels[0].context.is_some());
    }

    #[test]
    fn labels_can_be_persisted_to_encrypted_trail() {
        let observations = vec![ProcessBehaviorObservation {
            pid: 55,
            process_name: "dropper.exe".to_string(),
            indicator: BehaviorIndicator::DllInjection,
            detail: Some("remote thread observed".to_string()),
        }];

        let dir = std::env::temp_dir().join(format!("omnimon-sec-test-{}", std::process::id()));
        let trail = crate::audit_trail::EncryptedAuditTrail::new(&dir, 2048, 3);
        let key = [3u8; 32];

        let labels = label_and_record_observations(&observations, &trail, &key).expect("record");
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn network_policy_maps_to_mitre_network_techniques() {
        let policy = NetworkPolicy::default();
        let events = vec![crate::network::ProcessConnectionEvent {
            pid: 99,
            protocol: crate::network::TransportProtocol::Tcp,
            direction: crate::network::TrafficDirection::Outbound,
            src_ip: "10.0.0.2".to_string(),
            dst_ip: policy.blocked_ips[0].clone(),
            src_port: 55000,
            dst_port: 4444,
            bytes: 128,
        }];

        let observations = evaluate_network_events(&events, &policy);
        assert_eq!(observations.len(), 1);
        let labels = label_process_observations(&observations);
        let ids = labels[0]
            .mitre_techniques
            .iter()
            .map(|t| t.technique_id.clone())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"T1043".to_string()));
    }
}
