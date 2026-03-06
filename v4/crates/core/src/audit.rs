use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCveDatabase {
    pub schema_version: u32,
    pub entries: Vec<CveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CveEntry {
    pub cve_id: String,
    pub product: String,
    pub affected_version_reqs: Vec<String>,
    pub severity: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessVersionInfo {
    pub pid: u32,
    pub process_name: String,
    pub product: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CveMatch {
    pub pid: u32,
    pub process_name: String,
    pub product: String,
    pub detected_version: String,
    pub cve_id: String,
    pub severity: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NistIdentificationStatus {
    pub tracked_processes: usize,
    pub known_cve_matches: usize,
    pub asset_inventory_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NistMonitoringStatus {
    pub dpi_active: bool,
    pub suspicious_connection_count: usize,
    pub mitre_alert_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NistResponseStatus {
    pub encrypted_audit_trail_enabled: bool,
    pub last_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeartbeat {
    pub generated_at_unix_ms: u128,
    pub nist_control_family: String,
    pub identification: NistIdentificationStatus,
    pub monitoring: NistMonitoringStatus,
    pub response: NistResponseStatus,
}

impl LocalCveDatabase {
    pub fn from_json_str(content: &str) -> Result<Self, String> {
        serde_json::from_str(content).map_err(|e| format!("failed to parse CVE database JSON: {e}"))
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = fs::read_to_string(path.as_ref()).map_err(|e| {
            format!(
                "failed to read CVE database {}: {e}",
                path.as_ref().display()
            )
        })?;
        Self::from_json_str(&content)
    }
}

pub fn audit_processes_against_cves(
    processes: &[ProcessVersionInfo],
    db: &LocalCveDatabase,
) -> Vec<CveMatch> {
    let mut findings = Vec::new();

    for process in processes {
        let Some(version) = parse_semver(&process.version) else {
            continue;
        };

        for entry in &db.entries {
            if !entry.product.eq_ignore_ascii_case(&process.product) {
                continue;
            }

            let affected = entry
                .affected_version_reqs
                .iter()
                .filter_map(|req| VersionReq::parse(req).ok())
                .any(|req| req.matches(&version));

            if affected {
                findings.push(CveMatch {
                    pid: process.pid,
                    process_name: process.process_name.clone(),
                    product: process.product.clone(),
                    detected_version: process.version.clone(),
                    cve_id: entry.cve_id.clone(),
                    severity: entry.severity.clone(),
                    summary: entry.summary.clone(),
                });
            }
        }
    }

    findings
}

fn parse_semver(raw: &str) -> Option<Version> {
    Version::parse(raw)
        .ok()
        .or_else(|| Version::parse(raw.trim_start_matches('v')).ok())
}

pub fn build_security_heartbeat(
    tracked_processes: usize,
    cve_matches: usize,
    dpi_active: bool,
    suspicious_connection_count: usize,
    mitre_alert_count: usize,
    encrypted_audit_trail_enabled: bool,
    last_action: &str,
) -> SecurityHeartbeat {
    SecurityHeartbeat {
        generated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        nist_control_family: "NIST 800-53 (Identification, Monitoring, Response)".to_string(),
        identification: NistIdentificationStatus {
            tracked_processes,
            known_cve_matches: cve_matches,
            asset_inventory_complete: tracked_processes > 0,
        },
        monitoring: NistMonitoringStatus {
            dpi_active,
            suspicious_connection_count,
            mitre_alert_count,
        },
        response: NistResponseStatus {
            encrypted_audit_trail_enabled,
            last_action: last_action.to_string(),
        },
    }
}

pub fn persist_encrypted_security_heartbeat(
    path: impl AsRef<Path>,
    key: &[u8; 32],
    heartbeat: &SecurityHeartbeat,
) -> Result<(), String> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create heartbeat dir {}: {e}", parent.display()))?;
    }

    let payload = crate::crypto::encrypt_json(key, heartbeat)?;
    let encoded = serde_json::to_string(&payload)
        .map_err(|e| format!("failed to serialize encrypted heartbeat: {e}"))?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(|e| {
            format!(
                "failed to open heartbeat log {}: {e}",
                path.as_ref().display()
            )
        })?;

    file.write_all(encoded.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| format!("failed to write encrypted heartbeat: {e}"))
}

pub fn security_heartbeat_json(heartbeat: &SecurityHeartbeat) -> Result<String, String> {
    serde_json::to_string(heartbeat).map_err(|e| format!("failed to encode heartbeat JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audits_matching_versions() {
        let db = LocalCveDatabase {
            schema_version: 1,
            entries: vec![CveEntry {
                cve_id: "CVE-2024-0001".to_string(),
                product: "chrome".to_string(),
                affected_version_reqs: vec!["<124.0.0".to_string()],
                severity: Some("high".to_string()),
                summary: Some("Sandbox escape".to_string()),
            }],
        };

        let processes = vec![ProcessVersionInfo {
            pid: 123,
            process_name: "Chrome Renderer".to_string(),
            product: "Chrome".to_string(),
            version: "123.1.0".to_string(),
        }];

        let findings = audit_processes_against_cves(&processes, &db);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].cve_id, "CVE-2024-0001");
    }

    #[test]
    fn skips_non_semver_versions() {
        let db = LocalCveDatabase {
            schema_version: 1,
            entries: vec![CveEntry {
                cve_id: "CVE-2024-0002".to_string(),
                product: "chrome".to_string(),
                affected_version_reqs: vec!["<124.0.0".to_string()],
                severity: None,
                summary: None,
            }],
        };

        let processes = vec![ProcessVersionInfo {
            pid: 124,
            process_name: "Chrome".to_string(),
            product: "chrome".to_string(),
            version: "beta-channel".to_string(),
        }];

        let findings = audit_processes_against_cves(&processes, &db);
        assert!(findings.is_empty());
    }

    #[test]
    fn builds_nist_security_heartbeat() {
        let heartbeat = build_security_heartbeat(25, 2, true, 3, 4, true, "quarantined process");
        assert!(heartbeat.generated_at_unix_ms > 0);
        assert!(heartbeat.monitoring.dpi_active);
        assert_eq!(heartbeat.identification.known_cve_matches, 2);
    }

    #[test]
    fn persists_encrypted_security_heartbeat() {
        let path =
            std::env::temp_dir().join(format!("omnimon-heartbeat-{}.log.enc", std::process::id()));
        let key = [17u8; 32];
        let heartbeat = build_security_heartbeat(10, 1, false, 0, 1, true, "observed");

        let result = persist_encrypted_security_heartbeat(&path, &key, &heartbeat);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&path).expect("read heartbeat file");
        let line = content.lines().next().expect("line exists");
        let encrypted: crate::crypto::EncryptedPayload =
            serde_json::from_str(line).expect("parse encrypted payload");
        let decrypted: SecurityHeartbeat =
            crate::crypto::decrypt_json(&key, &encrypted).expect("decrypt heartbeat");

        assert_eq!(decrypted.identification.tracked_processes, 10);
    }
}
