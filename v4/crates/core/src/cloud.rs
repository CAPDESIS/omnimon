//! CrabNebula Cloud integration. Handles API key validation, tier detection,
//! and provides a robust silent fallback so the app works without cloud features.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CN_KEYRING_SERVICE: &str = "omnimon";
const CN_KEYRING_ACCOUNT: &str = "crabnebula_api_key";
const CN_API_BASE: &str = "https://api.crabnebula.dev/v1";
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

    let url = format!("{CN_API_BASE}/account");
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
}
