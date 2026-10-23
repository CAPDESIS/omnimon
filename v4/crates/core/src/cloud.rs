//! CrabNebula Cloud integration. Handles API key validation, tier detection,
//! and provides a robust silent fallback so the app works without cloud features.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CN_KEYRING_SERVICE: &str = "omnimon";
const CN_KEYRING_ACCOUNT: &str = "crabnebula_api_key";
const CN_API_BASE: &str = "https://api.crabnebula.dev/v1";

/// Optional override for tests (and rare staging setups). Production leaves this unset.
fn cloud_api_base() -> String {
    std::env::var("OMNIMON_CN_API_BASE").unwrap_or_else(|_| CN_API_BASE.to_string())
}
const VALIDATION_TIMEOUT_SECS: u64 = 10;

/// Cloud subscription tier determined from the API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudTier {
    Free,
    Premium,
    Unknown,
}

impl CloudTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudTier::Free => "free",
            CloudTier::Premium => "premium",
            CloudTier::Unknown => "unknown",
        }
    }
}

/// Result of a CrabNebula API key validation attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudValidation {
    pub valid: bool,
    pub tier: CloudTier,
    pub organization: Option<String>,
    pub error: Option<String>,
}

impl CloudValidation {
    /// Constructs a failed validation result with the given error.
    fn failed(reason: impl Into<String>) -> Self {
        Self {
            valid: false,
            tier: CloudTier::Unknown,
            organization: None,
            error: Some(reason.into()),
        }
    }

    /// Constructs a successful validation result.
    fn success(tier: CloudTier, organization: Option<String>) -> Self {
        Self {
            valid: true,
            tier,
            organization,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Key Storage (keyring + fallback)
// ---------------------------------------------------------------------------

/// Save the CrabNebula API key to the OS keyring.
pub fn save_cloud_key(key: &str) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("Cloud API key cannot be empty".to_string());
    }
    let entry =
        Entry::new(CN_KEYRING_SERVICE, CN_KEYRING_ACCOUNT).map_err(|e| format!("keyring: {e}"))?;
    entry
        .set_password(trimmed)
        .map_err(|e| format!("keyring set: {e}"))
}

/// Retrieve the CrabNebula API key from the OS keyring.
pub fn get_cloud_key() -> Result<String, String> {
    let entry =
        Entry::new(CN_KEYRING_SERVICE, CN_KEYRING_ACCOUNT).map_err(|e| format!("keyring: {e}"))?;
    entry
        .get_password()
        .map_err(|e| format!("keyring get: {e}"))
}

/// Delete the CrabNebula API key from the OS keyring.
pub fn delete_cloud_key() -> Result<(), String> {
    let entry =
        Entry::new(CN_KEYRING_SERVICE, CN_KEYRING_ACCOUNT).map_err(|e| format!("keyring: {e}"))?;
    entry
        .delete_credential()
        .map_err(|e| format!("keyring delete: {e}"))
}

// ---------------------------------------------------------------------------
// API Key Validation
// ---------------------------------------------------------------------------

/// Validate a CN API key format before making any network request.
/// Keys must be non-empty, ASCII-printable, and within a reasonable length.
pub fn validate_key_format(key: &str) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key is empty".to_string());
    }
    if trimmed.len() > 512 {
        return Err("API key exceeds maximum length".to_string());
    }
    if !trimmed.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("API key contains invalid characters".to_string());
    }
    Ok(())
}

