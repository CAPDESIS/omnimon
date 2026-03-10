# OmniMon CLI Manual (v6.2.0)

## Overview

The OmniMon CLI provides full access to system monitoring, AI optimization, network analysis, security auditing, and release management directly from your terminal. It offers complete parity with the desktop GUI.

## Global Options

- `--sync-keychain`: Force a credential presence check across supported AI providers before running the selected subcommand.

## Commands

### 1. `status`

Get real-time system resources and top processes by memory.

```bash
omnimon status [--format <FORMAT>]
```

**Options:**
- `--format <FORMAT>`: `text` (default) or `json`.

**Examples:**
```bash
omnimon status
omnimon status --format json
```

### 2. `kill`

Kill a process by PID. Respects system protections and immutable blocklists.

```bash
omnimon kill <PID>
```

**Examples:**
```bash
omnimon kill 12345
```

### 3. `optimize`

Analyze processes with AI and receive optimization suggestions.

```bash
omnimon optimize --ai <PROVIDER> [--target <TARGET>]
```

**Options:**
- `--ai <PROVIDER>`: `openai`, `anthropic`, `openrouter`, or `gemini` (required).
- `--target <TARGET>`: Profile to optimize (e.g. `browsers`, `all`). Default: `all`.

**Default models:** gpt-4o-mini (OpenAI), claude-haiku-4-5 (Anthropic), llama-3.2-3b (OpenRouter), gemini-2.0-flash (Gemini).

**Examples:**
```bash
omnimon optimize --ai anthropic --target browsers
omnimon optimize --ai openrouter --target all
```

### 4. `tabs`

Manage browser tabs across Chrome, Safari, Brave, Edge, Arc, and Firefox.

**Subcommands:**
- `list` — List all open browser tabs.
- `close --browser <BROWSER> [--id <ID>] [--url <URL>]` — Close a tab.
- `focus --browser <BROWSER> [--id <ID>] [--url <URL>]` — Focus a tab.

**Examples:**
```bash
omnimon tabs list
omnimon tabs close --browser Chrome --url "https://example.com"
omnimon tabs focus --browser Arc --url "https://github.com"
```

### 5. `chat`

Send a prompt to the AI assistant with live system context.

```bash
omnimon chat --ai <PROVIDER> <PROMPT>
```

**Examples:**
```bash
omnimon chat --ai anthropic "How do I clear my DNS cache?"
omnimon chat --ai openai "What is using the most RAM?"
```

### 6. `apikey`

Save and validate an API key for an AI provider in the native OS keyring.

```bash
omnimon apikey --ai <PROVIDER> <KEY>
```

**Examples:**
```bash
omnimon apikey --ai openai "sk-..."
omnimon apikey --ai anthropic "sk-ant-..."
```

### 7. `settings`

Read or update application settings.

**Subcommands:**
- `get` — Show all current settings.
- `set <KEY> <VALUE>` — Update a setting.
- `presets` — List all available profile presets.
- `use <ID>` — Apply a preset by ID.

**Supported keys:** `theme`, `font-size`, `locale`, `idle-threshold`, `ai-profile`, `poll-interval-ms`, `automation-interval-secs`, `active-profile-preset`.

**Examples:**
```bash
omnimon settings get
omnimon settings set theme dark
omnimon settings set idle-threshold 2.5
omnimon settings presets
omnimon settings use battery-saver
```

### 8. `auth`

Store the CrabNebula API key in the OS keyring.

```bash
omnimon auth login <KEY>
```

**Examples:**
```bash
omnimon auth login "cn_live_..."
```

### 9. `cloud`

Sync encrypted security reports to CrabNebula Cloud.

```bash
omnimon cloud sync --report-path <PATH>
```

**Examples:**
```bash
omnimon cloud sync --report-path /tmp/omnimon_scan_report.enc
```

### 10. `security-scan`

Run a local security scan against an optional CVE database.

```bash
omnimon security-scan [--cve-db <PATH>]
```

Scans top 50 processes by memory, matches against CVE database, generates an encrypted security heartbeat, and saves to a temporary AES-256-GCM encrypted report.

**Examples:**
```bash
omnimon security-scan
omnimon security-scan --cve-db ./fixtures/cves.json
```

### 11. `doctor`

Run environment, driver, and keyring health checks.

```bash
omnimon doctor
```

**Checks:**
- Operating system and architecture
- CLI version
- Network capture driver (libpcap/WinDivert/eBPF)
- Native OS keyring accessibility

