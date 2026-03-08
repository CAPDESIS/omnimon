//! Artificial Intelligence integration module. Handles communication with various LLM providers (OpenAI, Anthropic, Gemini, OpenRouter) for predictive system optimization and context analysis.

use keyring::Entry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use std::sync::RwLock;
use std::time::Duration;

static AI_CACHE: OnceLock<RwLock<HashMap<u64, String>>> = OnceLock::new();

fn get_ai_cache() -> &'static RwLock<HashMap<u64, String>> {
    AI_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn check_prompt_injection(text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let lower = text.to_lowercase();
    let blocked_phrases = [
        "ignora las instrucciones",
        "ignore previous instructions",
        "borra mis reglas",
        "delete rules",
        "olvida tu propósito",
        "forget your purpose",
        "actúa como",
        "act as",
    ];

    for phrase in blocked_phrases {
        if lower.contains(phrase) {
            return Err("Acción bloqueada: posible inyección de prompt detectada.".into());
        }
    }
    Ok(())
}

const MAX_RETRIES: u32 = 1;
const INITIAL_BACKOFF_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 60;

// --- API URLs ---
const API_URL_OPENROUTER: &str = "https://openrouter.ai/api/v1/chat/completions";
const API_URL_OPENAI: &str = "https://api.openai.com/v1/chat/completions";
const API_URL_GEMINI: &str =
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions";
const API_URL_ANTHROPIC: &str = "https://api.anthropic.com/v1/messages";
const API_URL_OLLAMA: &str = "http://localhost:11434/v1/chat/completions";
const OLLAMA_TAGS_URL: &str = "http://localhost:11434/api/tags";

// --- Keyring service names ---
const KEYRING_SERVICE_OPENROUTER: &str = "omnimon_openrouter";
const KEYRING_SERVICE_OPENAI: &str = "omnimon_openai";
const KEYRING_SERVICE_GEMINI: &str = "omnimon_gemini";
const KEYRING_SERVICE_ANTHROPIC: &str = "omnimon_anthropic";
const KEYRING_SERVICE_OLLAMA: &str = "omnimon_ollama";
const KEYRING_USER: &str = "ai_api_key";

// --- OpenRouter headers ---
const OPENROUTER_REFERER: &str = "https://github.com/chochy2001/omnimon";
const OPENROUTER_TITLE: &str = "OmniMon";

// --- Anthropic protocol version ---
const ANTHROPIC_VERSION: &str = "2023-06-01";

// --- Max tokens ---
const MAX_TOKENS_ANALYSIS: u32 = 4096;
const MAX_TOKENS_CONTEXT: u32 = 2048;
const MAX_TOKENS_CHAT: u32 = 2048;
const MAX_TOKENS_VALIDATION: u32 = 1;

// --- Validation model names ---
const VALIDATION_MODEL_ANTHROPIC: &str = "claude-haiku-4-5-20251001";
const VALIDATION_MODEL_OPENROUTER: &str = "meta-llama/llama-3.2-3b-instruct:free";
const VALIDATION_MODEL_DEFAULT: &str = "gpt-4o-mini";

// --- Byte conversion constants ---
const BYTES_PER_GB: f64 = 1_073_741_824.0;
const BYTES_PER_MB: f64 = 1_048_576.0;

/// Supported AI backend providers for process analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    OpenRouter,
    OpenAI,
    Gemini,
    Anthropic,
    Ollama,
}

impl AiProvider {
    pub fn keyring_service(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => KEYRING_SERVICE_OPENROUTER,
            AiProvider::OpenAI => KEYRING_SERVICE_OPENAI,
            AiProvider::Gemini => KEYRING_SERVICE_GEMINI,
            AiProvider::Anthropic => KEYRING_SERVICE_ANTHROPIC,
            AiProvider::Ollama => KEYRING_SERVICE_OLLAMA,
        }
    }

    pub fn api_url(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => API_URL_OPENROUTER,
            AiProvider::OpenAI => API_URL_OPENAI,
            AiProvider::Gemini => API_URL_GEMINI,
            AiProvider::Anthropic => API_URL_ANTHROPIC,
            AiProvider::Ollama => API_URL_OLLAMA,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::OpenRouter => "OpenRouter",
            AiProvider::OpenAI => "OpenAI",
            AiProvider::Gemini => "Gemini",
            AiProvider::Anthropic => "Anthropic",
            AiProvider::Ollama => "Ollama (Local)",
        }
    }

    /// Returns true if this provider requires an API key.
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, AiProvider::Ollama)
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
            "ollama" => Ok(AiProvider::Ollama),
            _ => Err(format!("Unknown AI provider: {s}")),
        }
    }
}

