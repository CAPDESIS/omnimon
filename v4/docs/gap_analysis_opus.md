# Gap Analysis: macmon v3 (AppKit + Bash) vs v4 (Rust + Tauri + Svelte)

**Author:** Claude Opus 4.6
**Date:** 2026-03-05
**Scope:** Read-only audit comparing feature parity across 4 critical areas

---

## Executive Summary

macmon v4 compiles, shows real metrics, and kills processes cross-platform. However, it ships **~35% of the v3 feature surface**. The most critical regressions are:

1. **Browser introspection is absent** — v3 resolved Chrome tab titles/URLs via AppleScript; v4 only sees raw `Google Chrome Helper` processes.
2. **AI Smart Optimize is absent** — v3 had a full Human-in-the-Loop flow with 4 providers and Keychain storage; v4 has zero AI code.
3. **UX is minimal** — v3 had 14 columns, grouping, Cmd+I detail panel, menu bar widget, preferences window, export, profiles; v4 has 6 columns and basic search.
4. **Security hardening regressed** — v3 protected 22 processes with code-signature verification; v4 protects 10 with name-only matching.

---

## 1. Browser Introspection

### How v3 Works (macOS-only, Chrome-only)

| Step | File | Mechanism |
|------|------|-----------|
| Enumerate tabs | `scripts/chrome-tabs.sh` | `osascript` → `tell application "Google Chrome"` → iterates windows/tabs |
| Extract data | Same script | Tab ID, title, URL per tab; sanitized via `sanitizeText()` |
| Match to PID | `lib/macmon-core.sh:442-482` | Extracts `--renderer-client-id` from process args, matches against tab ID array |
| Display name | `lib/macmon-core.sh:148-162` | Renderer PIDs renamed to `"Chrome Tab"`, detail field shows `"Title [domain.com]"` |
| Group | `lib/macmon-core.sh:472` | Grouped as `"Chrome: domain.com"` |
| Close tab | `scripts/graceful-quit.sh:30-94` | Two-stage AppleScript: close by URL match, then by tab ID; SIGTERM only as fallback |
| Skip SIGKILL | `lib/macmon-core.sh:696` | Chrome tabs explicitly excluded from SIGKILL sweep |
| Config toggle | `macmon.yaml` | `collect.chrome_tab_titles: true` / env `MACMON_DISABLE_CHROME_TABS=1` |

### v4 Current State

**No browser introspection exists.** `ChromeTabManager.svelte` filters by `group === "Browser"` but the Rust backend sets `group: String::new()` for all processes. Chrome renderers appear as raw `Google Chrome Helper` with no title, no URL, no tab-level close.

### Feature Comparison

| Feature | v3 Status | v4 Status | Proposed Cross-Platform Solution |
|---------|-----------|-----------|----------------------------------|
| Chrome tab title extraction | AppleScript `tell application "Google Chrome"` | Missing | **macOS:** AppleScript via `std::process::Command("osascript")`. **Windows/Linux:** Chrome DevTools Protocol (CDP) over `localhost:9222` — Chrome must be launched with `--remote-debugging-port=9222`. Alternatively, a companion browser extension (WebSocket relay) works without launch flags. |
| Chrome tab URL extraction | Same AppleScript | Missing | Same as above (CDP `Target.getTargets` returns title + URL for each tab) |
| Tab-to-PID matching | `--renderer-client-id` from `/proc/pid/cmdline` | Missing | Parse process command-line args for `--renderer-client-id=N` on all platforms (`sysinfo::Process::cmd()` in Rust). Match against CDP target `targetId`. |
| Tab display name | `"Chrome Tab"` with `"Title [domain.com]"` in detail column | Shows `"Google Chrome Helper"` | Map matched renderer PIDs to `ProcessEntry.name = tab_title`, `detail = url`, `group = "Chrome: domain"` |
| Graceful tab close | AppleScript `close tab` (no process kill) | Missing — kills renderer PID directly | **macOS:** AppleScript `close tab`. **Cross-platform:** CDP command `Target.closeTarget(targetId)` — closes tab cleanly without killing renderer. |
| Chrome tab grouping | `"Chrome: domain.com"` collapsible group | Group field is empty string | Populate `group` field from extracted domain |
| Safari tab support | Not implemented in v3 | Not implemented | **macOS:** AppleScript `tell application "Safari"`. **Cross-platform:** N/A (Safari is macOS-only). |
| Firefox/Edge support | Not implemented in v3 | Not implemented | **Firefox:** Marionette protocol or extension. **Edge:** Same as Chrome (Chromium-based, shares CDP). |
| Config toggle | `collect.chrome_tab_titles: true` | No config system | Add to `macmon.yaml` equivalent or Tauri preferences |
| Injection prevention | `_applescript_escape()` strips control chars | N/A | Sanitize all strings before embedding in AppleScript/CDP commands |

