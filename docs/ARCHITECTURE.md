# Architecture

## Overview

macmon is structured as five cooperating components with a shared library:

```
┌────────────-─┐    ┌─────────────-┐    ┌──────────────────┐
│   macmond    │    │  macmon CLI  │    │  ProcessPicker   │
│  (daemon)    │    │   (bash)     │    │   (Swift/AppKit) │
│              │    │              │    │                  │
│  Monitors    │    │  Subcommands │    │  Native UI with  │
│  thresholds  │───▶│  & picker    │───▶│  search, groups, │
│  & alerts    │    │  launcher    │    │  summary bar     │
└──────┬───────┘    └──────┬───────┘    └──────────────────┘
       │                   │                      ▲
       └───────┬───────────┘                      │
               ▼                                  │
       ┌────────────-───┐              ┌──────────┴───────-┐
       │ macmon-core.sh │              │  JSON temp file   │
       │ (shared lib)   │─────────────▶│  (process data)   │
       │                │              └───────────────────┘
       │ - Security     │
       │ - Collection   │
       │ - Kill logic   │
       └───────┬────────┘
               │
       ┌───────▼──────-──┐
       │macmon-config.sh │
       │ (YAML loader)   │
       └─────────────────┘

┌────────────────┐    ┌──────────────────┐
│ MacmonStatusBar│    │  DiskIOHelper    │
│ (Swift/AppKit) │    │  (Swift/CLI)     │
│                │    │                  │
│ Menu bar with  │    │ proc_pid_rusage  │
│ live RAM/swap  │    │ per-process I/O  │
│ & quick actions│    │ → JSON output    │
└────────────────┘    └──────────────────┘
```

## Components

### 1. Daemon (`macmond.sh`)
Background loop that checks RAM, swap, dynamic `custom_processes` thresholds, orphan build daemons, and idle processes every 60 seconds. Shows native macOS notifications when thresholds are crossed. Supports signal-based config reload (SIGUSR1) and clean shutdown (SIGTERM/SIGINT).

### 2. CLI (`macmon.sh`)
User-facing entry point with subcommands: `status`, `start/stop/restart`, `config`, `export`, `log`, `version`, `help`. Launches the process picker as the default action.

### 3. Process Picker (`ProcessPicker.swift` + `ProcessPickerModel.swift`)
Native AppKit window using MVVM architecture:
- **Model** (`ProcessPickerModel.swift`) — Foundation-only, contains `ProcessEntry`, `SystemHealth`, `ProcessData`, `ProcessViewModel` with filter/sort/group pipeline. Fully testable with XCTest.
- **View** (`ProcessPicker.swift`) — AppKit layer with `NSTableView`, `NSSearchField`, `MemoryPressureGauge`, keyboard shortcuts, and cell recycling.

### 4. Menu Bar (`MacmonStatusBar.swift`)
Persistent `NSStatusItem` showing live RAM usage. Collects data natively via `host_statistics64` and `sysctlbyname` (no subprocess spawning). Metrics collection runs on a background queue and applies UI updates on the main thread. Provides quick access to picker, config editing, export, and status.

### 5. AI Analysis Layer (`AIService.swift`)
Keychain backed API key storage with provider support for OpenAI, Anthropic, and OpenRouter. This layer is analysis only and returns PID suggestions as strict JSON for user review.

### 5. Disk I/O Helper (`DiskIOHelper.swift`)
Standalone binary that reads per-process disk I/O via `proc_pid_rusage` with `RUSAGE_INFO_V4`. Takes PIDs as arguments or via `--stdin`, outputs JSON. Does not require root.

## Data Flow

1. **Collection**: `collect_processes_json()` runs 3 batched `ps` calls + 1 batched `lsof` call, enriches data with disk I/O from DiskIOHelper, and outputs JSON via `jq`
2. **Transfer**: JSON written to a temp file in `$TMPDIR` (per-user private directory)
3. **Display**: Swift picker reads JSON via `Codable`, presents in `NSTableView` with cell recycling
4. **Selection**: User selects processes; picker outputs PIDs to stdout
5. **Killing**: `kill_processes()` verifies PID identity, checks system process protection and code signatures, uses graceful quit for apps/Chrome tabs

## Security Model

### Input Sanitization
- All strings interpolated into AppleScript go through `_applescript_escape()` (strips control chars, escapes `\` and `"`)
- All JSON construction uses `jq` (no string concatenation)
- Temp files use `$TMPDIR` (macOS per-user private directory, not world-readable `/tmp`)
- CPU and interval values validated with regex before use in arithmetic

### Process Protection
- `is_system_process()` checks against a configurable protected list before any kill
- `_verify_apple_signed()` verifies Apple code signatures on protected process names to prevent spoofing
- `verify_pid()` confirms process identity matches before sending signals (prevents PID reuse race condition)
- `pgrep -x` / `pkill -x` for exact match (no substring matching)

### Chrome Safety
- Chrome tabs are closed via AppleScript `close tab` rather than `kill` on renderer processes
- Prevents browser crash and data loss

## Performance Design

| Operation | Before | After |
|-----------|--------|-------|
| Process collection | ~300 subprocess spawns | 3 `ps` + 1 `lsof` call |
| Memory pressure check | Called 2x per cycle | Cached with 30s TTL |
| Disk I/O collection | N/A | Single DiskIOHelper invocation for all PIDs |
| Menu bar refresh | N/A | Native `host_statistics64`, no subprocesses |
| Table view rendering | New views per cell per reload | `makeView(withIdentifier:)` recycling |
| Checkbox toggle | Full table reload | Single row reload |

## File Layout

```
lib/
  macmon-core.sh                 # Shared functions (security, collection, kill, picker)
  macmon-config.sh               # Flat YAML parser → MACMON_CFG_* env vars
src/
  daemon/macmond.sh              # Background loop with signal handlers
  cli/macmon.sh                  # CLI subcommand dispatcher
  gui/ProcessPickerModel.swift   # Model layer (Foundation only, testable)
  gui/ProcessPicker.swift        # AppKit UI layer
  gui/MacmonStatusBar.swift      # Menu bar status item
  gui/DiskIOHelper.swift         # Disk I/O via proc_pid_rusage
scripts/
  chrome-tabs.sh                 # Chrome tab enumeration via AppleScript
  graceful-quit.sh               # Safe app/tab closing
config/
  macmon.default.yaml            # Default configuration reference
templates/
  com.macmon.daemon.plist.in     # LaunchAgent template
tests/
  *.bats                         # BATS shell script tests (46 tests)
  swift/ProcessViewModelTests.swift  # XCTest suite (12 tests)
  swift/run_tests.sh             # XCTest runner script
brew/
  macmon.rb                      # Homebrew formula
```

## Signal Handling

The daemon handles:
- `SIGTERM` / `SIGINT` — clean shutdown (remove PID file, clean temp files)
- `SIGUSR1` — reload configuration without restart

The daemon also watches config file mtime and triggers the same reload path automatically when `macmon.yaml` changes.

Profiles are resolved from `~/.config/macmon/profiles/` through `active_profile`, and profile changes also trigger daemon reload.

## Configuration System

`macmon-config.sh` parses flat YAML files into environment variables:
```
thresholds:
  ram_free_percent: 25
```
Becomes: `MACMON_CFG_THRESHOLDS_RAM_FREE_PERCENT=25`

Default config is loaded first, then user config overlays it. List values (like protected processes) use `:` as delimiter. The `~` prefix in paths is automatically expanded to `$HOME`. Dynamic `custom_processes` are parsed from YAML list entries into `name:max_instances:max_ram_mb:max_cpu_percent` records with cache invalidation on config reload.
