# OmniMon Documentation Audit

> Estado actual 2026-06-29: este reporte es una captura histórica del
> 2026-03-08. La versión actual de la app, crates y Tauri metadata es 6.7.0, y
> la guía operativa actualizada vive en `omnimon_apps/AGENTS.md`,
> `omnimon_apps/MASTER_SPEC.md`, y `omnimon_apps/macmon/AGENTS.md`.

Date: 2026-03-08
Scope: `worktree-gpt-2` / branch `audit/gpt-docs`

## Current State

- Main product version is `6.0.1` in `v4/crates/core/Cargo.toml`, `v4/crates/cli/Cargo.toml`, `v4/crates/tui/Cargo.toml`, `v4/apps/desktop/src-tauri/Cargo.toml`, `v4/apps/desktop/package.json`, and `v4/apps/desktop/src-tauri/tauri.conf.json`.
- Public documentation was partially stale: `README.md` still referenced OmniMon v5 and `docs/CLI_MANUAL.md` still referenced v5.2.0.
- Runtime-visible version strings were inconsistent: `v4/apps/desktop/src/App.svelte` showed `v6.0.3`, while the packaged app and crates declared `6.0.1`.
- CLI and network client version markers were also stale: `v4/crates/cli/src/main.rs` and `v4/crates/core/src/cloud.rs` still embedded `5.2.0`.

## Changes Applied

- Updated `README.md` to reflect OmniMon `6.0.1` and linked the new compliance/reference documents.
- Expanded `docs/CLI_MANUAL.md` to cover the full CLI command surface, including `auth`, `cloud`, `security-scan`, `doctor`, and `tui`.
- Aligned visible/runtime version markers in:
  - `v4/crates/cli/src/main.rs`
  - `v4/crates/core/src/cloud.rs`
  - `v4/crates/tui/src/ui.rs`
  - `v4/apps/desktop/src-tauri/src/lib.rs`
  - `v4/apps/desktop/src/App.svelte`
- Updated `CHANGELOG.md` with a `6.0.1` documentation/compliance entry.

## Documentation Gaps Found

- The repo still contains older historical documents that reference `v4`, `v5`, or `v5.2.0`; they appear archival and were not normalized in this pass.
- IPC commands were implemented but not documented in a single source of truth before this audit.
- AI chat user actions existed in code, but their user-facing behavior and expected readable output were not documented centrally.
- Versioning policy was implicit rather than written down.

## Versioning Assessment

- Effective release version: `6.0.1`.
- Recommended convention: SemVer `MAJOR.MINOR.PATCH` with one canonical source per artifact type:
  - Rust crates: `Cargo.toml`
  - Desktop frontend/package: `package.json`
  - Tauri bundle metadata: `tauri.conf.json`
- Recommended follow-up: add a small generated/shared version constant for the Svelte UI so visible labels do not drift from package metadata.

## Recommended Next Steps

- Add a CI assertion that compares the version in Cargo, Tauri, and `package.json`.
- Consolidate visible UI version labels behind a single exported constant.
- Review and archive or relabel legacy docs that still mention `v4`/`v5` if they remain relevant.
