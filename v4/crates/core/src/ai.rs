use keyring::Entry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::future::Future;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    OpenRouter,
    OpenAI,
    Gemini,
    Anthropic,
}

impl AiProvider {
    pub fn keyring_service(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => "omnimon_openrouter",
            AiProvider::OpenAI => "omnimon_openai",
            AiProvider::Gemini => "omnimon_gemini",
            AiProvider::Anthropic => "omnimon_anthropic",
        }
    }

    pub fn api_url(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
            AiProvider::OpenAI => "https://api.openai.com/v1/chat/completions",
            AiProvider::Gemini => {
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
            }
            AiProvider::Anthropic => "https://api.anthropic.com/v1/messages",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => "OpenRouter",
            AiProvider::OpenAI => "OpenAI",
            AiProvider::Gemini => "Gemini",
            AiProvider::Anthropic => "Anthropic",
        }
    }
}

impl std::str::FromStr for AiProvider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openrouter" => Ok(AiProvider::OpenRouter),
            "openai" => Ok(AiProvider::OpenAI),
            "gemini" => Ok(AiProvider::Gemini),
            "anthropic" => Ok(AiProvider::Anthropic),
            _ => Err(format!("Unknown AI provider: {s}")),
        }
    }
}

pub fn save_api_key(provider: AiProvider, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let entry = Entry::new(provider.keyring_service(), "ai_api_key")?;
    entry.set_password(key)?;
    Ok(())
}

fn normalize_api_key(key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key cannot be empty".into());
    }
    Ok(trimmed.to_string())
}

async fn save_api_key_validated_impl<V, VFut, S>(
    key: &str,
    validate: V,
    save: S,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    V: FnOnce(String) -> VFut,
    VFut: Future<Output = Result<(), Box<dyn Error + Send + Sync>>>,
    S: FnOnce(String) -> Result<(), Box<dyn Error + Send + Sync>>,
{
    let normalized = normalize_api_key(key)?;
    validate(normalized.clone()).await?;
    save(normalized)?;
    Ok(())
}

pub async fn save_api_key_with_ping(
    provider: AiProvider,
    model: &str,
    key: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    save_api_key_validated_impl(
        key,
        |normalized| async move { validate_api_key(provider, model, &normalized).await },
        |normalized| save_api_key(provider, &normalized),
    )
    .await
}

pub fn get_api_key(provider: AiProvider) -> Result<String, Box<dyn Error + Send + Sync>> {
    let entry = Entry::new(provider.keyring_service(), "ai_api_key")?;
    Ok(entry.get_password()?)
}

/// Validate an API key by making a lightweight test request to the provider.
pub async fn validate_api_key(
    provider: AiProvider,
    _model: &str,
    key: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = build_client()?;
    let url = provider.api_url();

    let resp = if matches!(provider, AiProvider::Anthropic) {
        client
            .post(url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(r#"{"model":"claude-haiku-4-5-20251001","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await?
    } else {
        client
            .post(url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .body(r#"{"model":"gpt-4o-mini","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
            .send()
            .await?
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err("Invalid API key — authentication failed".into());
    }
    // Any other response (including 400 for bad model) means the key itself is valid
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessSuggestion {
    pub pid: u32,
    pub name: String,
    pub reason: String,
}

fn build_client() -> Result<Client, Box<dyn Error + Send + Sync>> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?)
}

async fn send_with_retry(
    request_builder: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
    let mut backoff = INITIAL_BACKOFF_MS;
    for attempt in 0..=MAX_RETRIES {
        let resp = request_builder().send().await;
        match resp {
            Ok(r) => {
                let status = r.status();
                if status.is_success() || status.is_client_error() {
                    return Ok(r);
                }
                // Server error or rate limit — retry
                if attempt == MAX_RETRIES {
                    let err_text = r.text().await.unwrap_or_default();
                    return Err(
                        format!("API Error after {} retries: {}", MAX_RETRIES, err_text).into(),
                    );
                }
            }
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(Box::new(e));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(backoff)).await;
        backoff *= 2;
    }
    unreachable!()
}

pub async fn analyze_with_ai(
    provider: AiProvider,
    model: &str,
    processes_json: &str,
    profile: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let api_key = get_api_key(provider)?;
    let client = build_client()?;

    let prompt = format!(
        "You are macmon, a system optimization assistant. The user's current profile is: {}. \
        Analyze these running processes and suggest which ones should be safely closed to free up resources. \
        Return ONLY a JSON array of objects with 'pid' (number), 'name' (string), and 'reason' (string) keys. No markdown, no explanations.\n\nProcesses:\n{}",
        profile, processes_json
    );

    if provider == AiProvider::Anthropic {
        return analyze_anthropic(&client, &api_key, model, &prompt).await;
    }

    // OpenAI-compatible endpoint (OpenRouter, OpenAI, Gemini)
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

    let resp = send_with_retry(|| {
        client
            .post(provider.api_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
    })
    .await?;

    if resp.status().is_client_error() {
        let err_text = resp.text().await?;
        return Err(format!("API Error: {}", err_text).into());
    }

    let resp_json: serde_json::Value = resp.json().await?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Invalid response format")?;

    parse_suggestions(content)
}

async fn analyze_anthropic(
    client: &Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": "You are a helpful assistant that returns strictly raw JSON arrays of suggestions.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = send_with_retry(|| {
        client
            .post(AiProvider::Anthropic.api_url())
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
    })
    .await?;

    if resp.status().is_client_error() {
        let err_text = resp.text().await?;
        return Err(format!("API Error: {}", err_text).into());
    }

    let resp_json: serde_json::Value = resp.json().await?;

    let content = resp_json["content"][0]["text"]
        .as_str()
        .ok_or("Invalid Anthropic response format")?;

    parse_suggestions(content)
}

/// Free-form AI analysis: send context, get back plain text insight.
pub async fn analyze_context(
    provider: AiProvider,
    model: &str,
    context: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let api_key = get_api_key(provider)?;
    let client = build_client()?;

    let system_msg = "You are macmon, a macOS system monitor assistant. Analyze the given process and browser tab information. Provide concise, actionable insights: what the process does, whether it's safe to close, memory impact, and any recommendations. Use short paragraphs. Be direct.";

    if provider == AiProvider::Anthropic {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 2048,
            "system": system_msg,
            "messages": [{ "role": "user", "content": context }]
        });
        let resp = send_with_retry(|| {
            client
                .post(AiProvider::Anthropic.api_url())
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
        })
        .await?;
        if resp.status().is_client_error() {
            return Err(format!("API Error: {}", resp.text().await?).into());
        }
        let resp_json: serde_json::Value = resp.json().await?;
        return resp_json["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid Anthropic response format".into());
    }

    // OpenAI-compatible
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_msg },
            { "role": "user", "content": context }
        ]
    });
    let resp = send_with_retry(|| {
        client
            .post(provider.api_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
    })
    .await?;
    if resp.status().is_client_error() {
        return Err(format!("API Error: {}", resp.text().await?).into());
    }
    let resp_json: serde_json::Value = resp.json().await?;
    resp_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid response format".into())
}

