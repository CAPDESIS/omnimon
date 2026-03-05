use keyring::Entry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub fn save_api_key(key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let entry = Entry::new("macmon", "ai_api_key")?;
    entry.set_password(key)?;
    Ok(())
}

pub fn get_api_key() -> Result<String, Box<dyn Error + Send + Sync>> {
    let entry = Entry::new("macmon", "ai_api_key")?;
    Ok(entry.get_password()?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessSuggestion {
    pub pid: u32,
    pub name: String,
    pub reason: String,
}

pub async fn analyze_with_ai(
    provider: &str,
    model: &str,
    processes_json: &str,
    profile: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let api_key = get_api_key()?;
    let client = Client::new();

    let url = if provider.to_lowercase() == "openrouter" {
        "https://openrouter.ai/api/v1/chat/completions"
    } else {
        "https://api.openai.com/v1/chat/completions"
    };

    let prompt = format!(
        "You are macmon, a system optimization assistant. The user's current profile is: {}. \
        Analyze these running processes and suggest which ones should be safely closed to free up resources. \
        Return ONLY a JSON array of objects with 'pid' (number), 'name' (string), and 'reason' (string) keys. No markdown, no explanations.\n\nProcesses:\n{}",
        profile, processes_json
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant that returns strictly raw JSON arrays of suggestions."
            },
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let err_text = resp.text().await?;
        return Err(format!("API Error: {}", err_text).into());
    }

    let resp_json: serde_json::Value = resp.json().await?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Invalid response format")?;

    let content_clean = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let suggestions: Vec<ProcessSuggestion> = serde_json::from_str(content_clean)?;
    Ok(suggestions)
}