/// Persists an API key for the given provider in the OS keyring.
pub fn save_api_key(provider: AiProvider, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let entry = Entry::new(provider.keyring_service(), KEYRING_USER)?;
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

/// Validates the API key with a lightweight ping request, then saves it to the keyring.
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

/// Retrieves the stored API key for the given provider from the OS keyring.
pub fn get_api_key(provider: AiProvider) -> Result<String, Box<dyn Error + Send + Sync>> {
    let entry = Entry::new(provider.keyring_service(), KEYRING_USER)?;
    Ok(entry.get_password()?)
}

// ---------------------------------------------------------------------------
// Header helpers
// ---------------------------------------------------------------------------

/// Adds Anthropic-specific headers (x-api-key, version, content-type) to a request.
fn add_anthropic_headers(req: reqwest::RequestBuilder, api_key: &str) -> reqwest::RequestBuilder {
    req.header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
}

/// Adds OpenRouter-specific headers (Referer, X-Title) to a request.
fn add_openrouter_headers(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("HTTP-Referer", OPENROUTER_REFERER)
        .header("X-Title", OPENROUTER_TITLE)
}

/// Checks a response status and returns an error string for non-success responses.
/// On success, returns the response body as a `String`.
async fn check_response_status(
    resp: reqwest::Response,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!(
            "AI request failed (status {}): {}",
            status.as_u16(),
            body_text.chars().take(200).collect::<String>()
        )
        .into());
    }
    Ok(resp.text().await?)
}

/// Validate an API key by making a lightweight test request to the provider.
pub async fn validate_api_key(
    provider: AiProvider,
    _model: &str,
    key: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = build_client()?;
    let url = provider.api_url();

    // Ollama: just check that the server is reachable (no API key needed)
    if provider == AiProvider::Ollama {
        let resp = client.get(OLLAMA_TAGS_URL).send().await.map_err(
            |_| -> Box<dyn Error + Send + Sync> {
                "Ollama is not running — start it with `ollama serve`".into()
            },
        )?;
        if !resp.status().is_success() {
            return Err("Ollama server returned an error".into());
        }
        return Ok(());
    }

    let resp = match provider {
        AiProvider::Anthropic => {
            let body = format!(
                r#"{{"model":"{}","max_tokens":{},"messages":[{{"role":"user","content":"hi"}}]}}"#,
                VALIDATION_MODEL_ANTHROPIC, MAX_TOKENS_VALIDATION
            );
            add_anthropic_headers(client.post(url), key)
                .body(body)
                .send()
                .await?
        }
        AiProvider::OpenRouter => {
            let body = format!(
                r#"{{"model":"{}","max_tokens":{},"messages":[{{"role":"user","content":"hi"}}]}}"#,
                VALIDATION_MODEL_OPENROUTER, MAX_TOKENS_VALIDATION
            );
            let req = client
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json");
            add_openrouter_headers(req).body(body).send().await?
        }
        _ => {
            let body = format!(
                r#"{{"model":"{}","max_tokens":{},"messages":[{{"role":"user","content":"hi"}}]}}"#,
                VALIDATION_MODEL_DEFAULT, MAX_TOKENS_VALIDATION
            );
            client
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?
        }
    };

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err("Invalid API key — authentication failed".into());
    }
    // Any other response (including 400 for bad model) means the key itself is valid
    Ok(())
}

/// An AI-generated suggestion to close a specific process, with a human-readable reason.
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
                    return Err(format!(
                        "AI service unavailable after {} retries (status {})",
                        MAX_RETRIES,
                        status.as_u16()
                    )
                    .into());
                }
            }
            Err(e) => {
                eprintln!("[ai-retry] attempt {attempt}/{MAX_RETRIES} network error: {e}");
                if attempt == MAX_RETRIES {
                    return Err(Box::new(e));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(backoff)).await;
        backoff *= 2;
    }
    Err("Unexpected exit from retry loop".into())
}

