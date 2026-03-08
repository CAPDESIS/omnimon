//! Artificial Intelligence integration module. Handles communication with various LLM providers (OpenAI, Anthropic, Gemini, OpenRouter) for predictive system optimization and context analysis.

use keyring::Entry;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::future::Future;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 30;

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
            AiProvider::OpenRouter => "omnimon_openrouter",
            AiProvider::OpenAI => "omnimon_openai",
            AiProvider::Gemini => "omnimon_gemini",
            AiProvider::Anthropic => "omnimon_anthropic",
            AiProvider::Ollama => "omnimon_ollama",
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
            AiProvider::Ollama => "http://localhost:11434/v1/chat/completions",
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

    // Ollama: just check that the server is reachable (no API key needed)
    if provider == AiProvider::Ollama {
        let resp = client
            .get("http://localhost:11434/api/tags")
            .send()
            .await
            .map_err(|_| -> Box<dyn Error + Send + Sync> {
                "Ollama is not running — start it with `ollama serve`".into()
            })?;
        if !resp.status().is_success() {
            return Err("Ollama server returned an error".into());
        }
        return Ok(());
    }

    let resp = match provider {
        AiProvider::Anthropic => {
            client
                .post(url)
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .body(r#"{"model":"claude-haiku-4-5-20251001","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
                .send()
                .await?
        }
        AiProvider::OpenRouter => {
            client
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://github.com/chochy2001/omnimon")
                .header("X-Title", "OmniMon")
                .body(r#"{"model":"meta-llama/llama-3.2-3b-instruct:free","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
                .send()
                .await?
        }
        _ => {
            client
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .body(r#"{"model":"gpt-4o-mini","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
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
        "You are macmon, a system optimization assistant. The user's current profile is: {}. \
        Analyze these running processes and suggest which ones should be safely closed to free up resources. \
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
            req = req
                .header("HTTP-Referer", "https://github.com/chochy2001/omnimon")
                .header("X-Title", "OmniMon");
        }
        req.json(&body)
    })
    .await?;

    if resp.status().is_client_error() {
        let status = resp.status().as_u16();
        return Err(format!("AI request failed (status {})", status).into());
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
        let status = resp.status().as_u16();
        return Err(format!("AI request failed (status {})", status).into());
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
    analyze_context_key(provider, model, context, &api_key).await
}

/// Like `analyze_context` but accepts an explicit API key (for Tauri Store fallback).
pub async fn analyze_context_key(
    provider: AiProvider,
    model: &str,
    context: &str,
    api_key: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
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
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
        })
        .await?;
        if resp.status().is_client_error() {
            return Err(format!("AI request failed (status {})", resp.status().as_u16()).into());
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
        let mut req = client
            .post(provider.api_url())
            .header("Authorization", format!("Bearer {api_key}"));
        if provider == AiProvider::OpenRouter {
            req = req
                .header("HTTP-Referer", "https://github.com/chochy2001/omnimon")
                .header("X-Title", "OmniMon");
        }
        req.json(&body)
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
    let ram_total_gb = state.total_memory_bytes as f64 / 1_073_741_824.0;
    let ram_used_gb = state.used_memory_bytes as f64 / 1_073_741_824.0;
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
                p.memory_bytes as f64 / 1_048_576.0,
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
3. **close_tabs** - Close browser tabs by URL/title pattern. Args: {{"pattern": "<url_or_title_substring>"}}
   - Pattern matches against tab URL and title (case-insensitive substring).
   - Use pipe `|` to match multiple patterns: "youtube.com|reddit.com"
   - To close all tabs EXCEPT certain ones, list patterns for the tabs you WANT TO CLOSE.
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
6. Respond in the same language the user writes in."#,
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
            // Verify PID exists in current state
            let proc_info = state.cached_process_info.iter().find(|p| p.pid == pid);
            let proc_name = proc_info.map(|p| p.name.as_str()).unwrap_or("unknown");
            match crate::killer::kill_process_safe(pid as i32, &[]) {
                Ok(_) => ToolResult {
                    tool: "kill_process".into(),
                    success: true,
                    details: format!("Killed process {} (PID {})", proc_name, pid),
                },
                Err(e) => ToolResult {
                    tool: "kill_process".into(),
                    success: false,
                    details: format!("Failed to kill PID {}: {}", pid, e),
                },
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

            let mut killed = 0u32;
            let mut failed = 0u32;
            for pid in &matching_pids {
                match crate::killer::kill_process_safe(*pid as i32, &[]) {
                    Ok(_) => killed += 1,
                    Err(_) => failed += 1,
                }
            }
            ToolResult {
                tool: "kill_by_name".into(),
                success: killed > 0,
                details: format!(
                    "Killed {}/{} processes matching '{}'{}",
                    killed,
                    matching_pids.len(),
                    name,
                    if failed > 0 {
                        format!(" ({} failed — likely protected)", failed)
                    } else {
                        String::new()
                    }
                ),
            }
        }
        "close_tabs" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            if pattern.is_empty() {
                return ToolResult {
                    tool: "close_tabs".into(),
                    success: false,
                    details: "No URL pattern provided".into(),
                };
            }
            // Tab closing is handled by the frontend; return instruction
            ToolResult {
                tool: "close_tabs".into(),
                success: true,
                details: format!("close_tabs:{}", pattern),
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
    user_message: &str,
    system_prompt: &str,
) -> Result<(String, Option<RawToolCall>), Box<dyn Error + Send + Sync>> {
    let client = build_client()?;

    let ai_text = if provider == AiProvider::Anthropic {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 2048,
            "system": system_prompt,
            "messages": [{"role": "user", "content": user_message}]
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
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("AI request failed (status {}): {}", status.as_u16(), body_text.chars().take(200).collect::<String>()).into());
        }
        let resp_text = resp.text().await?;
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        resp_json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        // OpenAI-compatible (OpenAI, OpenRouter, Gemini, Ollama)
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message}
            ]
        });
        let resp = send_with_retry(|| {
            let mut req = client.post(provider.api_url());
            if provider != AiProvider::Ollama {
                req = req.header("Authorization", format!("Bearer {}", api_key));
            }
            if provider == AiProvider::OpenRouter {
                req = req
                    .header("HTTP-Referer", "https://github.com/chochy2001/omnimon")
                    .header("X-Title", "OmniMon");
            }
            req.json(&body)
        })
        .await?;
        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("AI request failed (status {}): {}", status.as_u16(), body_text.chars().take(200).collect::<String>()).into());
        }
        let resp_text = resp.text().await?;
        let resp_json: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("Invalid JSON from AI provider: {e}"))?;
        resp_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

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
