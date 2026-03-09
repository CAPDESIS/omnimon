# OmniMon Commands Reference

Version: `6.0.1`

## AI Chat Actions

These are the backend-supported actions that can be triggered from the in-app AI chat. OmniMon is designed to return readable text first; destructive actions require confirmation before execution.

| Action | General user description | Professional description | Example | Expected output |
| --- | --- | --- | --- | --- |
| `kill_process` | Close one app by PID after confirmation | Resolves a single PID and returns a deferred kill instruction for the frontend confirmation flow | "Cierra el proceso con PID 1234" | Plain-language confirmation preview, then success/failure text such as `Successfully killed process 'X' (PID 1234)` |
| `kill_by_name` | Close all matching apps by name after confirmation | Matches current cached process names and returns a PID set for batch confirmation | "Cierra todos los Chrome Helper" | Plain-language list of matching processes and readable batch result |
| `close_tabs` | Close browser tabs that match a title or URL | Supports `pattern` mode and `except` mode for selective multi-tab control | "Cierra YouTube y Netflix" / "Deja solo GitHub y Gemini" | Human-readable tab summary; no raw JSON shown to the end user |
| `add_automation_rule` | Create an alert or auto-action rule | Adds an automation rule with `id`, `process_pattern`, `metric`, `threshold`, `duration_secs`, and `action` | "Avísame si node supera 2 GB por 60 segundos" | Readable confirmation such as `Added automation rule successfully` |
| `remove_automation_rule` | Remove a saved automation rule | Deletes a rule by stable identifier | "Elimina la regla node-memory" | Readable confirmation such as `Removed automation rule successfully` |

## CLI Commands

| Command | Purpose | Example |
| --- | --- | --- |
| `omnimon status [--format text|json]` | Show current system state and top processes | `omnimon status --format json` |
| `omnimon kill <PID>` | Kill one process safely by PID | `omnimon kill 4242` |
| `omnimon optimize --ai <provider> [--target <profile>]` | Ask an AI provider for process optimization suggestions | `omnimon optimize --ai openai --target browsers` |
| `omnimon tabs list` | List open browser tabs | `omnimon tabs list` |
| `omnimon tabs close --browser <BROWSER> [--id <ID>] [--url <URL>]` | Close one browser tab | `omnimon tabs close --browser Chrome --url "https://example.com"` |
| `omnimon tabs focus --browser <BROWSER> [--id <ID>] [--url <URL>]` | Focus one browser tab | `omnimon tabs focus --browser Arc --url "https://github.com"` |
| `omnimon chat --ai <provider> <prompt>` | Send free-form context to the AI assistant | `omnimon chat --ai gemini "What is using the most RAM?"` |
| `omnimon apikey --ai <provider> <key>` | Validate and save an AI API key | `omnimon apikey --ai anthropic "sk-ant-..."` |
| `omnimon settings get` | Read saved settings | `omnimon settings get` |
| `omnimon settings set <key> <value>` | Update one setting | `omnimon settings set idle-threshold 2.5` |
| `omnimon auth login <key>` | Save CrabNebula auth key | `omnimon auth login "cn_live_..."` |
| `omnimon cloud sync --report-path <path>` | Upload an encrypted report | `omnimon cloud sync --report-path /tmp/omnimon_scan_report.enc` |
| `omnimon security-scan [--cve-db <path>]` | Run a local security scan | `omnimon security-scan --cve-db ./cves.json` |
| `omnimon doctor` | Check platform, drivers, and keyring health | `omnimon doctor` |
| `omnimon tui` | Launch the terminal UI | `omnimon tui` |

## Tauri IPC Commands

| Command | Parameters | Response |
| --- | --- | --- |
| `get_metrics` | `idleThreshold?: number` | `Metrics` object with `processes[]` and `stats` |
| `get_network_data` | none | JSON object with `top_processes`, `recent_connections`, aggregate throughput, backend, and DPI flag |
| `kill_process` | `pid: number` | `boolean` |
| `kill_processes` | `pids: number[]` | `{ killed: number[], failed: [number, string][] }` |
| `save_ai_config` | `provider: string`, `model: string`, `key: string` | `void` |
| `check_api_key` | `provider: string` | `boolean` |
| `apply_ai_rules` | `payload: string` | `number` of rules applied |
| `get_ai_rules_schema` | none | JSON schema string |
| `validate_api_key` | `provider: string`, `key: string` | `boolean` |
| `analyze_processes` | `profile: string`, `provider: string`, `model: string` | `ProcessSuggestion[]` |
| `analyze_context` | `context: string`, `provider: string`, `model: string` | plain text `string` |
| `ai_chat` | `message: string`, `provider: string`, `model: string`, `history: [string, string][]` | `ChatResponse` with `reply` and optional `tool_call` |
| `get_browser_tabs` | none | `BrowserTab[]` |
| `close_browser_tab` | `tabId: string`, `tabUrl: string`, `browser: string` | `boolean` |
| `focus_browser_tab` | `tabId: string`, `tabUrl: string`, `browser: string` | `boolean` |
| `get_window_visible` | none | `boolean` |
| `save_cloud_key` | `key: string` | `void` |
| `get_cloud_key` | none | `string` |
| `get_automation_rules` | none | `AutomationRule[]` |
| `add_automation_rule` | `rule: AutomationRule` or Tauri app + rule in Rust | `void` |
| `remove_automation_rule` | `id: string` | `void` |
| `list_plugins` | none | `PluginDescriptor[]` |
| `install_plugin` | `fileName: string`, `source: string` | `PluginDescriptor` |
| `set_plugin_enabled` | `pluginId: string`, `enabled: boolean` | `PluginDescriptor` |
| `remove_plugin` | `pluginId: string` | `void` |

## Readable Output Expectations

- CLI and AI chat should present human-readable text by default for user-facing flows.
- JSON output is explicit and opt-in only, for example `omnimon status --format json`.
- AI tool calls are internal transport objects; the end user should see the natural-language reply and the action result, not the raw JSON envelope.