### 12. `tui`

Launch the interactive terminal UI (htop-style dashboard).

```bash
omnimon tui
```

Features: process table with sorting, system gauges (CPU/MEM/NET), built-in AI chat. See [v6_tui_architecture.md](v6_tui_architecture.md) for details.

### 13. `config`

Manage cryptographic key configuration.

**Subcommands:**
- `rotate-key` — Rotate the scan encryption key (NIST SC-12 key rotation).

Generates a new AES-256-GCM key, stores it in the OS keyring, and re-encrypts existing security scan reports.

```bash
omnimon config rotate-key
```

### 14. `network`

Real-time network telemetry: throughput per process, connections, and alerts.

```bash
omnimon network [OPTIONS]
```

**Options:**
- `--format <FORMAT>`: `text` (default) or `json`.
- `--connections`: Show recent connection events.
- `--filter <PROTOCOL>`: Filter by `tcp`, `udp`, `icmp`, or `other` (implies connections view).
- `--port <PORT>`: Filter by local or remote port (implies connections view).
- `--alerts`: Show evaluated network alerts from watcher.
- `--top`: Show top per-process throughput (default if no flags).
- `--watch`: Refresh continuously until Ctrl+C.
- `--watch-interval-ms <MS>`: Refresh interval (default: 2000ms, requires --watch).
- `--watch-iterations <N>`: Limit iterations for automation (requires --watch).

**Views:**

| Flag | Output |
|------|--------|
| `--top` (default) | PID, Process, RX/s, TX/s, TCP pkts/s, UDP pkts/s |
| `--connections` | PID, Local IP:Port, Remote Host:Port, Protocol, State |
| `--alerts` | Severity, Rule ID, Destination, Message |

**Examples:**
```bash
omnimon network --top
omnimon network --connections
omnimon network --filter tcp --port 443
omnimon network --alerts
omnimon network --watch --watch-interval-ms 1000
omnimon network --top --format json
```

See [NETWORK_ANALYSIS.md](NETWORK_ANALYSIS.md) for detailed workflows.

### 15. `rules`

Manage AI-driven security alert rules (MITRE ATT&CK mapping).

**Subcommands:**
- `list` — List all active rules with ID, name, kind, and enabled status.
- `load <PATH>` — Load rules from a JSON file (schema version 1).
- `remove <ID>` — Remove a rule by ID.
- `schema` — Print the expected JSON schema for rule payloads.

**Rule kinds:** `process_country`, `process_ip`, `process_cidr`, `process_port`, `process_memory`.

**Examples:**
```bash
omnimon rules list
omnimon rules load ./security-rules.json
omnimon rules remove proc-mem-004
omnimon rules schema
```

### 16. `release`

Release signing, verification, and manifest generation (NIST SI-7).

**Subcommands:**

#### `generate-keypair`
Generate a new Ed25519 signing keypair. Private key is stored in the OS keyring; public key is printed to stdout (base64).

```bash
omnimon release generate-keypair
```

#### `sign --version <VERSION> [--key-file <PATH>] <FILE>`
Sign a release artifact.

```bash
omnimon release sign --version 6.2.0 ./target/release/omnimon
omnimon release sign --version 6.2.0 --key-file ./signing.key ./omnimon.dmg
```

Produces a `.sig.json` file alongside the artifact.

#### `verify --sig <PATH> [--pubkey <KEY>] <FILE>`
Verify an artifact's Ed25519 signature.

```bash
omnimon release verify --sig ./omnimon.sig.json ./omnimon
omnimon release verify --sig ./omnimon.sig.json --pubkey <base64-key> ./omnimon
```

#### `checksum <FILE>`
Compute SHA-256 checksum.

```bash
omnimon release checksum ./omnimon
# Output: a1b2c3...  omnimon
```

#### `manifest --version <VERSION> --dir <DIR> [--output <PATH>] [--key-file <PATH>]`
Generate a release manifest (`releases.json`) for all artifacts in a directory. Auto-detects platform and architecture, signs each artifact, and creates a comprehensive manifest.

```bash
omnimon release manifest --version 6.2.0 --dir ./dist
omnimon release manifest --version 6.2.0 --dir ./dist --output ./releases.json
```

#### `verify-manifest --pubkey <KEY> <FILE>`
Verify a release manifest.

```bash
omnimon release verify-manifest --pubkey <base64-key> ./releases.json
```