/// Sends the running process list to the AI provider and returns kill suggestions.
pub async fn analyze_with_ai(
    provider: AiProvider,
    model: &str,
    processes_json: &str,
    profile: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let api_key = get_api_key(provider)?;
    analyze_with_ai_key(provider, model, processes_json, profile, &api_key).await
}

/// Like `analyze_with_ai` but accepts an explicit API key (for Tauri Store fallback).
pub async fn analyze_with_ai_key(
    provider: AiProvider,
    model: &str,
    processes_json: &str,
    profile: &str,
    api_key: &str,
) -> Result<Vec<ProcessSuggestion>, Box<dyn Error + Send + Sync>> {
    let client = build_client()?;

    let prompt = format!(
        "You are OmniMon, a system optimization assistant. The user's current profile is: {}. \
        Analyze these running processes and suggest which ones should be safely closed to free up resources. \
        For browser helper/renderer processes, explain which type of activity likely causes the resource usage. \
        Return ONLY a JSON array of objects with 'pid' (number), 'name' (string), and 'reason' (string) keys. No markdown, no explanations.\n\nProcesses:\n{}",
        profile, processes_json
    );

    if provider == AiProvider::Anthropic {
        return analyze_anthropic(&client, api_key, model, &prompt).await;
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
        let mut req = client
            .post(provider.api_url())
            .header("Authorization", format!("Bearer {}", api_key));
        if provider == AiProvider::OpenRouter {
            req = add_openrouter_headers(req);
        }
        req.json(&body)
    })
    .await?;

    let resp_text = check_response_status(resp).await?;
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
        .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;

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
        "max_tokens": MAX_TOKENS_ANALYSIS,
        "system": "You are a helpful assistant that returns strictly raw JSON arrays of suggestions.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let resp = send_with_retry(|| {
        add_anthropic_headers(client.post(AiProvider::Anthropic.api_url()), api_key).json(&body)
    })
    .await?;

    let resp_text = check_response_status(resp).await?;
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
        .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;

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
    analyze_context_key(provider, model, context, &api_key).await
}

/// Like `analyze_context` but accepts an explicit API key (for Tauri Store fallback).
pub async fn analyze_context_key(
    provider: AiProvider,
    model: &str,
    context: &str,
    api_key: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    check_prompt_injection(context)?;

    let cache_key = calculate_hash(&(provider as u8, model, context));
    if let Ok(cache) = get_ai_cache().read() {
        if let Some(cached_response) = cache.get(&cache_key) {
            return Ok(cached_response.clone());
        }
    }

    let client = build_client()?;

    let system_msg = "You are OmniMon, a cross-platform system monitor assistant. Analyze the given process and browser tab information. If the process is a browser renderer/helper, use the tab context to explain which tab or site it likely belongs to. Provide concise, actionable insights: what the process does, whether it's safe to close, memory impact, and any recommendations. Use short paragraphs. Be direct.";

    let result_text = if provider == AiProvider::Anthropic {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": MAX_TOKENS_CONTEXT,
            "system": system_msg,
            "messages": [{ "role": "user", "content": context }]
        });
        let resp = send_with_retry(|| {
            add_anthropic_headers(client.post(AiProvider::Anthropic.api_url()), api_key).json(&body)
        })
        .await?;
        let resp_text = check_response_status(resp).await?;
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        let text_res: Result<String, Box<dyn Error + Send + Sync>> = resp_json["content"][0]
            ["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid Anthropic response format".into());
        text_res?
    } else {
        // OpenAI-compatible
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system_msg },
                { "role": "user", "content": context }
            ]
        });
        let resp = send_with_retry(|| {
            let mut req = client
                .post(provider.api_url())
                .header("Authorization", format!("Bearer {api_key}"));
            if provider == AiProvider::OpenRouter {
                req = add_openrouter_headers(req);
            }
            req.json(&body)
        })
        .await?;
        let resp_text = check_response_status(resp).await?;
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        let text_res: Result<String, Box<dyn Error + Send + Sync>> = resp_json["choices"][0]
            ["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid response format".into());
        text_res?
    };

    if let Ok(mut cache) = get_ai_cache().write() {
        cache.insert(cache_key, result_text.clone());
    }

    Ok(result_text)
}

