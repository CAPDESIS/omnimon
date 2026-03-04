# Cross-Platform Research for macmon v4.0.0

Date: 2026-03-04

## Objective

Define the best technical path to port macmon's core concept ("Monitor + AI Human-in-the-Loop") from macOS-only to macOS, Linux, and Windows with:

- Low resource usage (similar to current lightweight profile)
- Native process control and telemetry
- System tray/menu bar UX and pop-up windows
- Reliable autostart per OS
- Security-first handling for AI context and kill operations

## Current Architecture Constraints (v3.x)

- Process collection/kill logic is Bash-centric (`lib/macmon-core.sh`, `src/cli/macmon.sh`)
- GUI is AppKit/Swift-only (`ProcessPicker`, `MacmonStatusBar`)
- macOS startup is LaunchAgent/`launchd`
- Chrome tab handling uses AppleScript + renderer metadata

These are excellent for macOS but not directly portable to Linux/Windows.

## Evaluation Criteria

1. Runtime footprint (CPU/RAM) for long-running monitor + tray app
2. Native OS access for processes, memory, kill, startup integration
3. Developer velocity and maintainability
4. Packaging/distribution complexity across 3 desktop OSes
5. Security surface area and hardening options
6. Feasibility of Chrome tab context and AI privacy controls

## Candidate Stack Matrix

| Stack | Footprint | Native Syscalls | Tray/Menu UX | Packaging | Pros | Cons | Score (1-5) |
|---|---|---|---|---|---|---|---|
| Rust core + Tauri UI | Very low; near-native | Excellent via Rust crates and OS APIs | Strong (tray support on all desktop OSes) | Good (`dmg`, `msi`, `deb`/`AppImage`) | Best balance of perf/security/portability; small binaries | Requires Rust expertise; frontend+Rust boundary design | 4.8 |
| Go core + Wails UI | Low to medium | Good (Go stdlib + x/sys) | Good (native window + menu/tray support) | Good | Fast iteration for backend-heavy app; single binary experience | Slightly heavier runtime than Rust/Tauri; ecosystem less deep for some desktop edge cases | 4.2 |
| Flutter UI + C++/Rust native plugin core | Medium | Good (through plugin channels) | Good desktop support | Good but larger artifacts | Best-in-class UI polish and rapid UI iteration | Heavier binaries/runtime; plugin complexity for deep OS telemetry/kill paths | 3.9 |
| Electron + Node/Rust sidecar (reference only) | High | Good with native modules/sidecar | Very good | Excellent ecosystem | Fast prototyping and huge ecosystem | Resource-heavy vs target profile; larger attack/resource surface | 3.1 |

## OS Capability Feasibility

### Process + Memory + Kill

- macOS: `proc_pidinfo`, `rusage`, signals, Apple APIs where needed
- Linux: `/proc` (`/proc/[pid]/*`), cgroups metadata, signals
- Windows: Toolhelp/PSAPI/PDH + Win32 process APIs

All three are practical with a Rust core service abstraction layer.

### Autostart / Background Agent

- macOS: LaunchAgent (`~/Library/LaunchAgents`)
- Linux: systemd user unit (`~/.config/systemd/user`), optional fallback for non-systemd
- Windows: Registry Run key and/or Task Scheduler service model

### Tray/Menu Bar

- Tauri and Wails both support system tray paradigms across desktop platforms
- Per-OS behavior differences remain (icon templates, indicator libs on Linux)

### Chrome Tabs Context

- macOS: current AppleScript path remains viable
- Linux/Windows: recommended to prefer Chrome DevTools Protocol (CDP) local endpoint integration
- Fallback strategy: if tab introspection unavailable, keep process-only optimization suggestions

## Security and Privacy Notes (for v4)

- Preserve current privacy default: browser URL sharing OFF
- Send minimized AI payloads by default (title/domain only unless explicit opt-in)
- Keep immutable protected-process blocklist at core level
- Introduce signed action pipeline for kill requests (UI -> core daemon validation)

## Recommended Architecture (Verdict)

### Final recommendation: Rust Core + Tauri Desktop UI

Why this wins:

1. Lowest steady-state overhead among practical cross-platform GUI stacks
2. Best native control for system monitoring and process management
3. Strong security posture (memory safety + smaller runtime surface)
4. Good packaging story for all target desktop platforms
5. Good tray UX and event model without Electron-level resource costs

## Proposed v4.0.0 High-Level Design

- `macmon-core` (Rust library):
  - process collector, system metrics, kill policy engine, safety filters
- `macmon-agent` (Rust daemon/background service):
  - polling, history, policy execution, IPC endpoint
- `macmon-ui` (Tauri app):
  - tray, picker window, AI recommendations, settings/privacy toggles
- `macmon-cli` (thin frontend):
  - talks to agent via IPC for parity with GUI

## Component Translation Map (Current -> v4)

- Daemon Bash (`src/daemon/macmond.sh`) -> Rust `macmon-agent` service
- Shared Bash core (`lib/macmon-core.sh`) -> Rust `macmon-core` crate
- CLI Bash (`src/cli/macmon.sh`) -> Rust CLI client (or thin shell wrapper over Rust binary)
- AppKit picker (`ProcessPicker.swift`) -> Tauri window + Rust command bridge
- AppKit status bar (`MacmonStatusBar.swift`) -> Tauri tray menu/window controller
- AppleScript Chrome close (`scripts/graceful-quit.sh`) ->
  - macOS: keep AppleScript adapter module
  - Linux/Windows: CDP adapter module
- LaunchAgent install flow (`install.sh`) ->
  - macOS: LaunchAgent installer
  - Linux: systemd user unit installer
  - Windows: Run/Task Scheduler installer
- Localized strings (`Resources/*.lproj`) -> i18n catalog in Tauri frontend (and shared key schema)

## Migration Plan (Phased)

1. **Phase A - Core extraction**
   - Re-implement collector + safety kill pipeline in Rust
   - Validate parity against current macOS outputs
2. **Phase B - Agent + IPC**
   - Introduce background service and stable JSON IPC schema
3. **Phase C - UI port**
   - Tauri tray + picker parity with existing UX
4. **Phase D - Linux/Windows bring-up**
   - Autostart installers, packaging, smoke tests
5. **Phase E - AI layer parity**
   - Reapply privacy controls and suggestion sanitization across OSes

## Risks and Mitigations

- Risk: CDP permissions/availability differ by browser profile
  - Mitigation: capability detection + graceful fallback + explicit UX messaging
- Risk: Linux distro tray fragmentation
  - Mitigation: support both appindicator variants and document dependencies
- Risk: feature drift between CLI and GUI
  - Mitigation: force both to consume same agent IPC and policy engine

## Conclusion

For v4.0.0, the technically strongest path is **Rust Core + Tauri UI**. It best satisfies lightweight performance, native system control, and cross-platform packaging while preserving the safety and human-in-the-loop philosophy of macmon.
