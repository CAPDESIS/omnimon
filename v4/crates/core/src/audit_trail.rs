use crate::crypto;
use crate::security::ProcessThreatLabel;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitreAlertRecord {
    pub timestamp_unix_ms: u128,
    pub pid: u32,
    pub process_name: String,
    pub confidence: f32,
    pub technique_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EncryptedAuditTrail {
    root_dir: PathBuf,
    max_file_bytes: u64,
    max_files: usize,
}

impl EncryptedAuditTrail {
    pub fn new(root_dir: impl AsRef<Path>, max_file_bytes: u64, max_files: usize) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            max_file_bytes,
            max_files: max_files.max(2),
        }
    }

    pub fn append_label(&self, key: &[u8; 32], label: &ProcessThreatLabel) -> Result<(), String> {
        let techniques = label
            .mitre_techniques
            .iter()
            .map(|t| t.technique_id.clone())
            .collect::<Vec<_>>();

        let record = MitreAlertRecord {
            timestamp_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            pid: label.pid,
            process_name: label.process_name.clone(),
            confidence: label.confidence,
            technique_ids: techniques,
        };

        self.append_record(key, &record)
    }

    pub fn append_record(&self, key: &[u8; 32], record: &MitreAlertRecord) -> Result<(), String> {
        fs::create_dir_all(&self.root_dir).map_err(|e| {
            format!(
                "failed to create audit dir {}: {e}",
                self.root_dir.display()
            )
        })?;

        self.rotate_if_needed()?;

        let encrypted = crypto::encrypt_json(key, record)?;
        let line = serde_json::to_string(&encrypted)
            .map_err(|e| format!("failed to serialize encrypted record: {e}"))?;

        let active = self.active_file();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active)
            .map_err(|e| format!("failed to open audit file {}: {e}", active.display()))?;

        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|e| format!("failed to append audit record: {e}"))
    }

    fn rotate_if_needed(&self) -> Result<(), String> {
        let active = self.active_file();
        let current_size = fs::metadata(&active).map(|m| m.len()).unwrap_or(0);
        if current_size < self.max_file_bytes {
            return Ok(());
        }

        let last_index = self.max_files.saturating_sub(1);
        for idx in (1..=last_index).rev() {
            let src = self.indexed_file(idx);
            if !src.exists() {
                continue;
            }

            let dst = self.indexed_file(idx + 1);
            let _ = fs::remove_file(&dst);
            fs::rename(&src, &dst).map_err(|e| {
                format!(
                    "failed to rotate audit file {} -> {}: {e}",
                    src.display(),
                    dst.display()
                )
            })?;
        }

        if active.exists() {
            let first = self.indexed_file(1);
            let _ = fs::remove_file(&first);
            fs::rename(&active, &first).map_err(|e| {
                format!(
                    "failed to rotate active audit file {} -> {}: {e}",
                    active.display(),
                    first.display()
                )
            })?;
        }

        Ok(())
    }

    fn active_file(&self) -> PathBuf {
        self.root_dir.join("mitre-alerts.log.enc")
    }

    fn indexed_file(&self, index: usize) -> PathBuf {
        self.root_dir.join(format!("mitre-alerts.log.enc.{index}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_audit_log_writes_records() {
        let dir = std::env::temp_dir().join(format!("omnimon-audit-test-{}", std::process::id()));
        let trail = EncryptedAuditTrail::new(&dir, 1024, 3);
        let key = [9u8; 32];

        let record = MitreAlertRecord {
            timestamp_unix_ms: 1,
            pid: 10,
            process_name: "suspicious.exe".to_string(),
            confidence: 0.92,
            technique_ids: vec!["T1055.001".to_string()],
        };

        let write_result = trail.append_record(&key, &record);
        assert!(write_result.is_ok());
        assert!(trail.active_file().exists());
    }
}
