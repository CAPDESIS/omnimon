# Changelog

## 1.2.0 (2026-03-03)

### Features
- Menu bar monitor (`MacmonStatusBar`) — NSStatusItem with live RAM/swap display, native data collection via `host_statistics64`, quick access to picker and export
- MVVM refactor — extracted `ProcessPickerModel.swift` (Foundation-only) from ProcessPicker for testable model layer
- Homebrew formula (`brew/macmon.rb`) for tap-based distribution
- Release workflow (`.github/workflows/release.yml`) — automatic GitHub Releases on version tags
- Semantic versioning with `docs/VERSIONING.md`

### Testing
- XCTest suite: 12 tests covering JSON parsing, filter/sort, grouping, collapse, selection, system process skip
- XCTest runner script (`tests/swift/run_tests.sh`) compiles and runs tests without Xcode

### Documentation
- Complete README rewrite with CI badge, feature comparison table, FAQ, versioning section
- Updated ARCHITECTURE.md with all 5 components
- Updated CONFIGURATION.md with disk I/O, orphan daemons, export, menu bar sections

## 1.1.0 (2026-03-03)

### Features
- Orphan build daemon detection: SourceKitService, GradleDaemon, xcodebuild, qemu-system (Android emulator)
- Per-process disk I/O metrics via `proc_pid_rusage` (DiskIOHelper Swift binary)
- `macmon export [json|csv]` command for snapshot export
- `macmon export --peaks` for daily peak consumption tracking
- Disk Read/Write columns in process picker UI
- Orphan daemon counts in `macmon status` output

### Security
- Code signature verification for system process names (prevents kill-immunity spoofing)
- Input validation for CPU values from ps (prevents awk injection)
- Numeric validation for all config-sourced intervals passed to `sleep`/arithmetic
- bash 3.2 compatibility for config loader (no `^^` operator)

### Testing & CI
- BATS test suite: 46 tests covering friendly_name, AppleScript escaping, system process protection, config loading, uptime calculation
- GitHub Actions CI pipeline: shell syntax checks, Swift compilation, BATS tests on macOS runner

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