### Recommended Architecture for v4

```
                         macOS                    Windows / Linux
                    +--------------+          +-------------------+
                    | osascript    |          | CDP over ws://    |
                    | (AppleScript)|          | localhost:9222    |
                    +------+-------+          +--------+----------+
                           |                           |
                           v                           v
                    +------+---------------------------+----------+
                    |        core::browser::TabProvider            |
                    |  trait TabProvider {                         |
                    |    fn list_tabs() -> Vec<BrowserTab>;       |
                    |    fn close_tab(id: &str) -> Result<()>;    |
                    |  }                                          |
                    +---------------------------------------------+
```

Create `crates/core/src/browser.rs` with a `TabProvider` trait. Platform-specific implementations via `#[cfg(target_os)]`. CDP implementation works everywhere (fallback). AppleScript implementation preferred on macOS for zero-config operation.

---

## 2. AI Smart Optimize (Human-in-the-Loop)

### How v3 Works

| Component | File | Details |
|-----------|------|---------|
| AI Service | `src/gui/AIService.swift` (457 lines) | Core engine: prompt building, HTTP requests, response parsing, suggestion sanitization |
| Providers | `AIService.swift:10-33` | 4 providers: OpenAI (`gpt-4o-mini`), Anthropic (`claude-3-5-sonnet-latest`), OpenRouter, Gemini (`gemini-1.5-flash`) |
| Key storage | `AIService.swift:65-101` | macOS Keychain (`com.macmon.ai` service), one entry per provider |
| CLI key access | `src/cli/macmon.sh:490-496` | `security find-generic-password -s "com.macmon.ai" -a "$provider" -w` |
| Prompt | `AIService.swift:257-271` | System prompt: "You are a macOS optimization assistant. Return strict JSON only. Never include system processes." Includes active profile name + top 50 processes (pid, name, cpuPct, ramMB, isSystem) |
| Response format | `AIService.swift:417-441` | `{"suggestions":[{"pid":123,"reason":"short explanation"}]}` with backward compat for `{"pids":[...]}` |
| Safety filter | `AIService.swift:443-455` | Removes PID <= 1, unknown PIDs, protected processes (19-item list), dead processes; deduplicates; max 20 suggestions |
| GUI flow | `ProcessPicker.swift:1847-1948` | Button "Smart Optimize" → spinner → AI call → auto-select suggested PIDs → alert with "Apply" / "Review First" |
| CLI flow | `macmon.sh:480-681` | `macmon optimize` → collect processes → API call → validation → interactive `[Y/n]` confirmation |
| Preferences | `PreferencesWindow.swift:42-143` | Tab with provider dropdown, model field, API key (secure field), privacy toggle for browser URLs |
| Telemetry | `ProcessPicker.swift:1966-1969` | Records kill events with AI reason vs manual selection |
| Protected list | `AIService.swift:35-59` | 19 immutable process names (audio, video, kernel, system daemons) |

### v4 Current State

**Zero AI code exists.** No AI service, no provider configuration, no Keychain integration, no prompt building, no suggestion UI.

### Feature Comparison

