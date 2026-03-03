# Architecture

## Overview

macmon is structured as three cooperating components with a shared library:

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
       ┌───────────────┐              ┌───────────┴───────┐
       │ macmon-core.sh │              │  JSON temp file   │
       │ (shared lib)   │─────────────▶│  (process data)   │
       │                │              └───────────────────┘
       │ - Security     │
       │ - Collection   │
       │ - Kill logic   │
       └───────┬────────┘
               │
       ┌───────▼────────┐
       │macmon-config.sh │
       │ (YAML loader)   │
       └─────────────────┘
```

## Data Flow

1. **Collection**: `collect_processes_json()` runs 3 batched `ps` calls + 1 batched `lsof` call, enriches data, and outputs JSON via `jq`
2. **Transfer**: JSON written to a temp file in `$TMPDIR` (per-user private directory)
3. **Display**: Swift picker reads JSON via `Codable`, presents in `NSTableView` with cell recycling
4. **Selection**: User selects processes; picker outputs PIDs to stdout
5. **Killing**: `kill_processes()` verifies PID identity, checks system process protection, uses graceful quit for apps/Chrome tabs

## Security Model

### Input Sanitization
- All strings interpolated into AppleScript go through `_applescript_escape()` (strips control chars, escapes `\` and `"`)
- All JSON construction uses `jq` (no string concatenation)
- Temp files use `$TMPDIR` (macOS per-user private directory, not world-readable `/tmp`)

### Process Protection
- `is_system_process()` checks against a configurable protected list before any kill
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
| Table view rendering | New views per cell per reload | `makeView(withIdentifier:)` recycling |
| Checkbox toggle | Full table reload | Single row reload |

## File Layout

```
lib/
  macmon-core.sh       # Shared functions (security, collection, kill, picker)
  macmon-config.sh     # Flat YAML parser → MACMON_CFG_* env vars
src/
  daemon/macmond.sh    # Background loop with signal handlers
  cli/macmon.sh        # CLI subcommand dispatcher
  gui/ProcessPicker.swift  # Native AppKit picker
scripts/
  chrome-tabs.sh       # Chrome tab enumeration via AppleScript
  graceful-quit.sh     # Safe app/tab closing
config/
  macmon.default.yaml  # Default configuration reference
templates/
  com.macmon.daemon.plist.in  # LaunchAgent template
```

## Signal Handling

The daemon handles:
- `SIGTERM` / `SIGINT` — clean shutdown (remove PID file, clean temp files)
- `SIGUSR1` — reload configuration without restart

## Configuration System

`macmon-config.sh` parses flat YAML files into environment variables:
```
thresholds:
  ram_free_percent: 25
```
Becomes: `MACMON_CFG_THRESHOLDS_RAM_FREE_PERCENT=25`

Default config is loaded first, then user config overlays it. List values (like protected processes) use `:` as delimiter.
