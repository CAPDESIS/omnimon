# AI Architecture — Ollama Local + Interactive Chat with Tool Calling

## Overview

This document describes the architecture for three interconnected features:
1. **Ollama Local Provider** — 100% local AI inference for corporate privacy
2. **AIChat Interactive Interface** — Natural language system control
3. **Tool Calling Engine** — AI-driven actions on the real OS

## Data Flow

```
┌─────────────┐     IPC (invoke)      ┌──────────────────┐
│  AIChat.svelte│ ──────────────────► │  ai_chat command  │
│  (Svelte 5)  │                      │  (src-tauri/lib.rs)│
│              │ ◄────────────────── │                    │
│  Renders     │   ChatResponse       │  1. Build system   │
│  messages +  │   {text, actions}    │     prompt with    │
│  action      │                      │     live OS state  │
│  results     │                      │  2. Call LLM       │
└─────────────┘                      │  3. Parse tool     │
                                      │     calls from     │
                                      │     response       │
                                      │  4. Execute safe   │
                                      │     actions        │
                                      └────────┬───────────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          │                    │                    │
                    ┌─────▼─────┐     ┌───────▼───────┐   ┌──────▼──────┐
                    │  Ollama    │     │  OpenAI /     │   │  Anthropic  │
                    │ localhost  │     │  OpenRouter   │   │  API        │
                    │  :11434    │     │  / Gemini     │   │             │
                    └───────────┘     └───────────────┘   └─────────────┘
```

## 1. Ollama Provider (Rust — ai.rs)

- New variant `AiProvider::Ollama` with endpoint `http://localhost:11434/api/chat`
- No API key required — uses OpenAI-compatible `/api/chat` endpoint
- Default model: `llama3.2` (configurable from frontend)
- Ollama uses OpenAI-compatible format for chat completions at `/v1/chat/completions`
- Validation: HTTP GET to `http://localhost:11434/api/tags` to check connectivity

## 2. System Prompt with OS State Injection

The `ai_chat` Tauri command builds a rich system prompt containing:

```
You are OmniMon, a system monitor assistant. You have access to the following
tools to control the system. When the user asks you to perform an action,
respond with a JSON tool call.

## Current System State
- RAM: {used}GB / {total}GB ({pct}%)
- CPU: {cpu_pct}%
- Top processes: [{pid, name, memory_mb, cpu_pct}, ...]
- Network: RX {rx}/s, TX {tx}/s

## Available Tools
- kill_process: {pid: number} — Kill a process by PID
- kill_by_name: {name: string} — Kill all processes matching name
- close_tabs: {pattern: string} — Close browser tabs matching URL pattern

Respond with EITHER:
1. A JSON object: {"tool": "<name>", "args": {...}, "reason": "..."}
2. Plain text analysis if no action is needed
```

## 3. Tool Calling Flow

1. User types natural language command in AIChat
2. Backend injects live `SystemState` into system prompt
3. LLM responds with either plain text or JSON tool call
4. Rust parses response, validates tool call against allowlist
5. If tool call detected: execute action (e.g., `kill_process_safe`)
6. Return `ChatResponse` with both AI text and action results

## 4. Security

- Tool calls are validated against a hardcoded allowlist
- Protected processes (kernel, launchd, etc.) cannot be killed
- PIDs are verified against current process list before action
- All actions are logged to audit trail
- Prompt injection detection runs on user input (existing `detectPromptInjection`)

## 5. IPC Contract

```typescript
// Frontend → Backend
invoke("ai_chat", { message: string, provider: string, model: string })

// Backend → Frontend
interface ChatResponse {
  reply: string;           // AI text response
  tool_call: ToolResult | null;  // Action result if tool was called
}

interface ToolResult {
  tool: string;          // "kill_process" | "kill_by_name" | "close_tabs"
  success: boolean;
  details: string;       // Human-readable result
}
```