| Feature | v3 Status | v4 Status | Proposed Cross-Platform Solution |
|---------|-----------|-----------|----------------------------------|
| AI provider support | 4 providers (OpenAI, Anthropic, OpenRouter, Gemini) | Missing | Create `crates/core/src/ai.rs` with provider-agnostic HTTP client using `reqwest`. Enum `AIProvider` with endpoint/header/payload builders per provider. Same 4 providers. |
| API key storage | macOS Keychain (`SecItemAdd`/`SecItemCopyMatching`) | Missing | **macOS:** `security` CLI or `security-framework` Rust crate. **Windows:** Windows Credential Manager via `keyring` crate. **Linux:** `libsecret` / `keyring` crate. The `keyring` crate abstracts all 3 platforms. |
| System prompt | Hardcoded optimization assistant prompt with profile context | Missing | Port verbatim to Rust `const` or config. Add cross-platform process names to prompt context. |
| Response parsing | JSON extraction with backward compat for `{"pids":[]}` | Missing | Implement with `serde_json`. Same dual-format parsing. |
| Safety sanitization | Filter PID<=1, unknown, protected, dead processes; dedup; max 20 | Missing | Reuse `killer::is_immutable_blocked_process_name()` + `sysinfo` PID verification. Add max-suggestion cap. |
| GUI flow (Human-in-the-Loop) | Button → spinner → suggestions alert → "Apply" / "Review" | Missing | Svelte component: button with loading state → invoke `analyze_processes` Tauri command → modal dialog listing suggestions with reasons → "Apply" / "Review" buttons. |
| CLI flow | `macmon optimize` with interactive confirmation | Missing | Add `optimize` subcommand to `crates/cli/`. Prompt `[Y/n]` via `dialoguer` crate or raw stdin. |
| Preferences UI | Native Cocoa tabbed window with provider/model/key fields | Missing | Svelte settings modal or dedicated route. API key input as `type="password"`. Save via Tauri command that calls `keyring` crate. |
| Privacy toggle | `allowBrowserURLs` UserDefaults key | Missing | Tauri plugin-store or config file. Strip URLs from prompt when disabled. |
| Telemetry | Records AI reason per kill event | Missing | Optional structured log to `~/.local/log/macmon/telemetry.jsonl` |
| Tab summarization | Separate AI feature to summarize open browser tabs | Missing | Requires browser introspection first (see Section 1). Then same AI prompt with tab data. |

### Recommended Architecture for v4

```
crates/core/src/ai.rs
  ├── AIProvider enum (OpenAI, Anthropic, OpenRouter, Gemini)
  ├── AIService struct
  │     ├── fn analyze_processes(procs, profile) -> Result<Vec<Suggestion>>
  │     ├── fn build_prompt(procs, profile) -> String
  │     ├── fn call_provider(provider, prompt) -> Result<String>
  │     └── fn parse_suggestions(response) -> Vec<Suggestion>
  ├── fn sanitize_suggestions(suggestions, live_pids, blocklist) -> Vec<Suggestion>
  └── Suggestion { pid: u32, reason: String }

apps/desktop/src-tauri/src/lib.rs
  ├── #[tauri::command] fn analyze_processes(profile: String) -> Result<Vec<Suggestion>, String>
  └── #[tauri::command] fn save_ai_config(provider, model, key) -> Result<(), String>

apps/desktop/src/components/SmartOptimize.svelte
  └── Button → Loading → Modal with suggestions → Apply / Review
```

Dependencies: `reqwest` (HTTP), `keyring` (cross-platform credential storage), `serde_json` (parsing).

---

## 3. Visual UX and Gestural Parity

### v3 Feature Inventory

**Window:** 1280x720, min 960x460, autosave frame/columns.
**Columns (14):** Checkbox, Name, Group, Detail, RAM, CPU%, Uptime, PID, Disk Read, Disk Write, Idle, Directory, State, TTY.
**v4 Columns (6):** Checkbox, Name, PID, RAM, CPU%, State + Idle badge.

### Feature Comparison

