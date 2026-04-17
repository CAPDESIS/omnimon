use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_IDLE_THRESHOLD: f64 = 1.0;
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;
pub const DEFAULT_AUTOMATION_INTERVAL_SECS: u64 = 5;
pub const MIN_IDLE_THRESHOLD: f64 = 0.1;
pub const MAX_IDLE_THRESHOLD: f64 = 10.0;
pub const MIN_POLL_INTERVAL_MS: u64 = 500;
pub const MAX_POLL_INTERVAL_MS: u64 = 10_000;
pub const MIN_AUTOMATION_INTERVAL_SECS: u64 = 1;
pub const MAX_AUTOMATION_INTERVAL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePreset {
    pub id: String,
    pub label: String,
    pub idle_threshold: f64,
    pub poll_interval_ms: u64,
    pub automation_interval_secs: u64,
    pub ai_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_profile_preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_interval_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_profile: Option<String>,
    /// Privacy toggle for AI prompts. When `Some(true)`, process names, exe
    /// paths, and window titles are redacted to stable pseudonymous tokens
    /// before being sent to any LLM provider. Defaults to `None` (treated as
    /// disabled). Stored in camelCase as `aiPrivacyMode`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_privacy_mode: Option<bool>,
    /// Hard ceiling on LLM calls allowed per UTC day. Complements the burst
    /// token bucket by preventing runaway-cost scenarios (e.g. an infinite
    /// retry loop overnight). `None` applies [`DEFAULT_AI_DAILY_LIMIT`].
    /// `Some(0)` disables the daily cap entirely — useful for Ollama/local
    /// providers where there is no per-call cost. Stored as `aiDailyLimit`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_daily_limit: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub profile_presets: Vec<ProfilePreset>,
    #[serde(flatten)]
    pub rest: HashMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: None,
            font_size: None,
            locale: None,
            idle_threshold: None,
            active_profile_preset: Some("general".to_string()),
            poll_interval_ms: Some(DEFAULT_POLL_INTERVAL_MS),
            automation_interval_secs: Some(DEFAULT_AUTOMATION_INTERVAL_SECS),
            ai_profile: Some("general".to_string()),
            ai_privacy_mode: None,
            ai_daily_limit: None,
            profile_presets: default_profile_presets(),
            rest: HashMap::new(),
        }
    }
}

pub fn default_profile_presets() -> Vec<ProfilePreset> {
    vec![
        ProfilePreset {
            id: "general".to_string(),
            label: "General".to_string(),
            idle_threshold: 1.0,
            poll_interval_ms: 2000,
            automation_interval_secs: 5,
            ai_profile: "general".to_string(),
        },
        ProfilePreset {
            id: "developer".to_string(),
            label: "Developer".to_string(),
            idle_threshold: 0.6,
            poll_interval_ms: 1500,
            automation_interval_secs: 3,
            ai_profile: "developer".to_string(),
        },
        ProfilePreset {
            id: "gaming".to_string(),
            label: "Gaming".to_string(),
            idle_threshold: 0.4,
            poll_interval_ms: 1000,
            automation_interval_secs: 2,
            ai_profile: "gaming".to_string(),
        },
        ProfilePreset {
            id: "battery".to_string(),
            label: "Battery Saver".to_string(),
            idle_threshold: 2.0,
            poll_interval_ms: 4000,
            automation_interval_secs: 10,
            ai_profile: "battery".to_string(),
        },
    ]
}

pub fn sanitize_profile_presets(mut presets: Vec<ProfilePreset>) -> Vec<ProfilePreset> {
    presets
        .retain(|preset| is_valid_preset_id(&preset.id) && is_valid_ai_profile(&preset.ai_profile));

    for preset in &mut presets {
        preset.label = sanitize_label(&preset.label, &preset.id);
        preset.id = preset.id.trim().to_lowercase();
        preset.idle_threshold = clamp_idle_threshold(preset.idle_threshold);
        preset.poll_interval_ms = clamp_poll_interval_ms(preset.poll_interval_ms);
        preset.automation_interval_secs =
            clamp_automation_interval_secs(preset.automation_interval_secs);
        preset.ai_profile = preset.ai_profile.trim().to_lowercase();
    }

    if presets.is_empty() {
        return default_profile_presets();
    }

    let mut deduped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for preset in presets {
        if seen.insert(preset.id.clone()) {
            deduped.push(preset);
        }
    }
    deduped
}