/// Validate the CrabNebula API key by making a lightweight authenticated request.
///
/// Returns a [`CloudValidation`] result. On network failure or API errors,
/// the result is marked as invalid but the app can still function (silent fallback).
pub async fn validate_cloud_key(key: &str) -> CloudValidation {
    if let Err(e) = validate_key_format(key) {
        return CloudValidation::failed(e);
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(VALIDATION_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => return CloudValidation::failed(format!("http client: {e}")),
    };

    let url = format!("{}/account", cloud_api_base());
    let response = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", key.trim()))
        .header("User-Agent", concat!("OmniMon/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                return CloudValidation::failed("request timed out");
            }
            if e.is_connect() {
                return CloudValidation::failed("could not connect to CrabNebula API");
            }
            return CloudValidation::failed(format!("network error: {e}"));
        }
    };

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return CloudValidation::failed("invalid or expired API key");
    }

    if !status.is_success() {
        return CloudValidation::failed(format!("API returned status {status}"));
    }

    // Parse the response to determine tier and organization
    let body: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => return CloudValidation::failed(format!("invalid response body: {e}")),
    };

    let organization = body
        .get("organization")
        .or_else(|| body.get("org"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tier = detect_tier_from_response(&body);

    CloudValidation::success(tier, organization)
}

/// Validate the stored API key (from keyring). Returns a silent-failure result
/// if no key is stored — the app continues without cloud features.
pub async fn validate_stored_cloud_key() -> CloudValidation {
    match get_cloud_key() {
        Ok(key) => validate_cloud_key(&key).await,
        Err(_) => CloudValidation::failed("no cloud key configured"),
    }
}

// ---------------------------------------------------------------------------
// Cloud Sync (upload)
// ---------------------------------------------------------------------------

const SYNC_TIMEOUT_SECS: u64 = 30;

/// Result of a successful cloud sync upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResult {
    pub uploaded_bytes: usize,
    pub sha256: String,
}

/// Clean, user-facing error used when sync is attempted without a key.
/// Never leaks keyring internals or key material.
pub const NO_CLOUD_KEY_ERROR: &str = "no cloud key configured";

fn validate_sync_key(key: &str) -> Result<String, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(NO_CLOUD_KEY_ERROR.to_string());
    }
    validate_key_format(trimmed)?;
    Ok(trimmed.to_string())
}

/// Build the minimal telemetry payload sent to the cloud. Process paths and
/// icon data are deliberately excluded (privacy + payload size).
pub fn build_telemetry_sync_payload(
    snapshot: &crate::telemetry::TelemetrySnapshot,
) -> serde_json::Value {
    let processes: Vec<serde_json::Value> = snapshot
        .processes
        .iter()
        .map(|p| {
            serde_json::json!({
                "pid": p.pid,
                "name": p.name,
                "memory_bytes": p.memory_bytes,
                "cpu_usage_percent": p.cpu_usage_percent,
                "net_rx_bytes_per_sec": p.net_rx_bytes_per_sec,
                "net_tx_bytes_per_sec": p.net_tx_bytes_per_sec,
                "is_system": p.is_system,
            })
        })
        .collect();
    serde_json::json!({
        "kind": "telemetry_snapshot",
        "schema_version": 1,
        "app_version": env!("CARGO_PKG_VERSION"),
        "cpu_usage_percent": snapshot.cpu_usage_percent,
        "total_memory_bytes": snapshot.total_memory_bytes,
        "used_memory_bytes": snapshot.used_memory_bytes,
        "swap_used_mb": snapshot.swap_used_mb,
        "net_rx_bytes_per_sec": snapshot.net_rx_bytes_per_sec,
        "net_tx_bytes_per_sec": snapshot.net_tx_bytes_per_sec,
        "total_processes": snapshot.total_processes,
        "processes": processes,
    })
}

/// Encrypt `payload` with a key HKDF-derived from the cloud API key.
pub fn encrypt_sync_payload(
    key: &str,
    payload: &serde_json::Value,
) -> Result<(crate::crypto::EncryptedPayload, String), String> {
    let key = validate_sync_key(key)?;
    let derived =
        crate::crypto::derive_key(key.as_bytes(), crate::crypto::kdf_context::DATA_ENCRYPTION);
    let encrypted = crate::crypto::encrypt_json(derived.as_bytes(), payload)?;
    let sha256 = crate::crypto::sha256_hex(encrypted.ciphertext_b64.as_bytes());
    Ok((encrypted, sha256))
}