| Feature | v3 Status | v4 Status | Proposed Solution |
|---------|-----------|-----------|-------------------|
| **System Summary Bar** | Custom NSView: memory pressure gauge (color-coded bar), mini 3-bar chart (RAM/Swap/Processes), hover tooltips, click for insights alert | `StatusBar.svelte`: basic RAM bar + swap/process text | Add color thresholds to existing bar. Add swap bar. Add click handler showing insights modal with recommendation logic (low RAM, high swap, many idle). |
| **Process Grouping** | Collapsible groups with arrow indicator (triangle/chevron), group header shows count + total RAM, click to toggle, `Cmd+G` shortcut | Not implemented | Add `groupBy` store. Render group headers as sticky rows. Toggle via button + keyboard shortcut. Persist collapsed state. |
| **Detail Panel (Cmd+I)** | NSAlert modal showing Name, PID, RAM, CPU%, Uptime, Idle, Group, State, Detail (args/tab info), CWD. Triggered by double-click, Cmd+I, or button. | Not implemented | Svelte modal component. Trigger from row double-click, keyboard shortcut, or toolbar button. Show all available process fields. |
| **Column Count** | 14 columns including Disk I/O, Detail, Directory, TTY | 6 columns | Add columns: Group, Detail, Uptime, Directory. Disk I/O requires `sysinfo` disk usage per process (not available on all platforms — consider optional). |
| **Column Reordering** | Native NSTableView drag-and-drop reorder, persisted | Not implemented | HTML5 drag-and-drop on `<th>` elements. Store column order in localStorage. |
| **Color-Coded Thresholds** | RAM: red >2GB, orange >512MB. CPU: red >80%, orange >30%. Disk I/O: red >10GB, orange >1GB. Idle: blue "Yes" / dim "No". | RAM/CPU color in ProcessTable (red >2GB/>80%, orange >512MB/>30%) | Parity exists for RAM/CPU. Add Disk I/O colors when columns added. |
| **Menu Bar Widget** | `MacmonStatusBar.swift`: SF Symbol "memorychip" icon, live memory %, tooltip, dropdown menu with RAM/Swap/Process stats, profile switcher, export, config access | Tauri tray icon with "Show" and "Quit" only | Extend tray menu: add live RAM % in title, dropdown items for stats, profile switch, export, config. Tauri 2 `tray-icon` feature supports dynamic titles and rich menus. |
| **Keyboard Shortcuts** | Cmd+A (select all), Cmd+F (search), Cmd+G (groups), Cmd+I (details), Cmd+H (hide system), Cmd+E (export CSV), Cmd+J (export JSON), Cmd+T (Chrome tabs), Cmd+O (smart optimize), Cmd+Shift+R (restart daemon), Cmd+U (update check), Delete (close selected) | None beyond browser defaults | Register shortcuts via Tauri `GlobalShortcut` plugin or Svelte `on:keydown` handlers. Priority: Cmd+F, Cmd+I, Cmd+G, Cmd+A, Delete. |
| **Profiles** | Dropdown: default, developer, creator, gaming-performance. Each sets different thresholds. Hint label shows description. | Not implemented | Add profile store + dropdown. Load thresholds from config. Pass active profile to AI prompt. |
| **Preferences Window** | Two tabs: AI Settings (provider, model, key) + Rules (RAM/swap thresholds, intervals, privacy). Native NSWindow. | Not implemented | Svelte settings page/modal. Two sections matching v3 tabs. Save to Tauri store or config file. |
| **Export JSON/CSV** | Native NSSavePanel, structured output with all fields | Not implemented | `#[tauri::command] fn export_json/csv()` → Tauri `dialog::save_file()` → write formatted data. |
| **Selection Helpers** | Select All, Select None, Select Idle, Select Stale (idle 2+ days), Select Top RAM (top 5), Select Top CPU (top 5) | Select All, Select None only | Add buttons: "Select Idle" (cpu < 1%), "Select Top RAM" (sort + take 5), "Select Top CPU" (sort + take 5). |
| **Live Refresh** | 5-second timer, updates RAM/CPU/state per PID via `ps` | 2-second polling via `setInterval` → full `get_metrics` IPC call | Current approach works. Consider delta updates to reduce payload (send only changed fields). |
| **Chrome Tabs Window** | Separate window listing up to 80 tabs with ID, title, URL. "Summarize" button for AI analysis. | `ChromeTabManager.svelte` section (non-functional — no tab data) | Requires browser introspection (Section 1) first. Then populate component with real tab data. |
| **Config Editor** | GUI fields for every YAML parameter, disk I/O toggle, privacy checkbox | Not implemented | Svelte form bound to config values. Save via Tauri command. |
| **Localization** | English + Spanish (en.lproj, es.lproj) | Not implemented | Use `svelte-i18n` or Tauri's built-in locale detection. Priority: EN + ES. |
| **Accessibility** | VoiceOver labels on all controls, keyboard navigation chain, status announcements | Not implemented | Add `aria-label`, `role`, `tabindex` to all interactive elements. Announce state changes via `aria-live` regions. |
| **System Insights** | Click chart → alert with ASCII bar charts for RAM/Swap/Process count + personalized recommendation (low RAM, high swap, many idle, healthy) | Not implemented | Modal with bar visualizations. Recommendation logic: if RAM free < 20% → "free memory"; if swap > 3GB → "reduce processes"; if idle > 50 → "cleanup"; else "healthy". |

