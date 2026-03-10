# OmniMon Commands Reference

Version: `6.2.0`

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
| `omnimon status [--format text\|json]` | Show current system state and top processes | `omnimon status --format json` |
| `omnimon kill <PID>` | Kill one process safely by PID | `omnimon kill 4242` |
| `omnimon optimize --ai <provider> [--target <profile>]` | Ask an AI provider for process optimization suggestions | `omnimon optimize --ai openai --target browsers` |
| `omnimon tabs list` | List open browser tabs | `omnimon tabs list` |
| `omnimon tabs close --browser <BROWSER> [--id <ID>] [--url <URL>]` | Close one browser tab | `omnimon tabs close --browser Chrome --url "https://example.com"` |
| `omnimon tabs focus --browser <BROWSER> [--id <ID>] [--url <URL>]` | Focus one browser tab | `omnimon tabs focus --browser Arc --url "https://github.com"` |
| `omnimon chat --ai <provider> <prompt>` | Send free-form context to the AI assistant | `omnimon chat --ai gemini "What is using the most RAM?"` |
| `omnimon apikey --ai <provider> <key>` | Validate and save an AI API key | `omnimon apikey --ai anthropic "sk-ant-..."` |
| `omnimon settings get` | Read saved settings | `omnimon settings get` |
| `omnimon settings set <key> <value>` | Update one setting | `omnimon settings set idle-threshold 2.5` |
| `omnimon settings presets` | List all available profile presets | `omnimon settings presets` |
| `omnimon settings use <id>` | Apply a shared profile preset by ID | `omnimon settings use battery-saver` |
| `omnimon auth login <key>` | Save CrabNebula auth key | `omnimon auth login "cn_live_..."` |
| `omnimon cloud sync --report-path <path>` | Upload an encrypted report | `omnimon cloud sync --report-path /tmp/omnimon_scan_report.enc` |
| `omnimon security-scan [--cve-db <path>]` | Run a local security scan | `omnimon security-scan --cve-db ./cves.json` |
| `omnimon doctor` | Check platform, drivers, and keyring health | `omnimon doctor` |
| `omnimon tui` | Launch the terminal UI | `omnimon tui` |
| `omnimon config rotate-key` | Rotate the scan encryption key (NIST SC-12) | `omnimon config rotate-key` |
| `omnimon network [--format text\|json]` | Show live network throughput per process | `omnimon network --top` |
| `omnimon network --connections` | Show filtered connection snapshots from network analysis | `omnimon network --connections` |
| `omnimon network --filter <tcp\|udp\|icmp\|other> --port <PORT>` | Filter connection snapshots by protocol and port | `omnimon network --filter tcp --port 443` |
| `omnimon network --alerts` | Show evaluated network alerts from watcher state | `omnimon network --alerts` |
| `omnimon network --top` | Force top-throughput view explicitly (top 10 processes) | `omnimon network --top --format json` |
| `omnimon network --watch [--watch-interval-ms <MS>] [--watch-iterations <N>]` | Refresh the selected network view continuously | `omnimon network --watch --watch-interval-ms 1000` |
| `omnimon rules list` | List all active AI security rules | `omnimon rules list` |
| `omnimon rules load <path>` | Load rules from a JSON file (schema v1) | `omnimon rules load ./rules.json` |
| `omnimon rules remove <id>` | Remove a rule by ID | `omnimon rules remove proc-mem-004` |
| `omnimon rules schema` | Print the expected JSON schema for rules | `omnimon rules schema` |
| `omnimon release generate-keypair` | Generate Ed25519 signing keypair | `omnimon release generate-keypair` |
| `omnimon release sign --version <VER> <file>` | Sign a release artifact with Ed25519 | `omnimon release sign --version 6.2.0 ./omnimon` |
| `omnimon release verify --sig <path> <file>` | Verify artifact signature | `omnimon release verify --sig ./omnimon.sig.json ./omnimon` |
| `omnimon release checksum <file>` | Compute SHA-256 checksum | `omnimon release checksum ./omnimon` |
| `omnimon release manifest --version <VER> --dir <dir>` | Generate release manifest with signatures | `omnimon release manifest --version 6.2.0 --dir ./dist` |
| `omnimon release verify-manifest --pubkey <key> <file>` | Verify a release manifest | `omnimon release verify-manifest --pubkey <b64> releases.json` |

## Tauri IPC Commands

| Command | Parameters | Response |
| --- | --- | --- |
| `get_metrics` | `idleThreshold?: number` | `Metrics` object with `processes[]` and `stats` |
| `get_network_data` | none | JSON object with `top_processes`, `recent_connections`, aggregate throughput, backend, and DPI flag |
| `get_network_connections` | none | `NetworkSnapshot` with current connection list and aggregate per-process analysis |
| `get_network_history` | `seconds: number` | `NetworkSnapshot[]` for the requested recent interval |
| `get_filtered_connections` | `filter: NetworkFilter` | `NetworkConnection[]` matching protocol/port/process/host filters |
| `kill_process` | `pid: number` | `boolean` |
| `kill_processes` | `pids: number[]` (max 50) | `{ killed: number[], failed: [number, string][] }` |
| `save_ai_config` | `provider: string`, `model: string`, `key: string` | `void` |
| `check_api_key` | `provider: string` | `boolean` |
| `apply_ai_rules` | `payload: string` (max 64KB) | `number` of rules applied |
| `get_ai_rules_schema` | none | JSON schema string |
| `set_network_alert_rules` | `payload_json: string` (max 128KB) | `number` of rules configured |
| `validate_api_key` | `provider: string`, `key: string` | `boolean` |
| `analyze_processes` | `profile: string`, `provider: string`, `model: string` | `ProcessSuggestion[]` |
| `analyze_context` | `context: string`, `provider: string`, `model: string` | plain text `string` |
| `ai_chat` | `message: string`, `provider: string`, `model: string`, `history: [string, string][]`, `cache_ttl_minutes?: number` | `ChatResponse` with `reply` and optional `tool_call` |
| `clear_ai_cache` | none | `void` |
| `get_browser_tabs` | none | `BrowserTab[]` |
| `close_browser_tab` | `tabId: string`, `tabUrl: string`, `browser: string` | `boolean` |
| `focus_browser_tab` | `tabId: string`, `tabUrl: string`, `browser: string` | `boolean` |
| `get_window_visible` | none | `boolean` |
| `save_cloud_key` | `key: string` | `void` |
| `get_cloud_key` | none | `string` |
| `get_automation_rules` | none | `AutomationRule[]` |
| `add_automation_rule` | `rule: AutomationRule` | `void` |
| `remove_automation_rule` | `id: string` | `void` |
| `list_plugins` | none | `PluginDescriptor[]` |
| `install_plugin` | `fileName: string`, `source: string` (max 256KB) | `PluginDescriptor` |
| `set_plugin_enabled` | `pluginId: string`, `enabled: boolean` | `PluginDescriptor` |
| `remove_plugin` | `pluginId: string` | `void` |

## Readable Output Expectations

- CLI and AI chat should present human-readable text by default for user-facing flows.
- JSON output is explicit and opt-in only, for example `omnimon status --format json`.
- AI tool calls are internal transport objects; the end user should see the natural-language reply and the action result, not the raw JSON envelope.
- Network analysis commands default to a top-throughput summary; `--connections`, `--alerts`, and `--filter/--port` switch the view without requiring a separate subcommand.
