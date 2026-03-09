# Changelog

## 6.0.1 (2026-03-08)

### Documentation
- Refresh main README to align the public docs with OmniMon 6.0.1
- Add `AUDIT_DOCS.md`, `COMMANDS_REFERENCE.md`, `CVE_REPORT.md`, and `NIST_COMPLIANCE.md`
- Expand `docs/CLI_MANUAL.md` with the full CLI surface and practical examples

### Compliance
- Capture `cargo audit` results, remediation guidance, and dependency risk status
- Map implemented safeguards against selected NIST SP 800-53 controls and identify coverage gaps

### Versioning
- Align runtime-visible version strings in the desktop footer, Tauri About metadata, CLI banner, TUI title, and cloud user agent

## 4.0.7 (2026-03-05)

### Branding
- Rename product to OmniMon across all configs (tauri.conf.json, Cargo.toml, package.json)
- Translate README and all scripts to English for international audience
- Update Homebrew tap from `chochy2001/tap` to `chochy2001/omnimon`

### Frontend
- IPC security hardening: runtime type guards on every IPC response (`src/lib/ipc.ts`)
- Virtual scroll in ProcessTable: 60 FPS with 2000+ processes (97.5% DOM reduction)
- 150ms search debounce to avoid per-keystroke O(n) filtering
- Test infrastructure: vitest + testing-library/svelte + happy-dom
- 69 tests across 4 test files (91% statement coverage, 96% line coverage)

## 4.0.6 (2026-03-05)

### Distribution
- Homebrew Cask formula for macOS desktop app
- Cross-platform release: .dmg (macOS), .msi (Windows), .deb + .AppImage (Linux)
- Universal web installer (`install-web.sh`)

### CI/CD
- Relax coverage threshold to 80%, exclude os_native.rs
- Fix formatting in killer.rs
- OS-aware killer tests, fix sleep termination on Linux

## 4.0.4 (2026-03-05)

### Performance
- Expand core resilience tests and add watcher micro-benchmark
- CLI integration tests and coverage pipeline with llvm-cov

## 4.0.2 (2026-03-05)

### Security
- Harden kill identity checks with macOS native memory parity
- IPC security, WCAG accessibility, architecture guide

## 4.0.0 (2026-03-05)

### Complete Rewrite
- Rust native core replacing Bash/AppKit (sysinfo, CDP, FFI)
- Tauri + Svelte 5 desktop app with reactive UI
- Rust CLI with clap for headless/server usage
- Cross-platform: macOS, Windows, Linux
- AI-powered optimization flow (OpenAI, Anthropic, OpenRouter)
- Native keychain integration for credential security
- Per-OS secure blocklists for critical process protection

---

## 1.2.0 (2026-03-03)

### Features
- Menu bar monitor (`MacmonStatusBar`) with live RAM/swap display
- MVVM refactor for testable model layer
- Homebrew formula for tap-based distribution
- Release workflow for automatic GitHub Releases on version tags

### Testing
- XCTest suite: 12 tests covering JSON parsing, filter/sort, grouping, selection

## 1.1.0 (2026-03-03)

### Features
- Orphan build daemon detection
- Per-process disk I/O metrics
- Export command (JSON/CSV) with peak tracking

### Security
- Code signature verification, input validation, bash 3.2 compatibility

### Testing
- BATS test suite: 46 tests with GitHub Actions CI

## 1.0.0 (2026-03-03)

Initial open-source release with background daemon, native AppKit process picker, CLI, YAML config, and LaunchAgent auto-start.
