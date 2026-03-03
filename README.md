# macmon

A lightweight macOS system monitor that watches RAM pressure, swap usage, flutter_tester accumulation, and idle processes. Includes a native AppKit process picker UI, a background daemon, and a CLI — all built without Xcode.

## Features

- **Background daemon** — monitors memory pressure, swap, and process accumulation with configurable thresholds and cooldowns
- **Native process picker** — AppKit-based UI with search, grouping, sorting, memory pressure gauge, and batch process closing
- **CLI** — `macmon status`, `macmon start/stop/restart`, `macmon config`, `macmon log`
- **Security-first** — AppleScript injection sanitization, jq-based JSON construction, PID reuse verification, system process protection
- **Performance-optimized** — batched `ps` calls (3 instead of ~300), cached `memory_pressure`, NSTableView cell recycling
- **Chrome-aware** — closes Chrome tabs via AppleScript instead of killing renderer processes

### What macmon shows that Activity Monitor doesn't

| Feature | macmon | Activity Monitor |
|---------|--------|-----------------|
| Idle process detection | Yes (configurable CPU threshold) | No |
| Process grouping by app bundle | Yes | Partial |
| Working directory per process | Yes | No |
| Chrome tab titles | Yes | No |
| Flutter tester accumulation alert | Yes | No |
| Proactive memory pressure notifications | Yes | No |
| CLI system health summary | Yes | No |

## Requirements

- macOS 13+ (Ventura or later)
- `jq` — `brew install jq`
- Xcode Command Line Tools — `xcode-select --install`

## Installation

```bash
git clone https://github.com/chochy2001/macmon.git
cd macmon
make check    # verify dependencies
./install.sh  # install and start daemon
```

The installer will:
1. Copy files to `~/.local/libexec/macmon/`
2. Compile the Swift process picker
3. Create a default config at `~/.config/macmon/macmon.yaml`
4. Install a LaunchAgent (auto-starts on login)
5. Create a `macmon` symlink in `~/.local/bin/`

Make sure `~/.local/bin` is in your PATH:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

## Usage

```bash
macmon                  # Open the process picker UI
macmon status           # Show system health in terminal
macmon start            # Start the background daemon
macmon stop             # Stop the background daemon
macmon restart          # Restart the daemon
macmon config           # Show current configuration
macmon config edit      # Open config in $EDITOR
macmon config reset     # Reset to default configuration
macmon log              # Show last 50 lines of daemon log
macmon log -f           # Follow daemon log in real time
macmon version          # Show version
macmon help             # Show all commands
```

### Process Picker Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+A | Select all processes |
| Cmd+F | Focus search field |
| Cmd+G | Toggle process grouping |
| Delete | Close selected processes |
| Enter | Close selected processes |
| Escape | Cancel |

## Configuration

Edit `~/.config/macmon/macmon.yaml` (or run `macmon config edit`):

```yaml
thresholds:
  flutter_process_count: 10    # alert when flutter_tester exceeds this
  ram_free_percent: 25         # alert when free RAM drops below this
  swap_used_mb: 2048           # alert when swap exceeds this
  process_ram_min_kb: 102400   # minimum RAM (KB) to show in picker
  idle_cpu_percent: 1.0        # CPU below this = idle

intervals:
  check: 60                    # seconds between monitoring cycles
  idle_check: 600              # seconds between proactive idle scans
  cooldown: 300                # seconds between same-type alerts
```

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for all options.

## Architecture

macmon consists of three components:

1. **Daemon** (`macmond.sh`) — background loop that checks thresholds and shows alerts
2. **CLI** (`macmon.sh`) — user-facing entry point with subcommands
3. **Process Picker** (`ProcessPicker.swift`) — native AppKit window for selecting processes to close

The shared library (`macmon-core.sh`) eliminates code duplication and centralizes security-critical functions.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full design.

## Uninstall

```bash
./uninstall.sh
```

Or: `make uninstall`

## License

MIT License - Copyright (c) 2026 Jorge Salgado Miranda

See [LICENSE](LICENSE) for details.
