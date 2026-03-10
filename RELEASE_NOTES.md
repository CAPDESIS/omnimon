# OmniMon v6.3.0 Release Notes

Release date: 2026-03-10

## User Profiles

New profile presets system with three modes (minimal/balanced/power) that control dashboard section visibility, refresh intervals, and notification levels. Each user can pin favorite processes to the top of the process table and customize their dashboard layout.

## E2E Testing with Playwright

Migrated from WebdriverIO/Tauri to standalone Playwright. Five E2E suites cover: app loading, process table interaction, navigation, settings, and AI chat. Reusable fixtures mock Tauri IPC for metrics, tabs, network, and AI.

## Post-Sprint Audit

- 29 dead imports removed across 12 components
- NetworkMap onclick handler bug fixed (out-of-scope event)
- 3 implicit `any` types resolved
- Rate limiting added to 5 previously unprotected IPC commands
- CSP hardened: `object-src 'none'; base-uri 'self'`
- Virtual scroll buffer increased from 0 to 3 rows

## Quality

| Metric | Value |
|--------|-------|
| Total tests | 1083 (663 frontend + 413 Rust + 7 E2E) |
| Statement coverage | 86.5% |
| Branch coverage | 72% |
| Function coverage | 87.7% |

## Documentation

- README.md rewritten for v6.3.0 with architecture diagram and badges
- `docs/ARCHITECTURE.md`: 7 Mermaid diagrams, 13 modules documented
- CONTRIBUTING.md updated with full workflow
- COMMANDS_REFERENCE.md: +15 CLI commands, +3 IPC commands
- CLI_MANUAL.md: 4 new sections (config, network, rules, release)

## Install

```bash
# macOS
brew tap chochy2001/omnimon && brew install --cask omnimon

# Linux
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash

# Windows — download .msi from GitHub Releases
```

## Artifacts

All platform artifacts (`.dmg`, `.msi`, `.deb`, `.AppImage`, `.rpm`) are signed with Ed25519 and include SHA-256 checksums. Verify with `omnimon release verify`.

See [CHANGELOG.md](CHANGELOG.md) for the full list of changes.
