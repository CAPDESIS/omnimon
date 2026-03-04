# macmon

A lightweight macOS system monitor that watches RAM pressure, swap usage, custom process thresholds, orphan build daemons, and idle processes. Includes a native AppKit process picker, a menu bar monitor, a background daemon, and a CLI — all built without Xcode.

[![CI](https://github.com/chochy2001/macmon/actions/workflows/ci.yml/badge.svg)](https://github.com/chochy2001/macmon/actions/workflows/ci.yml)

<!-- SCREENSHOT: Process Picker (main window)
     Capture the full ProcessPicker window showing:
     - The memory pressure gauge at the top (green/yellow/red bar)
     - The search field
     - The table with columns: checkbox, Name, RAM, CPU, Uptime, Disk R/W, State
     - Several processes listed with at least one group expanded
     - The summary bar at the bottom showing selected count and RAM
     Window size: 1100x600, use a mix of apps (Chrome tabs, Xcode, etc.)
     Save as: docs/images/picker.png
-->
![Process Picker](docs/images/picker.png)

## Features

- **Background daemon** — monitors memory pressure, swap, and process accumulation with configurable thresholds, cooldowns, and native macOS notifications
- **Native process picker** — AppKit-based UI with search, grouping, sorting, memory pressure gauge, disk I/O columns, and batch process closing
- **Live picker refresh** — process table auto-refreshes every 5 seconds while open (keeps selections by PID)
- **Menu bar monitor** — NSStatusItem showing live RAM/swap usage with quick access to the picker
- **CLI** — `macmon status`, `macmon start/stop/restart`, `macmon config`, `macmon log`, `macmon export`
- **Guided config editor** — quick settings form in GUI for non-technical users + YAML preview
- **AI settings in GUI** — provider/model/API key configuration from `Preferences...` (Keychain-backed)
- **Multi-provider AI** — OpenAI, Anthropic, OpenRouter, and Gemini
- **Orphan daemon detection** — SourceKitService (when Xcode closes), Gradle daemons, hanging xcodebuild, zombie Android emulators
- **Disk I/O metrics** — per-process disk read/write via `proc_pid_rusage` (no root required)
- **Metrics export** — `macmon export json/csv` for profiling and `--peaks` for daily peak tracking
- **Security-first** — AppleScript injection sanitization, jq-based JSON construction, PID reuse verification, system process protection, code signature verification
- **Performance-optimized** — batched `ps` calls (3 instead of ~300), cached `memory_pressure`, NSTableView cell recycling
- **Chrome-aware** — closes Chrome tabs via AppleScript instead of killing renderer processes
- **Chrome tab context** — best-effort title/domain/URL enrichment plus "Show Chrome Tabs" action
- **Tested** — BATS test suite + Swift XCTests + GitHub Actions CI

### macmon vs Activity Monitor

| Feature | macmon | Activity Monitor |
|---------|:------:|:----------------:|
| Idle process detection (configurable CPU threshold) | Yes | No |
| Process grouping by app bundle | Yes | Partial |
| Working directory per process | Yes | No |
| Chrome tab titles | Yes | No |
| Orphan build daemon detection (SourceKit, Gradle, xcodebuild) | Yes | No |
| Dynamic per-process threshold alerts | Yes | No |
| Per-process disk I/O (lifetime read/write) | Yes | No |
| Metrics export (JSON/CSV) | Yes | No |
| Proactive memory pressure notifications | Yes | No |
| CLI system health summary | Yes | No |
| Menu bar quick-glance monitor | Yes | No |
| Search/filter processes by name, PID, or directory | Yes | Limited |
| Configurable thresholds and cooldowns | Yes | No |
| Keyboard shortcuts for batch operations | Yes | Limited |

## Requirements

- macOS 13+ (Ventura or later)
- `jq` — `brew install jq`
- Xcode Command Line Tools — `xcode-select --install`

## Installation

### Homebrew (recommended)

```bash
brew install chochy2001/tap/macmon
brew services start chochy2001/tap/macmon
```

This installs macmon, sets up the background daemon, and creates the `macmon` CLI in your PATH. Done.

### From source

```bash
git clone https://github.com/chochy2001/macmon.git
cd macmon
make check    # verify dependencies
./install.sh  # install and start daemon
```

The installer will:
1. Copy files to `~/.local/libexec/macmon/`
2. Compile Swift binaries (ProcessPicker, DiskIOHelper, MacmonStatusBar)
3. Create a default config at `~/.config/macmon/macmon.yaml`
4. Install a LaunchAgent (auto-starts on login)
5. Create a `macmon` symlink in `~/.local/bin/`

### One-line installer (latest release)

```bash
curl -fsSL https://raw.githubusercontent.com/chochy2001/macmon/main/install-web.sh | bash
```

This always installs the latest published GitHub release and verifies checksums before installation.

Make sure `~/.local/bin` is in your PATH:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

## Usage

### CLI Commands

```bash
macmon                  # Open the process picker UI
macmon status           # Show system health in terminal
macmon start            # Start the background daemon
macmon stop             # Stop the background daemon
macmon restart          # Restart the daemon
macmon config           # Show current configuration
macmon config edit      # Open config in $EDITOR
macmon config reset     # Reset to default configuration
macmon export           # Export current snapshot as JSON
macmon export csv       # Export as CSV
macmon export --peaks   # Show daily peak consumption
macmon log              # Show last 50 lines of daemon log
macmon log -f           # Follow daemon log in real time
macmon version          # Show version
macmon help             # Show all commands
```

### Process Picker Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+A | Select all non-system processes |
| Cmd+F | Focus search field |
| Cmd+G | Toggle process grouping |
| Cmd+I | View selected process details |
| Delete | Close selected processes |
| Enter | Close selected processes |
| Escape | Cancel and close |

Tip: double-click any process row to open full details (PID, RAM, CPU, uptime, state, and directory/URL).

### AI Quick Start (GUI)

1. Open `ProcessPicker -> Preferences...` (or `Actions -> AI Settings`).
2. Choose provider (OpenAI / Anthropic / OpenRouter / Gemini).
3. Paste model + API key, click `Save` (`Saved in Keychain` confirms success).
4. In the Process Picker, click `Smart Optimize` to request safe PID suggestions.
5. Review suggestions and confirm manually (human-in-the-loop).

Notes:
- Keys are stored in macOS Keychain.
- AI never executes shell commands directly.
- Suggestions still pass protected-process and signature safety checks.

### Menu Bar Monitor

<!-- SCREENSHOT: Menu Bar dropdown
     Capture the macOS menu bar with the macmon icon (memorychip + percentage)
     clicked open, showing the dropdown menu with:
     - RAM: XX% free of XXGB (colored green/yellow/red)
     - Swap: XXMB used
     - Processes: XX total
     - Separator
     - Open Process Picker
     - Export Snapshot... submenu
     - Status...
     - Quit macmon
     Make sure the menu bar area and dropdown are both visible.
     Save as: docs/images/menubar.png
-->
![Menu Bar](docs/images/menubar.png)

After installation, the menu bar icon shows live RAM usage. Click it to see:
- Current RAM and swap usage (color-coded)
- Total process count
- Quick actions: Open Picker, Export, Quit

## Data Freshness and Accuracy

- **Process Picker:** refreshes every 5 seconds while the window is open.
- **Menu Bar:** refreshes every 30 seconds.
- **Daemon checks:** run every `intervals.check` seconds (default `60`).

If values differ from Activity Monitor, that is expected in short windows: CPU and memory can differ by sampling instant, aggregation window, and process categorization. For fast-changing workloads, compare after a few refresh cycles instead of a single instant.

## Configuration

Edit `~/.config/macmon/macmon.yaml` (or run `macmon config edit`):

```yaml
thresholds:
  ram_free_percent: 25         # alert when free RAM % drops below this
  swap_used_mb: 2048           # alert when swap usage exceeds this (MB)
  process_ram_min_kb: 102400   # minimum RAM to show a process in picker (KB)
  idle_cpu_percent: 1.0        # CPU % below this marks a process as idle
  idle_ram_trigger_percent: 40 # only suggest idle cleanup when free RAM below this

custom_processes:
  - name: "flutter_tester"
    max_instances: 10
  - name: "gradlew"
    max_ram_mb: 2048
  - name: "SourceKitService"
    max_cpu_percent: 90

intervals:
  check: 60          # seconds between monitoring cycles
  idle_check: 600    # seconds between proactive idle process scans
  cooldown: 300      # minimum seconds between same-type alerts
  kill_grace: 3      # seconds after SIGTERM before SIGKILL

collect:
  disk_io: true              # collect per-process disk I/O
  batch_lsof_limit: 50       # max PIDs for batch lsof
  chrome_tab_titles: true    # resolve Chrome tab titles

log:
  max_size_mb: 10    # rotate log at this size
  max_files: 5       # keep this many rotated logs
  dir: ~/.local/log/macmon
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for all options.

## How It Works

macmon consists of four components:

<!-- SCREENSHOT: Native macOS notification
     Capture a macOS notification banner from macmon showing either:
     - Memory pressure alert: "RAM is low (XX% free). Open process picker to free memory?"
     - OR dynamic process alert: "Detected X process threshold violation(s). Kill offending processes?"
     - OR orphan daemon alert: "Found X orphan build daemon(s)"
     Use tools/simulate_load.sh to trigger the notification, then capture it.
     Save as: docs/images/notification.png
-->
![Notification](docs/images/notification.png)

1. **Daemon** (`macmond.sh`) — background loop that checks RAM, swap, and process thresholds every 60 seconds. Shows native macOS notifications when thresholds are crossed and offers to open the process picker.

2. **CLI** (`macmon.sh`) — user-facing entry point with subcommands for daemon control, system status, config management, and data export.

3. **Process Picker** (`ProcessPicker.swift`) — native AppKit window with a table view showing all processes above the RAM threshold. Supports search, grouping by app bundle, sorting by any column, and batch closing.

4. **Menu Bar** (`MacmonStatusBar.swift`) — persistent NSStatusItem that shows live RAM usage and provides quick access to the picker and exports.

The shared library (`macmon-core.sh`) centralizes all security-critical functions and eliminates code duplication between daemon and CLI.

Security details and ATT&CK mapping: [docs/SECURITY.md](docs/SECURITY.md)

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full design.

## Testing

```bash
brew install bats-core  # one-time setup

make test               # run BATS tests (bash logic)
make check              # full verification (syntax + compilation + tests)
```

Tests cover: process name resolution (17 tests), AppleScript injection sanitization (7 tests), system process protection (8 tests), YAML config loading (9 tests), uptime calculation (5 tests).

## Versioning

macmon uses [Semantic Versioning](https://semver.org/). The version is defined once in `lib/macmon-core.sh` and read by all components.

To create a new release:
```bash
# 1. Update MACMON_VERSION in lib/macmon-core.sh
# 2. Update CHANGELOG.md
# 3. Commit, tag, and push
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin main --tags
```

Pushing a tag triggers the CI to build and publish a GitHub Release.

See [docs/VERSIONING.md](docs/VERSIONING.md) for the full release process.

## FAQ

**Q: Why not just use Activity Monitor?**
Activity Monitor shows raw process data. macmon adds: idle detection, orphan daemon alerts, proactive memory pressure notifications, process grouping, disk I/O, working directories, and the ability to batch-close processes with safety checks. It's built for developers who need actionable insights, not just numbers.

**Q: Does macmon need root or sudo?**
No. macmon runs entirely as your user. The LaunchAgent is in `~/Library/LaunchAgents/` (user-level). It can only see and kill your own processes. Disk I/O collection uses `proc_pid_rusage` which works without elevated privileges.

**Q: Will macmon kill system processes?**
No. macmon maintains a protected process list (launchd, WindowServer, kernel_task, etc.) and verifies Apple code signatures before granting kill immunity. You cannot accidentally kill critical system processes through macmon.

**Q: How much CPU does the daemon use?**
Approximately 0.03%. The daemon wakes every 60 seconds, runs 3-5 lightweight checks (~10-30ms), then goes back to sleep. It runs at Nice priority 10 with `LowPriorityBackgroundIO`.

**Q: What happens when I close Chrome tabs through macmon?**
macmon uses AppleScript to close Chrome tabs gracefully (URL-first matching, with safe fallbacks) instead of killing renderer processes directly. This preserves your session data and avoids browser instability.

**Q: Where do I find logs for troubleshooting?**
- Daemon: `~/.local/log/macmon/macmond.log`
- Picker UI actions: `~/.local/log/macmon/process-picker.log`

**Q: How do I change the monitoring frequency?**
Edit `~/.config/macmon/macmon.yaml` and set `intervals.check` to your preferred value in seconds. Then either restart the daemon (`macmon restart`) or send SIGUSR1 to reload: `kill -USR1 $(cat $TMPDIR/macmond.pid)`.

**Q: Can I add my own protected processes?**
Yes. Add them to the `protected` list in your config file:
```yaml
protected:
  - launchd
  - kernel_task
  - my_important_daemon
```

**Q: How do I export data for analysis?**
```bash
macmon export json    # full snapshot with system health + all processes
macmon export csv     # spreadsheet-friendly format
macmon export --peaks # daily peak RAM/CPU per process
```

**Q: Can I use macmon in a CI/CD environment?**
`macmon status` and `macmon export` work without a GUI. The daemon and picker require a display session.

## Uninstall

```bash
./uninstall.sh   # interactive: asks about config and logs
# or
make uninstall
```

## Contributing

1. Fork the repo and create a feature branch
2. Make your changes
3. Run `make check` to verify everything passes
4. Run `make test` to run the test suite
5. Submit a pull request

All commits should be in English. Follow conventional commit messages: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.

## License

MIT License - Copyright (c) 2026 Jorge Salgado Miranda

See [LICENSE](LICENSE) for details.
