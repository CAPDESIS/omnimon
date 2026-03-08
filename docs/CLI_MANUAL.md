# OmniMon CLI Manual (v5.2.0)

## Overview
The OmniMon CLI allows you to interact with the core system monitoring and management capabilities directly from your terminal. It offers full parity with the GUI features, allowing you to check system status, manage processes, analyze system state using AI, manage browser tabs, and sync configurations.

## Commands

### 1. `status`
Get the real-time status of your system resources and the top processes by memory usage.

**Usage:**
```bash
omnimon status [--format <FORMAT>]
```

**Options:**
- `--format <FORMAT>`: Output format. Options: `text`, `json`. Default: `text`.

**Examples:**
```bash
# Text format
omnimon status

# JSON format
omnimon status --format json
```

### 2. `kill`
Kill a process using its Process ID (PID). This respects system protections and prevents killing immutable critical processes.

**Usage:**
```bash
omnimon kill <PID>
```

**Examples:**
```bash
omnimon kill 12345
```

### 3. `optimize`
Analyze your system's current processes using AI and receive suggestions on which processes can be safely closed to optimize performance.

**Usage:**
```bash
omnimon optimize --ai <PROVIDER> [--target <TARGET>]
```

**Options:**
- `--ai <PROVIDER>`: The AI provider to use. Options: `openai`, `anthropic`, `openrouter`, `gemini`.
- `--target <TARGET>`: The target profile to optimize (e.g. `browsers`, `all`). Default: `all`.

**Examples:**
```bash
omnimon optimize --ai openrouter --target all
```

### 4. `tabs`
Manage browser tabs across installed and supported browsers.

**Subcommands:**
- `list`: List all open browser tabs.
- `close --browser <BROWSER> [--id <ID>] [--url <URL>]`: Close a specific tab.
- `focus --browser <BROWSER> [--id <ID>] [--url <URL>]`: Focus a specific tab.

**Examples:**
```bash
omnimon tabs list
omnimon tabs close --browser Chrome --url "https://example.com"
```

### 5. `chat`
Send an arbitrary context or prompt to the AI Assistant.

**Usage:**
```bash
omnimon chat --ai <PROVIDER> <PROMPT>
```

**Examples:**
```bash
omnimon chat --ai anthropic "How do I clear my DNS cache?"
```

### 6. `apikey`
Save and validate an API Key for a specific AI Provider securely in the native OS keyring.

**Usage:**
```bash
omnimon apikey --ai <PROVIDER> <KEY>
```

**Examples:**
```bash
omnimon apikey --ai openai "sk-..."
```

### 7. `settings`
Read or update application settings. These settings mirror the GUI preferences.

**Subcommands:**
- `get`: Show all current settings.
- `set <KEY> <VALUE>`: Set a specific configuration key to a new value. Supported keys: `theme`, `font-size`, `locale`, `idle-threshold`.

**Examples:**
```bash
omnimon settings get
omnimon settings set theme dark
```