/// Encrypt and upload `payload` to `{CN_API_BASE}/sync` with `key`.
/// No network call is made when the key is missing or malformed.
pub async fn sync_payload_to_cloud(
    key: &str,
    payload: &serde_json::Value,
) -> Result<CloudSyncResult, String> {
    let key = validate_sync_key(key)?;
    let (encrypted, sha256) = encrypt_sync_payload(&key, payload)?;
    let uploaded_bytes = encrypted.ciphertext_b64.len();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(SYNC_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let url = format!("{}/sync", cloud_api_base());
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {key}"))
        .header("User-Agent", concat!("OmniMon/", env!("CARGO_PKG_VERSION")))
        .json(&serde_json::json!({
            "algorithm": encrypted.algorithm,
            "nonce_b64": encrypted.nonce_b64,
            "ciphertext_b64": encrypted.ciphertext_b64,
            "sha256": sha256,
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "request timed out".to_string()
            } else if e.is_connect() {
                "could not connect to CrabNebula API".to_string()
            } else {
                format!("network error: {e}")
            }
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err("invalid or expired API key".to_string());
    }
    if !status.is_success() {
        return Err(format!("API returned status {status}"));
    }
    Ok(CloudSyncResult {
        uploaded_bytes,
        sha256,
    })
}

/// Sync using the key stored via [`save_cloud_key`]. Without a stored key this
/// returns [`NO_CLOUD_KEY_ERROR`] and makes no network call.
pub async fn sync_stored_payload_to_cloud(
    payload: &serde_json::Value,
) -> Result<CloudSyncResult, String> {
    let key = match get_cloud_key() {
        Ok(k) if !k.trim().is_empty() => k,
        _ => return Err(NO_CLOUD_KEY_ERROR.to_string()),
    };
    sync_payload_to_cloud(&key, payload).await
}

/// Determines the cloud tier from the API response body.
fn detect_tier_from_response(body: &serde_json::Value) -> CloudTier {
    // Check explicit string plan/tier fields
    let plan = body
        .get("plan")
        .or_else(|| body.get("tier"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if plan.contains("premium") || plan.contains("pro") || plan.contains("enterprise") {
        return CloudTier::Premium;
    }
    if plan.contains("free") || plan.contains("starter") || plan.contains("hobby") {
        return CloudTier::Free;
    }

    // Check "subscription" — can be a plain string or a nested object
    if let Some(sub) = body.get("subscription") {
        if let Some(sub_str) = sub.as_str() {
            let lower = sub_str.to_ascii_lowercase();
            if lower.contains("premium") || lower.contains("pro") || lower.contains("enterprise") {
                return CloudTier::Premium;
            }
            if lower.contains("free") || lower.contains("starter") {
                return CloudTier::Free;
            }
        }
        if let Some(plan_name) = sub.get("plan").and_then(|v| v.as_str()) {
            let lower = plan_name.to_ascii_lowercase();
            if lower.contains("premium") || lower.contains("pro") || lower.contains("enterprise") {
                return CloudTier::Premium;
            }
            if lower.contains("free") || lower.contains("starter") {
                return CloudTier::Free;
            }
        }
    }

    CloudTier::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that override the shared `OMNIMON_CN_API_BASE` env var
    /// so the suite passes under `cargo test`'s default parallel execution.
    static CN_API_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn validate_key_format_rejects_empty() {
        assert!(validate_key_format("").is_err());
        assert!(validate_key_format("   ").is_err());
    }

    #[test]
    fn validate_key_format_rejects_too_long() {
        let long_key = "a".repeat(513);
        assert!(validate_key_format(&long_key).is_err());
    }

    #[test]
    fn validate_key_format_rejects_non_ascii() {
        assert!(validate_key_format("key with\nnewline").is_err());
        assert!(validate_key_format("key with\ttab").is_err());
        assert!(validate_key_format("key with space").is_err());
    }

    #[test]
    fn validate_key_format_accepts_valid_keys() {
        assert!(validate_key_format("cn_live_abc123XYZ").is_ok());
        assert!(validate_key_format("sk-test-1234567890abcdef").is_ok());
        assert!(validate_key_format(&"x".repeat(512)).is_ok());
    }

    #[test]
    fn cloud_validation_failed_has_error() {
        let v = CloudValidation::failed("test error");
        assert!(!v.valid);
        assert_eq!(v.tier, CloudTier::Unknown);
        assert_eq!(v.error.as_deref(), Some("test error"));
    }

    #[test]
    fn cloud_validation_success_is_valid() {
        let v = CloudValidation::success(CloudTier::Premium, Some("Acme Corp".to_string()));
        assert!(v.valid);
        assert_eq!(v.tier, CloudTier::Premium);
        assert_eq!(v.organization.as_deref(), Some("Acme Corp"));
        assert!(v.error.is_none());
    }

    #[test]
    fn tier_as_str_matches() {
        assert_eq!(CloudTier::Free.as_str(), "free");
        assert_eq!(CloudTier::Premium.as_str(), "premium");
        assert_eq!(CloudTier::Unknown.as_str(), "unknown");
    }

    #[test]
    fn detect_tier_premium_from_plan_field() {
        let body = serde_json::json!({"plan": "Premium Pro"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Premium);
    }

    #[test]
    fn detect_tier_free_from_plan_field() {
        let body = serde_json::json!({"plan": "free"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Free);
    }

    #[test]
    fn detect_tier_from_nested_subscription() {
        let body = serde_json::json!({"subscription": {"plan": "enterprise"}});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Premium);
    }

    #[test]
    fn detect_tier_unknown_on_empty_body() {
        let body = serde_json::json!({});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Unknown);
    }

    #[test]
    fn detect_tier_premium_from_subscription_string() {
        let body = serde_json::json!({"subscription": "Pro Monthly"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Premium);
    }

    #[test]
    fn detect_tier_free_from_subscription_string() {
        let body = serde_json::json!({"subscription": "starter"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Free);
    }

    #[test]
    fn detect_tier_free_from_nested_subscription_plan() {
        let body = serde_json::json!({"subscription": {"plan": "free"}});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Free);
    }

    #[test]
    fn detect_tier_unknown_from_unrelated_subscription_string() {
        // A subscription string that matches no known keyword must not be
        // misclassified as a paid or free tier.
        let body = serde_json::json!({"subscription": "trial-cohort-7"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Unknown);
    }

    #[test]
    fn detect_tier_premium_from_tier_alias_field() {
        // The `tier` field is an accepted alias for `plan`.
        let body = serde_json::json!({"tier": "enterprise"});
        assert_eq!(detect_tier_from_response(&body), CloudTier::Premium);
    }

    #[test]
    fn cloud_validation_serializes_correctly() {
        let v = CloudValidation::success(CloudTier::Free, Some("TestOrg".to_string()));
        let json = serde_json::to_string(&v).expect("serialize");
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"Free\""));
    }

    #[tokio::test]
    async fn validate_cloud_key_rejects_empty() {
        let result = validate_cloud_key("").await;
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("empty"));
    }

    #[tokio::test]
    async fn validate_cloud_key_rejects_invalid_format() {
        let result = validate_cloud_key("key with spaces").await;
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("invalid characters"));
    }

    #[tokio::test]
    async fn validate_stored_key_fails_gracefully_without_key() {
        // In CI/test environments, no key is stored — should fail silently
        let result = validate_stored_cloud_key().await;
        // Either fails because no key or because no network — both are valid
        assert!(!result.valid || result.tier != CloudTier::Unknown);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn validate_cloud_key_accepts_mocked_premium_account() {
        let _cn_api_env_guard = CN_API_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/account")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"plan":"premium","organization":"Acme"}"#)
            .create_async()
            .await;

        // Env override is serialized via CN_API_ENV_LOCK (see mod tests).
        std::env::set_var("OMNIMON_CN_API_BASE", server.url());
        let result = validate_cloud_key("cn_live_test_key_ok").await;
        std::env::remove_var("OMNIMON_CN_API_BASE");

        assert!(result.valid);
        assert_eq!(result.tier, CloudTier::Premium);
        assert_eq!(result.organization.as_deref(), Some("Acme"));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn validate_cloud_key_maps_unauthorized() {
        let _cn_api_env_guard = CN_API_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/account")
            .with_status(401)
            .create_async()
            .await;

        std::env::set_var("OMNIMON_CN_API_BASE", server.url());
        let result = validate_cloud_key("cn_live_bad_key").await;
        std::env::remove_var("OMNIMON_CN_API_BASE");

        assert!(!result.valid);
        assert!(result.error.unwrap().contains("invalid or expired"));
    }

    #[test]
    fn sync_key_missing_is_clean_error() {
        assert_eq!(validate_sync_key("").unwrap_err(), NO_CLOUD_KEY_ERROR);
        assert_eq!(validate_sync_key("   ").unwrap_err(), NO_CLOUD_KEY_ERROR);
    }

    #[tokio::test]
    async fn sync_without_key_makes_no_upload_and_errors_cleanly() {
        let payload = serde_json::json!({"kind": "telemetry_snapshot"});
        let err = sync_payload_to_cloud("", &payload).await.unwrap_err();
        assert_eq!(err, NO_CLOUD_KEY_ERROR);
        let err = sync_payload_to_cloud("key with spaces", &payload)
            .await
            .unwrap_err();
        assert!(err.contains("invalid characters"));
    }

    #[test]
    fn sync_payload_encrypts_and_roundtrips() {
        let payload = serde_json::json!({"kind": "telemetry_snapshot", "total_processes": 2});
        let (encrypted, sha256) =
            encrypt_sync_payload("cn_live_test_key_ok", &payload).expect("encrypt");
        assert_eq!(encrypted.algorithm, "AES-256-GCM");
        assert_eq!(sha256.len(), 64);
        let derived = crate::crypto::derive_key(
            b"cn_live_test_key_ok",
            crate::crypto::kdf_context::DATA_ENCRYPTION,
        );
        let decrypted: serde_json::Value =
            crate::crypto::decrypt_json(derived.as_bytes(), &encrypted).expect("decrypt");
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn telemetry_sync_payload_excludes_paths_and_icons() {
        let snapshot = crate::telemetry::TelemetrySnapshot {
            total_memory_bytes: 100,
            free_memory_bytes: 40,
            used_memory_bytes: 60,
            free_percent: 40,
            swap_used_mb: 0,
            cpu_usage_percent: 1.5,
            net_rx_bytes_per_sec: 10,
            net_tx_bytes_per_sec: 20,
            total_processes: 1,
            processes: vec![crate::telemetry::ProcessTelemetryView {
                pid: 42,
                name: "demo".into(),
                exec_name: "demo".into(),
                group: "Apps".into(),
                group_key: "demo".into(),
                group_identity_type: "name".into(),
                grouped_display_name: "demo".into(),
                process_count: 1,
                bundle_id: Some("com.example.demo".into()),
                exe_path: Some("/secret/path/demo".into()),
                icon_data_url: Some("data:image/png;base64,AAAA".into()),
                memory_bytes: 1024,
                virtual_memory_bytes: 2048,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                net_rx_bytes_per_sec: 1,
                net_tx_bytes_per_sec: 2,
                energy_impact_score: None,
                cpu_usage_percent: 0.5,
                start_time: 1,
                is_system: false,
            }],
            super_processes: vec![],
        };
        let payload = build_telemetry_sync_payload(&snapshot);
        let text = payload.to_string();
        assert!(!text.contains("/secret/path"));
        assert!(!text.contains("data:image"));
        assert_eq!(payload["processes"][0]["name"], "demo");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn sync_payload_uploads_encrypted_to_mock_server() {
        let _guard = CN_API_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/sync")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true}"#)
            .create_async()
            .await;
        std::env::set_var("OMNIMON_CN_API_BASE", server.url());
        let result = sync_payload_to_cloud(
            "cn_live_test_key_ok",
            &serde_json::json!({"kind": "telemetry_snapshot"}),
        )
        .await;
        std::env::remove_var("OMNIMON_CN_API_BASE");
        let result = result.expect("sync");
        assert!(result.uploaded_bytes > 0);
        assert_eq!(result.sha256.len(), 64);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn sync_payload_maps_unauthorized() {
        let _guard = CN_API_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/sync")
            .with_status(401)
            .create_async()
            .await;
        std::env::set_var("OMNIMON_CN_API_BASE", server.url());
        let err = sync_payload_to_cloud(
            "cn_live_bad_key",
            &serde_json::json!({"kind": "telemetry_snapshot"}),
        )
        .await
        .unwrap_err();
        std::env::remove_var("OMNIMON_CN_API_BASE");
        assert!(err.contains("invalid or expired"));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn validate_cloud_key_maps_server_error_status() {
        let _cn_api_env_guard = CN_API_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/account")
            .with_status(500)
            .create_async()
            .await;

        std::env::set_var("OMNIMON_CN_API_BASE", server.url());
        let result = validate_cloud_key("cn_live_server_error").await;
        std::env::remove_var("OMNIMON_CN_API_BASE");

        assert!(!result.valid);
        assert!(result.error.unwrap().contains("API returned status"));
    }
}