// ---------------------------------------------------------------------------
// Interactive Chat with Tool Calling
// ---------------------------------------------------------------------------

/// Result of a tool call executed by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub details: String,
}

/// Full response from the AI chat endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub reply: String,
    pub tool_call: Option<ToolResult>,
}

/// Builds a system prompt injected with live OS state for tool-calling.
pub fn build_chat_system_prompt(state: &crate::watcher::SystemState) -> String {
    let ram_total_gb = state.total_memory_bytes as f64 / BYTES_PER_GB;
    let ram_used_gb = state.used_memory_bytes as f64 / BYTES_PER_GB;
    let ram_pct = if state.total_memory_bytes > 0 {
        (state.used_memory_bytes as f64 / state.total_memory_bytes as f64 * 100.0) as u32
    } else {
        0
    };

    let mut top_procs = state.cached_process_info.clone();
    top_procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    top_procs.truncate(15);

    let procs_list: Vec<String> = top_procs
        .iter()
        .map(|p| {
            format!(
                "  - PID {} | {} | {:.0}MB RAM | {:.1}% CPU",
                p.pid,
                p.name,
                p.memory_bytes as f64 / BYTES_PER_MB,
                p.cpu_pct
            )
        })
        .collect();

    format!(
        r#"You are OmniMon, a system monitor assistant running on {os}.

## System State
- CPU: {cpu:.1}% | RAM: {ram_used_gb:.1}/{ram_total_gb:.1} GB ({ram_pct}%) | Swap: {swap} MB | Net: RX {rx} B/s, TX {tx} B/s
- Top processes:
{procs}

## Tools
Respond with a JSON object ONLY when performing an action:
{{"tool": "<name>", "args": {{...}}, "reason": "brief explanation"}}

Available tools:
1. **kill_process** - Kill one process by PID. Args: {{"pid": <number>}}
2. **kill_by_name** - Kill ALL processes matching a name. Args: {{"name": "<string>"}}
3. **close_tabs** - Close browser tabs. Two modes:
   - **Positive match** (close tabs that match): Args: {{"pattern": "<url_or_title_substring>"}}
     Example: {{"tool": "close_tabs", "args": {{"pattern": "youtube|netflix"}}, "reason": "closing video sites"}}
   - **Exclusion match** (close ALL tabs EXCEPT matching): Args: {{"except": "<url_or_title_substring>"}}
     Example: {{"tool": "close_tabs", "args": {{"except": "crunchyroll|github|gemini"}}, "reason": "keeping only specified tabs"}}
   - Pattern matches against tab URL and title (case-insensitive substring).
   - Use pipe `|` to match multiple patterns.
   - When the user says "close everything except X, Y, Z" or "keep only X, Y, Z", use the `except` mode.
4. **add_automation_rule** - Auto-monitor a process. Args: {{"id": "<string>", "process_pattern": "<string>", "metric": "cpu|ram", "threshold": <number>, "duration_secs": <number>, "action": "kill|alert"}}
5. **remove_automation_rule** - Remove a rule. Args: {{"id": "<string>"}}

## Rules
1. If no action needed, respond with plain text analysis.
2. NEVER kill system-critical processes (kernel_task, launchd, WindowServer, loginwindow).
3. **Before ANY destructive action** (killing processes or closing tabs), you MUST:
   a. List EXACTLY what you will close/kill (names, URLs, PIDs).
   b. List what you will KEEP (if user specified exceptions).
   c. Ask for confirmation: "Should I proceed?"
   d. Only output the tool JSON AFTER the user confirms.
4. For close_tabs: ALWAYS list each tab you plan to close with its title and URL, and each tab you will keep.
5. Prefer kill_by_name over kill_process when the user references a process name.
6. Respond in the same language the user writes in.
7. **When the user confirms** with words like "sí", "yes", "hazlo", "procede", "dale", "do it", "go ahead", "adelante" — execute the previously discussed action immediately by outputting the tool JSON. Do NOT ask for confirmation again.
8. Use the conversation history to remember what was previously discussed. If you proposed an action and the user confirmed, execute it."#,
        os = std::env::consts::OS,
        cpu = state.cpu_usage_percent,
        swap = state.swap_used_mb,
        rx = state.net_rx_bytes_per_sec,
        tx = state.net_tx_bytes_per_sec,
        procs = procs_list.join("\n"),
    )
}

