use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
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
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

pub fn get_settings_path() -> PathBuf {
    let mut path = dirs::data_dir().expect("Could not find data directory");
    path.push("com.omnimon.desktop");
    path.push("preferences.json");
    path
}

pub fn read_settings() -> Settings {
    let path = get_settings_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Settings::default()
    }
}

pub fn write_settings(settings: &Settings) -> Result<(), std::io::Error> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(path, content)
}
