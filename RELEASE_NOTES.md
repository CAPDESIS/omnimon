# OmniMon v6.7.0 Release Notes

Release date: 2026-04-17

## Zombie Killer

New background engine that flags processes consuming excessive CPU or RAM for a sustained window and offers them for user-confirmed termination. Stateless core scorer lives in `crates/core/src/zombie_killer.rs` (with clamped config for CPU threshold, minimum uptime, sustained duration, and a `never_kill` blocklist). The stateful Tauri engine keys candidates by `(pid, start_time)` so PID reuse never causes a wrong kill, wraps each tick in `catch_unwind`, and exposes five IPC commands (`get/set_zombie_killer_config`, `list_zombie_candidates`, `kill_zombie`, `kill_all_zombies`). The `ZombieKiller.svelte` modal opens with Cmd/Ctrl+Shift+Z and subscribes to a push event (`zombie-killer-update`) to stay in sync without polling.

## AI Privacy Mode

A new `Settings.ai_privacy_mode` (default `false`) enables stable pseudonymous redaction of everything sent to the LLM: process names, file paths, URLs, browser tab titles, and external hostnames/IPs all become 24-bit SipHash tokens so the model can still reason about identity across calls without seeing the raw strings. Private IPs (RFC 1918, loopback, link-local, `fc00::/7`, `fe80::/10`) collapse to a single `<lan>` token. The toggle, a live usage counter, and the daily limit input live in a new "AI Privacy & Budget" section inside `ProfileSettings`.

## Daily AI Budget

A new `Settings.ai_daily_limit` (default `200`, `0` = unlimited) adds a per-UTC-day cap that complements the existing burst token bucket: burst caps the rate, daily caps the spend. The shared bucket spans `ai_chat`, `analyze_processes`, `analyze_context`, and `validate_api_key`. A new IPC command `get_ai_daily_usage` returns `(used, limit)` for the UI.

## Frontend-Confirmed Automation Rule Changes

`add_automation_rule` and `remove_automation_rule` are no longer executed server-side inside `ai_chat`. They now return a plan (`details` + `payload`) and the `AIChat` UI stages the change as a `pendingAction`, invoking the real IPC only after explicit user confirmation. Same pattern already used by `kill_process`, `kill_by_name`, `close_tabs`, and `close_connection`.

## Keyring Hardening

`get_api_key_with_fallback` now wipes the legacy plaintext store *before* attempting the secure keyring write. A process interruption mid-flight can no longer leave the API key readable on disk.

## DPI Transparency Badge

New `role="status" aria-live="polite"` indicator in `StatusBar` (Radar icon from lucide) that appears whenever `$networkTelemetryStatus.dpiActive === true`. The tooltip explains that OmniMon inspects packet metadata only (not payload) and points to the setting that disables DPI.

## Local Ollama Support

`tauri.conf.json` CSP `connect-src` now permits `http://localhost:11434` and `http://127.0.0.1:11434`, so local Ollama works without loosening the rest of the policy.

## Breaking Change — `network_alerts::EvaluatorState`

The alert evaluator previously stored its debounce counters and cooldown map in a process-global `OnceLock<RwLock<EvaluatorState>>`. Parallel test threads were contaminating each other's `consecutive_matches` map, which produced intermittent flaky failures in `active_rules_drive_evaluate_active_network_alerts`.

`evaluate_network_alerts` and `evaluate_active_network_alerts` now take `state: &mut EvaluatorState` as a final argument. The `EvaluatorState` struct is `pub` with `::new()` and `::clear()`. In-workspace call sites have been updated; out-of-tree consumers must instantiate their own state or wrap both functions.

Migration:

```rust
// Before
let alerts = evaluate_network_alerts(&snap, prev, &rules, &history);

// After
let mut state = network_alerts::EvaluatorState::new();
let alerts = evaluate_network_alerts(&snap, prev, &rules, &history, &mut state);
// keep `state` alive for subsequent ticks to preserve debounce + cooldown.
```

## Quality

| Metric | Value |
|--------|-------|
| Rust tests | 458 (288 core + 95 integration + 53 tauri + 18 + 4 tui) |
| Frontend tests | 689 (Vitest) |
| Statement coverage | 86% |
| Branch coverage | 70.37–70.80% (stable range) |
| Function coverage | 86% |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean under Rust 1.95 |

New Rust lints (`unnecessary_sort_by` and `collapsible_match`) promoted to hard errors under Rust 1.95 were fixed across 10 pre-existing call sites.

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