/// Parsed tool call from AI response.
#[derive(Debug, Clone, Deserialize)]
pub struct RawToolCall {
    pub tool: String,
    pub args: serde_json::Value,
    #[serde(default)]
    pub reason: String,
}

/// Tries to extract a JSON tool call from the AI response text.
fn parse_tool_call(text: &str) -> Option<RawToolCall> {
    // Find the first JSON object in the response
    let start = text.find('{')?;
    let mut depth = 0;
    let mut end = start;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let json_str = &text[start..end];
    let call: RawToolCall = serde_json::from_str(json_str).ok()?;
    // Only accept known tools
    match call.tool.as_str() {
        "kill_process"
        | "kill_by_name"
        | "close_tabs"
        | "add_automation_rule"
        | "remove_automation_rule" => Some(call),
        _ => None,
    }
}

/// Executes a validated tool call against the real OS.
///
/// **Important**: Destructive actions (`kill_process`, `kill_by_name`) are NOT
/// executed server-side. Instead, they return a deferred instruction so the
/// frontend can present a confirmation dialog before dispatching the actual
/// `kill_process` / `kill_processes` IPC command. This prevents silent kills
/// that bypass user consent.
pub fn execute_tool_call(
    call_tool: &str,
    args: &serde_json::Value,
    state: &crate::watcher::SystemState,
) -> ToolResult {
    match call_tool {
        "kill_process" => {
            let pid = args["pid"].as_u64().unwrap_or(0) as u32;
            if pid == 0 {
                return ToolResult {
                    tool: "kill_process".into(),
                    success: false,
                    details: "Invalid PID".into(),
                };
            }
            // Verify PID exists in current state — but do NOT kill it here.
            // The frontend must confirm and dispatch the IPC kill command.
            let proc_info = state.cached_process_info.iter().find(|p| p.pid == pid);
            let proc_name = proc_info
                .map(|p| p.name.as_str())
                .unwrap_or("unknown");

            if proc_info.is_none() {
                return ToolResult {
                    tool: "kill_process".into(),
                    success: false,
                    details: format!("Process with PID {} not found in current state", pid),
                };
            }

            ToolResult {
                tool: "kill_process".into(),
                success: true,
                details: format!("kill_process:{}:{}", pid, proc_name),
            }
        }
        "kill_by_name" => {
            let name = args["name"].as_str().unwrap_or("");
            if name.is_empty() {
                return ToolResult {
                    tool: "kill_by_name".into(),
                    success: false,
                    details: "No process name provided".into(),
                };
            }
            let name_lower = name.to_lowercase();
            let matching_pids: Vec<u32> = state
                .cached_process_info
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&name_lower))
                .map(|p| p.pid)
                .collect();

            if matching_pids.is_empty() {
                return ToolResult {
                    tool: "kill_by_name".into(),
                    success: false,
                    details: format!("No processes found matching '{}'", name),
                };
            }

            let pids_csv = matching_pids
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(",");

            ToolResult {
                tool: "kill_by_name".into(),
                success: true,
                details: format!("kill_by_name:{}:{}", name, pids_csv),
            }
        }
        "close_tabs" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let except = args.get("except").and_then(|v| v.as_str()).unwrap_or("");

            if !except.is_empty() {
                // Exclusion mode: close all tabs EXCEPT those matching
                ToolResult {
                    tool: "close_tabs".into(),
                    success: true,
                    details: format!("close_tabs_except:{}", except),
                }
            } else if !pattern.is_empty() {
                // Positive mode: close tabs that match
                ToolResult {
                    tool: "close_tabs".into(),
                    success: true,
                    details: format!("close_tabs:{}", pattern),
                }
            } else {
                ToolResult {
                    tool: "close_tabs".into(),
                    success: false,
                    details: "No pattern or except provided".into(),
                }
            }
        }
        "add_automation_rule" => {
            let id = args["id"].as_str().unwrap_or("").to_string();
            let process_pattern = args["process_pattern"].as_str().unwrap_or("").to_string();
            let metric = args["metric"].as_str().unwrap_or("cpu").to_string();
            let threshold = args["threshold"].as_f64().unwrap_or(0.0);
            let duration_secs = args["duration_secs"].as_u64().unwrap_or(30);
            let action = args["action"].as_str().unwrap_or("alert").to_string();

            if id.is_empty() || process_pattern.is_empty() {
                return ToolResult {
                    tool: "add_automation_rule".into(),
                    success: false,
                    details: "Missing required fields: id and process_pattern".into(),
                };
            }

            let rule_json = serde_json::json!([{
                "id": id,
                "process_pattern": process_pattern,
                "metric": metric,
                "threshold": threshold,
                "duration_secs": duration_secs,
                "action": action,
            }]);

            match crate::rules_engine::upsert_rules_from_ai_json(&rule_json.to_string()) {
                Ok(count) => ToolResult {
                    tool: "add_automation_rule".into(),
                    success: true,
                    details: format!(
                        "Added {} automation rule(s): {} on {} {} > {}",
                        count, id, process_pattern, metric, threshold
                    ),
                },
                Err(e) => ToolResult {
                    tool: "add_automation_rule".into(),
                    success: false,
                    details: format!("Failed to add rule: {}", e),
                },
            }
        }
        "remove_automation_rule" => {
            let id = args["id"].as_str().unwrap_or("");
            if id.is_empty() {
                return ToolResult {
                    tool: "remove_automation_rule".into(),
                    success: false,
                    details: "Missing required field: id".into(),
                };
            }
            match crate::rules_engine::remove_rule_by_id(id) {
                Ok(removed) => ToolResult {
                    tool: "remove_automation_rule".into(),
                    success: removed,
                    details: if removed {
                        format!("Removed automation rule '{}'", id)
                    } else {
                        format!("Rule '{}' not found", id)
                    },
                },
                Err(e) => ToolResult {
                    tool: "remove_automation_rule".into(),
                    success: false,
                    details: format!("Failed to remove rule: {}", e),
                },
            }
        }
        _ => ToolResult {
            tool: call_tool.into(),
            success: false,
            details: format!("Unknown tool: {}", call_tool),
        },
    }
}