pub fn sanitize_settings(settings: &mut Settings) {
    settings.profile_presets =
        sanitize_profile_presets(std::mem::take(&mut settings.profile_presets));

    settings.idle_threshold = Some(clamp_idle_threshold(
        settings.idle_threshold.unwrap_or(DEFAULT_IDLE_THRESHOLD),
    ));
    settings.poll_interval_ms = Some(clamp_poll_interval_ms(
        settings
            .poll_interval_ms
            .unwrap_or(DEFAULT_POLL_INTERVAL_MS),
    ));
    settings.automation_interval_secs = Some(clamp_automation_interval_secs(
        settings
            .automation_interval_secs
            .unwrap_or(DEFAULT_AUTOMATION_INTERVAL_SECS),
    ));

    let ai_profile = settings.ai_profile.as_deref().unwrap_or("general");
    settings.ai_profile = Some(normalize_ai_profile(ai_profile));

    let active_id = settings
        .active_profile_preset
        .as_deref()
        .unwrap_or("general");
    if settings
        .profile_presets
        .iter()
        .any(|preset| preset.id == active_id)
    {
        settings.active_profile_preset = Some(active_id.to_string());
    } else {
        settings.active_profile_preset = settings
            .profile_presets
            .first()
            .map(|preset| preset.id.clone());
    }
}

pub fn get_settings_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("com.omnimon.desktop");
    path.push("preferences.json");
    path
}

pub fn read_settings() -> Settings {
    let path = get_settings_path();
    let mut settings = if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Settings::default()
    };
    sanitize_settings(&mut settings);
    settings
}

pub fn write_settings(settings: &Settings) -> Result<(), std::io::Error> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut sanitized = settings.clone();
    sanitize_settings(&mut sanitized);
    let content = serde_json::to_string_pretty(&sanitized)?;
    fs::write(path, content)
}

fn clamp_idle_threshold(value: f64) -> f64 {
    if !value.is_finite() {
        return DEFAULT_IDLE_THRESHOLD;
    }
    value.clamp(MIN_IDLE_THRESHOLD, MAX_IDLE_THRESHOLD)
}

fn clamp_poll_interval_ms(value: u64) -> u64 {
    value.clamp(MIN_POLL_INTERVAL_MS, MAX_POLL_INTERVAL_MS)
}

fn clamp_automation_interval_secs(value: u64) -> u64 {
    value.clamp(MIN_AUTOMATION_INTERVAL_SECS, MAX_AUTOMATION_INTERVAL_SECS)
}

fn sanitize_label(label: &str, fallback_id: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        fallback_id.to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn is_valid_preset_id(id: &str) -> bool {
    let trimmed = id.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 32
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

fn is_valid_ai_profile(profile: &str) -> bool {
    matches!(
        profile.trim().to_lowercase().as_str(),
        "general" | "developer" | "gaming" | "battery"
    )
}

fn normalize_ai_profile(profile: &str) -> String {
    let normalized = profile.trim().to_lowercase();
    if is_valid_ai_profile(&normalized) {
        normalized
    } else {
        "general".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_profile_presets_restores_defaults_when_empty() {
        let presets = sanitize_profile_presets(vec![]);
        assert_eq!(presets, default_profile_presets());
    }

    #[test]
    fn sanitize_profile_presets_clamps_and_dedupes() {
        let presets = sanitize_profile_presets(vec![
            ProfilePreset {
                id: "developer".into(),
                label: " ".into(),
                idle_threshold: 99.0,
                poll_interval_ms: 10,
                automation_interval_secs: 999,
                ai_profile: "developer".into(),
            },
            ProfilePreset {
                id: "developer".into(),
                label: "Duplicate".into(),
                idle_threshold: 1.0,
                poll_interval_ms: 2000,
                automation_interval_secs: 5,
                ai_profile: "developer".into(),
            },
        ]);

        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].label, "developer");
        assert_eq!(presets[0].idle_threshold, MAX_IDLE_THRESHOLD);
        assert_eq!(presets[0].poll_interval_ms, MIN_POLL_INTERVAL_MS);
        assert_eq!(
            presets[0].automation_interval_secs,
            MAX_AUTOMATION_INTERVAL_SECS
        );
    }

    #[test]
    fn sanitize_settings_normalizes_active_preset_and_profiles() {
        let mut settings = Settings {
            active_profile_preset: Some("missing".into()),
            ai_profile: Some("weird".into()),
            profile_presets: vec![ProfilePreset {
                id: "custom".into(),
                label: "Custom".into(),
                idle_threshold: 1.2,
                poll_interval_ms: 1200,
                automation_interval_secs: 6,
                ai_profile: "gaming".into(),
            }],
            ..Settings::default()
        };

        sanitize_settings(&mut settings);

        assert_eq!(settings.active_profile_preset.as_deref(), Some("custom"));
        assert_eq!(settings.ai_profile.as_deref(), Some("general"));
    }
}
