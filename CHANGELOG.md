# Changelog

## 1.0.0 (2026-03-03)

Initial open-source release.

### Features
- Background daemon with memory pressure, swap, and flutter_tester monitoring
- Native AppKit process picker with search, grouping, and system summary
- CLI with subcommands: status, start/stop/restart, config, log
- Configurable YAML-based settings
- LaunchAgent for auto-start on login
- Install/uninstall scripts

### Security
- AppleScript injection sanitization for all osascript calls
- JSON construction via jq (no string interpolation)
- Temp files in per-user private $TMPDIR (not /tmp)
- System process protection list
- PID identity verification before kill signals
- Exact match process lookup (pgrep -x)
- Chrome tab closing via AppleScript (no kill on renderers)

### Performance
- Batched process collection (3 ps calls instead of ~300)
- Cached memory_pressure with 30s TTL
- NSTableView cell recycling
- Single-row reload on checkbox toggle
- Shared library eliminates code duplication