fn parse_suggestions(
    content: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let content_clean = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let suggestions: Vec<ProcessSuggestion> = serde_json::from_str(content_clean)?;
    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::str::FromStr;

    #[test]
    fn ai_provider_from_str_works() {
        assert_eq!(
            AiProvider::from_str("openrouter").unwrap(),
            AiProvider::OpenRouter
        );
        assert_eq!(AiProvider::from_str("OpenAI").unwrap(), AiProvider::OpenAI);
        assert_eq!(AiProvider::from_str("GEMINI").unwrap(), AiProvider::Gemini);
        assert_eq!(
            AiProvider::from_str("anthropic").unwrap(),
            AiProvider::Anthropic
        );
        assert!(AiProvider::from_str("unknown").is_err());
    }

    #[test]
    fn ai_provider_keyring_services_are_distinct() {
        let services: Vec<&str> = [
            AiProvider::OpenRouter,
            AiProvider::OpenAI,
            AiProvider::Gemini,
            AiProvider::Anthropic,
        ]
        .iter()
        .map(|p| p.keyring_service())
        .collect();

        for (i, a) in services.iter().enumerate() {
            for (j, b) in services.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "keyring services must be distinct");
                }
            }
        }
    }

    #[test]
    fn ai_provider_api_urls_are_https() {
        for provider in &[
            AiProvider::OpenRouter,
            AiProvider::OpenAI,
            AiProvider::Gemini,
            AiProvider::Anthropic,
        ] {
            assert!(
                provider.api_url().starts_with("https://"),
                "{:?} url must be https",
                provider
            );
        }
    }

    #[test]
    fn parse_suggestions_strips_markdown_fencing() {
        let input = "```json\n[{\"pid\":1,\"name\":\"foo\",\"reason\":\"bar\"}]\n```";
        let result = parse_suggestions(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "foo");
    }

    #[test]
    fn parse_suggestions_handles_plain_json() {
        let input = "[{\"pid\":42,\"name\":\"test\",\"reason\":\"heavy\"}]";
        let result = parse_suggestions(input).unwrap();
        assert_eq!(result[0].pid, 42);
    }

    #[test]
    fn parse_suggestions_rejects_invalid_json() {
        assert!(parse_suggestions("not json").is_err());
    }

    #[tokio::test]
    async fn send_with_retry_succeeds_on_first_try() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/test")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = server.url();
        let resp = send_with_retry(|| client.post(format!("{}/test", url)))
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }

    #[tokio::test]
    async fn send_with_retry_retries_on_server_error() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/test")
            .with_status(500)
            .with_body("internal error")
            .expect_at_least(2)
            .create_async()
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = server.url();
        let result = send_with_retry(|| client.post(format!("{}/test", url))).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API Error after"));
    }

    #[tokio::test]
    async fn send_with_retry_does_not_retry_client_errors() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/test")
            .with_status(401)
            .with_body("unauthorized")
            .expect(1)
            .create_async()
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = server.url();
        let resp = send_with_retry(|| client.post(format!("{}/test", url)))
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 401);
    }

    #[tokio::test]
    async fn save_api_key_with_ping_trims_key_before_validation_and_save() {
        let seen_validate = Arc::new(Mutex::new(String::new()));
        let seen_save = Arc::new(Mutex::new(String::new()));

        let seen_validate_closure = Arc::clone(&seen_validate);
        let seen_save_closure = Arc::clone(&seen_save);

        let result = save_api_key_validated_impl(
            "  sk-api-key  ",
            move |normalized| {
                let seen_validate_inner = Arc::clone(&seen_validate_closure);
                async move {
                    *seen_validate_inner.lock().unwrap() = normalized;
                    Ok(())
                }
            },
            move |normalized| {
                *seen_save_closure.lock().unwrap() = normalized;
                Ok(())
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(&*seen_validate.lock().unwrap(), "sk-api-key");
        assert_eq!(&*seen_save.lock().unwrap(), "sk-api-key");
    }

    #[tokio::test]
    async fn save_api_key_with_ping_does_not_save_when_ping_fails() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/ping")
            .match_header("authorization", "Bearer sk-invalid")
            .with_status(401)
            .with_body("unauthorized")
            .create_async()
            .await;

        let saved = Arc::new(Mutex::new(false));
        let saved_closure = Arc::clone(&saved);
        let url = format!("{}/ping", server.url());

        let result = save_api_key_validated_impl(
            "  sk-invalid  ",
            move |_normalized| {
                let url_inner = url.clone();
                async move {
                    let client = Client::new();
                    let resp = client
                        .post(url_inner)
                        .header("Authorization", "Bearer sk-invalid")
                        .send()
                        .await?;
                    if resp.status().is_success() {
                        Ok(())
                    } else {
                        Err("Invalid API key — authentication failed".into())
                    }
                }
            },
            move |_normalized| {
                *saved_closure.lock().unwrap() = true;
                Ok(())
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(*saved.lock().unwrap(), false);
    }

    #[tokio::test]
    async fn save_api_key_with_ping_saves_when_ping_succeeds() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/ping")
            .match_header("authorization", "Bearer sk-valid")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let saved_value = Arc::new(Mutex::new(String::new()));
        let saved_value_closure = Arc::clone(&saved_value);
        let url = format!("{}/ping", server.url());

        let result = save_api_key_validated_impl(
            "  sk-valid  ",
            move |_normalized| {
                let url_inner = url.clone();
                async move {
                    let client = Client::new();
                    let resp = client
                        .post(url_inner)
                        .header("Authorization", "Bearer sk-valid")
                        .send()
                        .await?;
                    if resp.status().is_success() {
                        Ok(())
                    } else {
                        Err("Invalid API key — authentication failed".into())
                    }
                }
            },
            move |normalized| {
                *saved_value_closure.lock().unwrap() = normalized;
                Ok(())
            },
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(&*saved_value.lock().unwrap(), "sk-valid");
    }
}