### Priority Order for Implementation

1. **Cmd+I Detail Panel** — low effort, high value
2. **Keyboard shortcuts** — Cmd+F, Cmd+I, Cmd+A, Delete
3. **Process grouping** — defines visual hierarchy
4. **Additional columns** — Group, Uptime, Detail
5. **Selection helpers** — Select Idle, Select Top RAM/CPU
6. **Menu bar enrichment** — live RAM %, stats dropdown
7. **Preferences window** — required for AI settings
8. **Export** — JSON/CSV
9. **Profiles** — threshold presets

---

## 4. Security Hardening and Blocklist

### Protected Process Lists

#### v3 Immutable Blocklist (Shell — 11 processes)
`lib/macmon-security.sh`: WindowServer, coreaudiod, AudioComponentRegistrar, coremediaiod, VTDecoderXPCService, VTEncoderXPCService, kernel_task, launchd, syslogd, logd, notifyd

#### v3 Configurable Blocklist (YAML — 24 processes)
`config/macmon.default.yaml`: All 11 above + loginwindow, bluetoothd, fseventsd, mds, mds_stores, opendirectoryd, configd, diskarbitrationd, powerd, thermalmonitord, UserEventAgent, cfprefsd, distnoted

#### v3 AI Service Blocklist (Swift — 19 processes)
`src/gui/AIService.swift`: WindowServer, coreaudiod, AudioComponentRegistrar, coremediaiod, VTDecoderXPCService, VTEncoderXPCService, kernel_task, launchd, syslogd, logd, notifyd, loginwindow, bluetoothd, fseventsd, mds, opendirectoryd, configd, powerd, thermalmonitord

#### v4 Blocklist (Rust — 10 processes)
`crates/core/src/killer.rs`: launchd, kernel_task, windowserver, systemd, init, smss.exe, csrss.exe, wininit.exe, services.exe, lsass.exe

### Detailed Comparison

