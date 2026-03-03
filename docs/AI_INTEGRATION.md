# AI Integration

macmon supports optional AI-assisted process analysis through external providers. The AI layer is strictly read-only — it suggests processes to close, but never executes commands directly.

## How It Works

1. Open Preferences from the menu bar and configure your provider, model, and API key
2. API keys are stored securely in the macOS Keychain (never in plain text files)
3. Press "Smart Optimize" in the Process Picker
4. macmon sends a lightweight process snapshot to your chosen provider
5. The provider returns a list of candidate PIDs as JSON
6. Suggested processes are highlighted in the table for your review
7. You choose which processes to close — nothing happens without explicit approval
8. Selected PIDs go through the same safety checks as manual selection (protected process list, code signature verification, graceful shutdown)

## Safety

- AI output is treated as untrusted input at every stage
- Only numeric PIDs are extracted from responses — no commands are ever executed
- Protected processes (WindowServer, kernel_task, launchd, etc.) cannot be selected regardless of AI suggestions
- Apple code signature verification applies to all system process names

## Supported Providers

| Provider | Endpoint |
|----------|----------|
| OpenAI | `api.openai.com/v1/chat/completions` |
| Anthropic | `api.anthropic.com/v1/messages` |
| OpenRouter | `openrouter.ai/api/v1/chat/completions` |

## Keychain Storage

- Service: `com.macmon.ai`
- Account: provider name (`openai`, `anthropic`, `openrouter`)
- Accessibility: `kSecAttrAccessibleWhenUnlocked`