/// Send a chat message to the AI and optionally execute tool calls.
pub async fn chat_with_tools(
    provider: AiProvider,
    model: &str,
    api_key: &str,
    messages: &[(String, String)],
    system_prompt: &str,
) -> Result<(String, Option<RawToolCall>), Box<dyn Error + Send + Sync>> {
    if let Some((_, last_user_msg)) = messages.last() {
        check_prompt_injection(last_user_msg)?;
    }

    let cache_key = calculate_hash(&(provider as u8, model, messages, system_prompt));
    if let Ok(cache) = get_ai_cache().read() {
        if let Some(cached_response) = cache.get(&cache_key) {
            let tool_call = parse_tool_call(cached_response);
            return Ok((cached_response.clone(), tool_call));
        }
    }

    let client = build_client()?;

    // Build the messages array from history (role, content) pairs
    let msg_array: Vec<serde_json::Value> = messages
        .iter()
        .map(|(role, content)| serde_json::json!({"role": role, "content": content}))
        .collect();

    // Log payload size for debugging
    let system_len = system_prompt.len();
    let history_len: usize = messages.iter().map(|(r, c)| r.len() + c.len()).sum();
    eprintln!("[ai-chat] provider={provider:?} model={model} system_prompt_len={system_len} history_msgs={} history_bytes={history_len}", messages.len());

    let ai_text = if provider == AiProvider::Anthropic {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": MAX_TOKENS_CHAT,
            "system": system_prompt,
            "messages": msg_array
        });
        let resp = send_with_retry(|| {
            add_anthropic_headers(client.post(AiProvider::Anthropic.api_url()), api_key).json(&body)
        })
        .await?;
        let status = resp.status();
        eprintln!("[ai-chat] anthropic response status={}", status.as_u16());
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            eprintln!(
                "[ai-chat] ERROR body: {}",
                &body_text[..body_text.len().min(500)]
            );
            return Err(format!(
                "AI request failed (status {}): {}",
                status.as_u16(),
                body_text.chars().take(200).collect::<String>()
            )
            .into());
        }
        let resp_text = resp.text().await?;
        eprintln!("[ai-chat] response_len={}", resp_text.len());
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        resp_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        // OpenAI-compatible (OpenAI, OpenRouter, Gemini, Ollama)
        let mut openai_msgs = vec![serde_json::json!({"role": "system", "content": system_prompt})];
        openai_msgs.extend(msg_array.iter().cloned());
        let body = serde_json::json!({
            "model": model,
            "messages": openai_msgs
        });
        let resp = send_with_retry(|| {
            let mut req = client.post(provider.api_url());
            if provider != AiProvider::Ollama {
                req = req.header("Authorization", format!("Bearer {}", api_key));
            }
            if provider == AiProvider::OpenRouter {
                req = add_openrouter_headers(req);
            }
            req.json(&body)
        })
        .await?;
        let status = resp.status();
        eprintln!(
            "[ai-chat] openai-compat response status={}",
            status.as_u16()
        );
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            eprintln!(
                "[ai-chat] ERROR body: {}",
                &body_text[..body_text.len().min(500)]
            );
            return Err(format!(
                "AI request failed (status {}): {}",
                status.as_u16(),
                body_text.chars().take(200).collect::<String>()
            )
            .into());
        }
        let resp_text = resp.text().await?;
        eprintln!("[ai-chat] response_len={}", resp_text.len());
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        resp_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    if let Ok(mut cache) = get_ai_cache().write() {
        cache.insert(cache_key, ai_text.clone());
    }

    let tool_call = parse_tool_call(&ai_text);
    Ok((ai_text, tool_call))
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
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

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
        assert_eq!(AiProvider::from_str("ollama").unwrap(), AiProvider::Ollama);
        assert!(AiProvider::from_str("unknown").is_err());
    }

    #[test]
    fn ai_provider_keyring_services_are_distinct() {
        let services: Vec<&str> = [
            AiProvider::OpenRouter,
            AiProvider::OpenAI,
            AiProvider::Gemini,
            AiProvider::Anthropic,
            AiProvider::Ollama,
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
    fn ai_provider_api_urls_are_https_or_localhost() {
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
        // Ollama runs locally — http is expected
        assert!(AiProvider::Ollama.api_url().starts_with("http://localhost"));
    }

    #[test]
    fn ollama_does_not_require_api_key() {
        assert!(!AiProvider::Ollama.requires_api_key());
        assert!(AiProvider::OpenAI.requires_api_key());
        assert!(AiProvider::Anthropic.requires_api_key());
    }

    #[test]
    fn parse_tool_call_extracts_kill_process() {
        let text = r#"I'll kill that process for you. {"tool": "kill_process", "args": {"pid": 1234}, "reason": "user requested"}"#;
        let call = parse_tool_call(text).expect("should parse tool call");
        assert_eq!(call.tool, "kill_process");
        assert_eq!(call.args["pid"], 1234);
    }

    #[test]
    fn parse_tool_call_extracts_kill_by_name() {
        let text =
            r#"{"tool": "kill_by_name", "args": {"name": "Chrome"}, "reason": "closing browser"}"#;
        let call = parse_tool_call(text).expect("should parse");
        assert_eq!(call.tool, "kill_by_name");
        assert_eq!(call.args["name"], "Chrome");
    }

    #[test]
    fn parse_tool_call_rejects_unknown_tools() {
        let text = r#"{"tool": "format_disk", "args": {}, "reason": "evil"}"#;
        assert!(parse_tool_call(text).is_none());
    }

    #[test]
    fn parse_tool_call_returns_none_for_plain_text() {
        assert!(parse_tool_call("Your system looks healthy. No action needed.").is_none());
    }

    #[test]
    fn build_chat_system_prompt_contains_state() {
        let state = crate::watcher::SystemState {
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            used_memory_bytes: 8 * 1024 * 1024 * 1024,
            cpu_usage_percent: 45.0,
            ..Default::default()
        };
        let prompt = build_chat_system_prompt(&state);
        assert!(prompt.contains("RAM:"));
        assert!(prompt.contains("kill_process"));
        assert!(prompt.contains("kill_by_name"));
        assert!(prompt.contains("close_tabs"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("AI service unavailable after"));
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
        assert!(!(*saved.lock().unwrap()));
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