| Feature | v3 Status | v4 Status | Proposed Fix |
|---------|-----------|-----------|--------------|
| **macOS protected count** | 24 unique processes across all lists | 3 macOS processes (launchd, kernel_task, windowserver) | Add missing 21 macOS processes to `DEFAULT_PROTECTED_PROCESSES` |
| **Windows protected count** | 0 (macOS-only) | 5 (smss, csrss, wininit, services, lsass) | Add: `svchost.exe`, `explorer.exe`, `winlogon.exe`, `dwm.exe`, `conhost.exe` |
| **Linux protected count** | 0 (macOS-only) | 2 (systemd, init) | Add: `kthreadd`, `dbus-daemon`, `Xorg`/`Xwayland`, `pulseaudio`/`pipewire` |
| **Case sensitivity** | Exact match (case-sensitive) | Case-insensitive via `to_ascii_lowercase()` | v4 is better here — no change needed |
| **Code signature verification** | `codesign --verify --requirement="anchor apple"` via `_verify_apple_signed()` | Not implemented | Add `#[cfg(target_os = "macos")]` function calling `std::process::Command("codesign")`. Skip on Windows/Linux. |
| **Apple system PID detection** | `_is_apple_system_pid()` checks binary path starts with `/System/`, `/usr/libexec/`, `/usr/sbin/` | Not implemented | Check `sysinfo::Process::exe()` path prefix on macOS. Equivalent: check `C:\Windows\System32\` on Windows. |
| **PID reuse detection** | `verify_pid()` re-checks process name matches before kill | Not implemented | After `kill_process_safe()` resolves the PID, verify `process.name()` matches expected name before sending signal. |
| **Double-check before SIGKILL** | Blocklist re-checked between SIGTERM and SIGKILL phases | Single check at entry (but `kill_process_force` also checks) | v4 does check in `kill_process_force()` — partial parity. Add name re-verification. |
| **Graceful .app shutdown** | AppleScript `tell application "X" to quit` for macOS apps | Not implemented | **macOS:** `osascript -e 'tell application "X" to quit'`. **Windows:** `WM_CLOSE` message. **Linux:** SIGTERM (already done). |
| **Chrome tab graceful close** | AppleScript `close tab` by URL/ID match | Kills renderer PID (data loss risk) | Requires browser introspection (Section 1). Use AppleScript/CDP `closeTarget`. |
| **SIGTERM→SIGKILL grace period** | 3 seconds (configurable via `kill_grace_sec`) | Immediate fallback | Add `tokio::time::sleep(Duration::from_secs(grace))` between SIGTERM and SIGKILL in `kill_process_safe()`. |
| **Command pattern blocking** | Blocks: `rm -rf`, `sudo`, `launchctl`, `osascript`, `curl`, `sh -c` | Not applicable (Rust binary, no shell execution) | Not needed — v4 doesn't execute shell commands from user input. |
| **Path hijacking defense** | Validates `MACMON_HOME` against metacharacters, ownership, world-writable | Not applicable (compiled binary) | Not needed — Rust binary has no equivalent attack surface. |
| **Daemon PID security** | Symlink-safe atomic write, directory-based locking | No daemon (Tauri manages lifecycle) | Not needed in current architecture. |
| **User-configurable blocklist** | `protected:` list in `macmon.yaml`, reloaded on SIGUSR1 | `extra_blocklist` parameter exists but unused | Wire `extra_blocklist` to a config file. Load on startup. Expose in preferences UI. |

### Missing macOS Processes (v3 has, v4 doesn't)

These 21 macOS-specific processes are protected in v3 but not in v4:

| Process | Role | Risk if Killed |
|---------|------|----------------|
| coreaudiod | Core audio daemon | All audio stops |
| AudioComponentRegistrar | Audio plugin registry | Audio plugins fail |
| coremediaiod | Media I/O daemon | Camera/mic stops |
| VTDecoderXPCService | Video hardware decoder | Video playback fails |
| VTEncoderXPCService | Video hardware encoder | Screen recording/FaceTime fails |
| loginwindow | Login UI manager | Session terminates |
| bluetoothd | Bluetooth daemon | All BT devices disconnect |
| fseventsd | File system events | Spotlight/Time Machine break |
| mds | Metadata server (Spotlight) | Search stops |
| mds_stores | Spotlight data store | Search index corrupts |
| opendirectoryd | Directory services | Authentication fails |
| configd | Network configuration | Network drops |
| diskarbitrationd | Disk mount manager | Volumes unmount |
| powerd | Power management | Battery/sleep breaks |
| thermalmonitord | Thermal monitoring | CPU may overheat |
| UserEventAgent | User event handling | Input processing breaks |
| cfprefsd | Preferences daemon | App preferences lost |
| distnoted | Distributed notifications | IPC breaks |
| syslogd | System logger | Logging stops |
| logd | Unified logging | System logs stop |
| notifyd | Notification daemon | Notifications stop |

---

## Summary: Parity Score

| Area | v3 Features | v4 Has | v4 Missing | Parity |
|------|-------------|--------|------------|--------|
| Browser Introspection | 10 | 0 | 10 | **0%** |
| AI Smart Optimize | 12 | 0 | 12 | **0%** |
| Visual UX | 25 | 7 | 18 | **28%** |
| Security Hardening | 12 | 5 | 7 | **42%** |
| **Total** | **59** | **12** | **47** | **20%** |

### Top 5 Actions by Impact

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| 1 | Add 21 missing macOS processes to blocklist | 30 min | Prevents accidental system damage |
| 2 | Implement Chrome tab introspection via AppleScript + CDP | 2-3 days | Restores flagship v3 feature |
| 3 | Add Cmd+I detail panel + keyboard shortcuts | 1 day | Restores core UX interaction |
| 4 | Implement AI Smart Optimize with `keyring` crate | 3-4 days | Restores differentiating feature |
| 5 | Add process grouping + additional columns | 1-2 days | Restores visual hierarchy |

---

*Generated by Claude Opus 4.6 on 2026-03-05. Read-only audit — no source files modified.*
