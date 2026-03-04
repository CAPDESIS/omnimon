# macmon Cross-Platform Research: Mac, Linux, Windows

**Date:** March 2026
**Author:** Jorge Salgado Miranda
**Status:** Research Document for v4.0.0 Planning

## Context

macmon is currently a macOS-only system monitor built on AppKit (Swift native GUI), a Bash daemon managed by launchd (LaunchAgent), and a CLI interface. The goal of this research is to evaluate cross-platform desktop stacks that can bring the same "Monitor + AI Human-in-the-Loop" concept to Windows and Linux while preserving macmon's lightweight, security-first philosophy.

The current architecture (v2.x/v3.x) relies on:
- **Bash scripts** for process collection (`ps`, `lsof`, `sysctl`), daemon logic, and CLI
- **Swift/AppKit** for the native ProcessPicker GUI and MacmonStatusBar menu bar app
- **AppleScript** for graceful Chrome tab management
- **launchd LaunchAgent** for daemon autostart
- **YAML** configuration at `~/.config/macmon/macmon.yaml`
- **Keychain** for AI API key storage

None of these are directly portable to Linux or Windows. A cross-platform rewrite requires selecting a stack that covers GUI rendering, system-level process APIs, background service management, and packaging for all three operating systems.

---

## 1. Comparative Matrix

| Dimension | Rust + Tauri v2 + Svelte | Go + Wails + Svelte | Electron + React/Svelte | Flutter Desktop + Dart FFI | Qt C++ (native widgets) |
|---|---|---|---|---|---|
| **Binary size** | ~5-8 MB | ~8-15 MB | ~150-200 MB | ~20-50 MB | ~15-30 MB |
| **RAM usage** | ~30-40 MB | ~35-50 MB | ~200-300 MB | ~80-120 MB | ~40-60 MB |
| **Startup time** | <500 ms | <500 ms | 1-3 s | 1-2 s | <500 ms |
| **System API access** | Excellent (`sysinfo`, `nix`, `windows` crates) | Excellent (`gopsutil`, `x/sys`) | Good (`systeminformation` npm, native addons) | Weak (FFI per platform, no unified lib) | Good (manual per-platform code) |
| **Process kill support** | Native via `sysinfo::Process::kill()` | Native via `gopsutil` | Via native addon or child_process | Requires FFI bridge per OS | Manual per-platform implementation |
| **System tray** | First-class (`tray-icon` API) | Built-in support | Mature (multiple libraries) | Plugin-based, less mature | Native `QSystemTrayIcon` |
| **Packaging** | Built-in: .dmg, .deb, .AppImage, .msi, .exe | Built-in: .dmg, .deb, .exe | electron-builder: all formats | Flutter build: .dmg, .msix, .deb | CPack/manual: all formats |
| **i18n** | Frontend libs (`svelte-i18n`, `typesafe-i18n`) | Frontend libs | Mature ecosystem (`i18next`) | Flutter built-in `intl` | Qt Linguist (mature) |
| **Rendering consistency** | OS webview (varies by platform) | OS webview (varies by platform) | Bundled Chromium (identical everywhere) | Skia engine (identical everywhere) | Native widgets (varies by platform) |
| **Learning curve** | Moderate-High (Rust + Svelte) | Moderate (Go + Svelte) | Low (JS/TS ecosystem) | Moderate (Dart + FFI) | High (C++ + Qt framework) |
| **Community** | Growing rapidly, active | Smaller but dedicated | Massive, most mature | Large (mobile-first) | Large but aging |
| **Auto-updater** | Built-in plugin with crypto signatures | Manual or third-party | electron-updater (mature) | Manual implementation | Manual implementation |
| **License** | MIT/Apache-2.0 | MIT | MIT | BSD-3-Clause | LGPL (complex for static linking) |

---

## 2. Detailed Analysis per Stack

### 2.1 Rust + Tauri v2 + Svelte (Recommended)

Tauri v2 represents the most compelling option for macmon's cross-platform future. It uses the operating system's native webview for rendering (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux), which means the application inherits the OS's own rendering engine rather than bundling a separate browser runtime. This architectural decision is what enables Tauri apps to ship as ~5 MB binaries consuming ~35 MB of RAM at runtime, a footprint that aligns perfectly with macmon's philosophy of being a lightweight system monitor that does not itself become a resource hog.

The Rust backend provides direct access to the `sysinfo` crate, which is the single most important library for this project. `sysinfo` provides cross-platform process listing, CPU usage, memory consumption, swap usage, disk I/O, and network statistics through a unified API. It covers an estimated 90%+ of macmon's current data collection needs without any platform-specific code. Process killing is also handled through `sysinfo::Process::kill()` with platform-appropriate behavior (SIGTERM/SIGKILL on Unix, TerminateProcess on Windows). The NeoHtop project (github.com/Abdenasser/neohtop) serves as a proven reference implementation, demonstrating that a Tauri-based system monitor is not just theoretically sound but practically viable.

Tauri v2 ships with a built-in bundler that produces .dmg files for macOS, .deb and .AppImage packages for Linux, and .msi/.exe installers for Windows. The built-in auto-updater plugin supports cryptographic signature verification, meaning macmon can offer secure self-updates on all platforms without integrating third-party update frameworks. The system tray API is first-class, supporting icon display, context menus, and click handlers on all three platforms, which directly maps to macmon's current MacmonStatusBar functionality.

The security model is another strong fit. Tauri enforces explicit permission grants for IPC between the frontend and backend, preventing the frontend from calling arbitrary system commands without declaration. This aligns with macmon's security-first approach, where process killing requires human confirmation and protected processes cannot be terminated. Rust's memory safety guarantees further reduce the attack surface of the backend daemon component.

The primary weakness is Linux rendering. WebKitGTK does not always render identically to WebKit on macOS or WebView2 on Windows, and some CSS features or font rendering may differ. This can be mitigated with thorough cross-platform CSS testing and by using AppImage bundling to include a known-good version of WebKitGTK. The Rust learning curve is also non-trivial, though developers with Swift or C experience typically achieve productive proficiency within 2-4 weeks.

### 2.2 Go + Wails + Svelte

Wails is Go's answer to Tauri, using the same OS-webview approach to keep binaries small (8-15 MB) and RAM usage modest (~35-50 MB). The Go ecosystem offers `gopsutil` (github.com/shirou/gopsutil), an excellent cross-platform library for system information that provides process listing, CPU/memory/disk/network metrics, and process management. For a system monitor, `gopsutil` is comparable in coverage to Rust's `sysinfo` crate, and Go's straightforward error handling and goroutine concurrency model make it easy to implement polling loops and background data collection.

The development velocity with Go is notably faster than Rust for backend-heavy applications. Go's compilation speed, simpler type system, and garbage collector mean less time wrestling with the borrow checker and lifetime annotations. For a small team or solo developer, this can translate to significantly faster iteration cycles during the initial development phase. The single-binary deployment story is also clean: `wails build` produces a self-contained executable per platform.

However, Wails has a smaller community and ecosystem compared to Tauri. Plugin availability is more limited, and the auto-update story requires either manual implementation or integration with third-party solutions like `go-selfupdate`. The Linux WebKitGTK rendering issue is identical to Tauri's since both use the same underlying webview technology. Additionally, Go's garbage collector introduces occasional pause events that, while typically sub-millisecond, could cause micro-stutters in a real-time process table that updates every second. This is unlikely to be user-visible but is a theoretical concern for a monitoring tool that must remain responsive.

The system tray support in Wails is functional but less polished than Tauri's. Menu construction and event handling work across platforms, but advanced features like animated tray icons or rich popup windows may require additional native code. For macmon's needs (displaying CPU/RAM in the menu bar, showing a context menu), the basic support is sufficient.

### 2.3 Electron + React/Svelte

Electron is the most mature and widely-deployed cross-platform desktop framework, powering applications like VS Code, Slack, Discord, and Spotify. Its key advantage is rendering consistency: by bundling Chromium, it guarantees pixel-identical rendering across macOS, Linux, and Windows. The `systeminformation` npm package (github.com/sebhildebrandt/systeminformation) provides comprehensive cross-platform system data, and the Node.js runtime offers access to `process.kill()` and child process management for process control.

The ecosystem benefits are substantial. Hundreds of well-maintained libraries exist for tables (AG Grid, TanStack Table), i18n (i18next, react-intl), auto-updating (electron-updater with S3/GitHub Releases), system tray management, and packaging (electron-builder supports every format). Developer hiring is also easiest for Electron since the required skills (JavaScript/TypeScript, React/Svelte, CSS) are the most common in the industry.

The fatal weakness for macmon is resource consumption. An Electron app bundles an entire Chromium browser and V8 JavaScript engine, resulting in 150-200 MB binaries and 200-300 MB of RAM usage at idle. For a general-purpose desktop application this is acceptable, but for a system monitor whose entire purpose is to help users reduce resource waste, shipping a 200 MB binary that consumes 300 MB of RAM is deeply ironic and undermines the product's credibility. Users running `macmon` to identify resource-hungry processes would see Electron-based macmon itself appearing near the top of the list.

Electron's security model has also been historically problematic. The `nodeIntegration` and context isolation settings require careful configuration to prevent the renderer process from accessing Node.js APIs directly, and the large attack surface of Chromium means frequent security patches. While these concerns are manageable with diligent engineering, they represent ongoing maintenance overhead that does not exist with Tauri's smaller surface area.

### 2.4 Flutter Desktop + Dart FFI

Flutter Desktop brings Google's cross-platform UI toolkit to macOS, Linux, and Windows, using the Skia rendering engine to draw every pixel identically across platforms. This gives Flutter the most consistent visual experience of any option on this list, including pixel-perfect font rendering, animation smoothness, and widget behavior. The Material Design and Cupertino widget sets provide polished, platform-appropriate UI components out of the box.

For a system monitor, Flutter's main challenge is system-level access. Unlike Rust's `sysinfo` or Go's `gopsutil`, the Dart ecosystem lacks a single, mature, cross-platform library for process listing and system metrics. The `process_run` package provides basic process execution, but collecting detailed per-process CPU and memory data requires writing Dart FFI bindings to platform-specific C libraries or spawning native helper processes. This means implementing three separate native plugins (one per platform) for the core monitoring functionality, which negates much of the cross-platform benefit.

The binary size (20-50 MB) and RAM usage (~80-120 MB) are in the middle range, heavier than Tauri or Wails but much lighter than Electron. Flutter Desktop is still maturing on Linux and Windows, with some platform-specific bugs and missing features compared to the mobile variants. The plugin ecosystem for desktop-specific features (system tray, global hotkeys, autostart) is less mature than mobile, though improving rapidly.

Flutter would be a strong choice if macmon were primarily a UI-heavy application with minimal system-level requirements. For a tool whose core value proposition is deep system monitoring and process management, the FFI overhead and lack of unified system libraries make it a suboptimal fit. The development team would spend more time building platform bridges than building features.

### 2.5 Qt C++ (native widgets)

Qt is the most mature desktop framework in this comparison, with over 25 years of production use in applications ranging from KDE Plasma to Autodesk Maya. Its native widget approach means that a `QPushButton` on macOS renders as a genuine Cocoa button, on Windows as a Win32 button, and on Linux as a GTK/Qt-themed button. For a process table, `QTableView` with `QSortFilterProxyModel` provides exceptional performance: virtual scrolling, lazy rendering, and efficient sorting of thousands of rows are built-in capabilities that have been optimized over decades.

Qt's system integration is strong. `QSystemTrayIcon` provides system tray support on all platforms, `QProcess` handles process spawning, and the Qt Network module supports HTTP/WebSocket for AI API integration. Qt Linguist provides a mature translation workflow with context-aware string management, `.ts` translation files, and runtime language switching.

The primary concern is licensing. Qt is available under LGPL, which requires either dynamic linking (to allow users to replace the Qt libraries) or a commercial license ($300+/month per developer). For an open-source project like macmon, LGPL compliance adds complexity to the build and distribution process, particularly on macOS where static linking is the norm for desktop applications. The commercial license removes this restriction but adds ongoing cost.

The second concern is the lack of a unified system monitoring library equivalent to `sysinfo` or `gopsutil`. Qt does not provide cross-platform process listing or system metrics APIs; the developer must write platform-specific code using `proc_pidinfo` on macOS, `/proc` filesystem on Linux, and Toolhelp/PSAPI on Windows. While this is entirely feasible, it represents significant additional development effort and ongoing maintenance burden for three separate code paths. The auto-update story also requires manual implementation or integration with third-party frameworks like Sparkle (macOS) and WinSparkle (Windows).

---

## 3. Verdict: Recommended Stack for v4.0.0

### Rust + Tauri v2 + Svelte

After evaluating all five candidates across the dimensions that matter most for a cross-platform system monitor, **Rust + Tauri v2 + Svelte** is the clear recommendation for macmon v4.0.0. The rationale is as follows:

1. **`sysinfo` crate unifies system monitoring across platforms.** A single Rust dependency provides cross-platform process listing (`System::processes()`), process killing (`Process::kill()`), CPU usage, total/used memory, swap, disk I/O, and network statistics. This eliminates the need to maintain three separate platform-specific collection backends, which is the single largest engineering risk in a cross-platform system monitor.

2. **Tiny footprint aligns with macmon's lightweight philosophy.** A ~5-8 MB binary consuming ~35 MB of RAM sends the right message for a tool that helps users optimize their system resources. This is 30-40x smaller than Electron in binary size and 6-8x smaller in RAM consumption.

3. **Built-in bundler handles all target formats.** Tauri's bundler produces .dmg (macOS), .deb and .AppImage (Linux), and .msi and .exe (Windows) from a single configuration file. No third-party packaging tools are required.

4. **Built-in auto-updater with cryptographic signature verification.** The Tauri updater plugin supports update manifests, delta updates, and Ed25519 signature verification out of the box. macmon can offer secure self-updates on all platforms without integrating Sparkle, WinSparkle, or custom update servers.

5. **System tray is first-class.** The `tray-icon` API supports icon display, context menus, tooltip text, and click event handlers on all three platforms. This directly replaces macmon's current `MacmonStatusBar.swift` with cross-platform equivalent functionality.

6. **Security-first permission model.** Tauri v2 requires explicit declaration of which IPC commands the frontend can invoke, which Rust APIs the backend exposes, and which shell commands can be executed. This maps naturally to macmon's human-in-the-loop philosophy where process killing requires explicit user confirmation.

7. **NeoHtop is a proven reference implementation.** The NeoHtop project (github.com/Abdenasser/neohtop) demonstrates a working Tauri-based system monitor with process listing, sorting, filtering, and system metrics. This validates the architecture and provides code-level reference for common patterns.

8. **Rust's memory safety matches macmon's security posture.** A system monitor with process kill capabilities is a security-sensitive application. Rust's ownership model eliminates entire classes of vulnerabilities (buffer overflows, use-after-free, data races) that could be exploited to escalate privileges or kill unintended processes.

---

## 4. Component Migration Map

This section maps every current macmon component to its cross-platform equivalent in the Tauri v2 architecture.

- **Daemon (Bash + LaunchAgent)**
  - Current: `src/daemon/macmond.sh` managed by `com.macmon.daemon` LaunchAgent plist
  - macOS: Rust binary registered as LaunchAgent (`~/Library/LaunchAgents/com.macmon.daemon.plist`), or retain current Bash daemon during transition
  - Linux: Rust binary registered as systemd user service (`~/.config/systemd/user/macmon.service` unit file)
  - Windows: Rust binary registered as Windows Task Scheduler task (startup trigger), or Windows Service for always-on monitoring

- **GUI (AppKit / ProcessPicker.swift)**
  - Current: `ProcessPicker.swift` compiled with `swiftc -framework Cocoa`, universal binary via `lipo`
  - All platforms: Tauri window rendering Svelte frontend
    - Table component: TanStack Table or AG Grid with virtual scrolling for 1000+ process rows
    - Sorting/filtering: Frontend-side with Svelte reactive stores, backed by Rust-side data
    - System tray: Tauri `tray-icon` API with context menu (Show/Hide, Quick Stats, Quit)
    - Theme: System-aware light/dark mode via CSS `prefers-color-scheme` media query

- **Process Collection (Bash ps/lsof/sysctl)**
  - Current: `lib/macmon-core.sh` using `ps aux`, `lsof -i`, `sysctl hw.memsize`
  - All platforms: Rust `sysinfo` crate
    - `System::new_all()` for initial scan
    - `System::refresh_processes()` for delta updates
    - `System::processes()` returns `HashMap<Pid, Process>` with name, CPU%, memory, status, user
    - `System::total_memory()`, `System::used_memory()`, `System::total_swap()`, `System::used_swap()`
    - `System::cpus()` for per-core usage
    - `System::disks()` for disk I/O and capacity

- **Process Killing (Bash kill + graceful-quit.sh AppleScript)**
  - Current: `kill -TERM` / `kill -KILL` in Bash, `osascript` for graceful Chrome tab close
  - macOS: `sysinfo::Process::kill()` for standard processes + `objc2` crate or `Command::new("osascript")` for AppleScript-equivalent graceful quit
  - Linux: `sysinfo::Process::kill_with(Signal::Term)` for graceful, `sysinfo::Process::kill_with(Signal::Kill)` for force
  - Windows: `sysinfo::Process::kill()` (internally calls `TerminateProcess`) for force kill + `windows` crate for sending `WM_CLOSE` message for graceful quit

- **Chrome Tab Management (AppleScript)**
  - Current: `scripts/graceful-quit.sh` using `osascript` to enumerate and close Chrome tabs
  - macOS: Keep AppleScript integration via Tauri shell plugin (`Command::new("osascript").args(...)`)
  - Linux: Chrome DevTools Protocol (CDP) via WebSocket connection to `localhost:9222` (Chrome must be launched with `--remote-debugging-port=9222`)
  - Windows: Chrome DevTools Protocol (CDP) via WebSocket, same approach as Linux
  - Shared: Rust CDP client library (e.g., `chromiumoxide` or `headless_chrome` crate) for tab enumeration and closing

- **Configuration (YAML + macmon-config.sh)**
  - Current: `~/.config/macmon/macmon.yaml` parsed by Bash with `_expand_tilde()`
  - All platforms: Rust YAML parser via `serde` + `serde_yaml` crates
  - Config file locations (platform-appropriate):
    - macOS: `~/Library/Application Support/macmon/config.yaml` or `~/.config/macmon/config.yaml` (XDG fallback)
    - Linux: `~/.config/macmon/config.yaml` (XDG_CONFIG_HOME)
    - Windows: `%APPDATA%\macmon\config.yaml`
  - Migration: Automatic detection and migration of existing `~/.config/macmon/macmon.yaml` on macOS

- **AI Integration (AIService.swift + Keychain)**
  - Current: `AIService.swift` with macOS Keychain for API key storage, HTTPS calls to AI providers
  - All platforms: Rust HTTP client via `reqwest` crate (async, TLS-native)
  - API key storage via `keyring` crate:
    - macOS: Keychain Services
    - Linux: Secret Service API (GNOME Keyring / KDE Wallet)
    - Windows: Windows Credential Manager
  - AI prompt construction: Rust-side with `serde_json` for structured payloads
  - Privacy controls: Same minimized payload defaults (process name + PID + CPU% + memory, no URLs unless opted in)

- **Telemetry (history.jsonl)**
  - Current: Append-only JSONL file for process history and AI recommendations
  - All platforms: Rust `serde_json` for serialization + `std::fs::OpenOptions::append()` for atomic appends
  - File locations:
    - macOS: `~/Library/Application Support/macmon/history.jsonl`
    - Linux: `~/.local/share/macmon/history.jsonl` (XDG_DATA_HOME)
    - Windows: `%LOCALAPPDATA%\macmon\history.jsonl`
  - Same JSONL schema for cross-platform compatibility

- **Localization (Localizable.strings)**
  - Current: `Resources/*.lproj/Localizable.strings` (Apple format)
  - All platforms: Frontend i18n library in Svelte
    - Recommended: `typesafe-i18n` for compile-time type safety on translation keys
    - Alternative: `svelte-i18n` for runtime flexibility
  - Translation file format: JSON (one file per locale, e.g., `locales/en.json`, `locales/es.json`)
  - Backend strings (error messages, CLI output): Rust `fluent` crate or embedded string tables

- **Menu Bar (MacmonStatusBar.swift)**
  - Current: Swift AppKit `NSStatusItem` with `NSMenu`, shows CPU/RAM in menu bar text
  - All platforms: Tauri `tray-icon` with:
    - Context menu: Show Window, Quick Stats submenu, Settings, Quit
    - Tooltip: Current CPU% and RAM usage
    - Optional: Small popup window on click (Tauri panel/popup window API)
  - Platform notes:
    - macOS: `NSStatusItem`-equivalent via Tauri tray
    - Linux: AppIndicator / StatusNotifierItem (Tauri handles both variants)
    - Windows: System tray notification area icon

- **Auto-update (macmon update)**
  - Current: `macmon update` CLI command pulling from GitHub Releases
  - All platforms: Tauri updater plugin
    - Update manifest: JSON file hosted on GitHub Releases or custom server
    - Signature verification: Ed25519 signatures on update bundles
    - Update flow: Check -> Download -> Verify signature -> Apply -> Restart
    - CLI fallback: `macmon update` command triggers same updater logic via IPC to running app, or standalone HTTP check

---

## 5. Migration Risk Assessment

### Rust Learning Curve
- **Risk level:** Moderate
- **Impact:** Development velocity reduction during initial 2-4 weeks
- **Detail:** Developers with Swift or C/C++ experience typically find Rust's ownership model conceptually familiar but syntactically challenging. The borrow checker will reject valid-seeming code during the learning phase. Lifetime annotations, trait bounds, and async Rust (required for `reqwest` HTTP client and Tauri command handlers) add complexity.
- **Mitigation:** Start with the `sysinfo` integration (straightforward Rust, minimal lifetime complexity), build confidence before tackling async IPC and Tauri command bridges. Use `clippy` and `rust-analyzer` from day one. Reference NeoHtop source code for Tauri-specific patterns.

### Linux WebKitGTK Rendering
- **Risk level:** Low-Moderate
- **Impact:** Visual inconsistencies on Linux, potential CSS layout differences
- **Detail:** WebKitGTK on Linux does not always match WebKit on macOS in CSS feature support, font rendering, or scrollbar behavior. Some CSS properties (backdrop-filter, some flexbox edge cases) may render differently or not at all. Different Linux distributions ship different WebKitGTK versions.
- **Mitigation:** Test on Ubuntu 22.04+ and Fedora 38+ as primary targets. Use AppImage packaging to bundle a known-good WebKitGTK version. Avoid cutting-edge CSS features. Implement a CSS normalization layer. Use `postcss` with autoprefixer for vendor prefix management.

### Windows SIGTERM Absence
- **Risk level:** Low-Moderate
- **Impact:** Graceful process termination behaves differently on Windows
- **Detail:** Windows does not have Unix signals. `TerminateProcess()` is the equivalent of `SIGKILL` (immediate, ungraceful). There is no direct equivalent of `SIGTERM` for console applications. GUI applications can be gracefully closed by sending `WM_CLOSE` to their main window, but headless/console processes have no standard graceful shutdown mechanism.
- **Mitigation:** Use the `windows` crate to send `WM_CLOSE` messages for GUI processes (equivalent to macmon's current AppleScript graceful quit). For console processes, use `GenerateConsoleCtrlEvent(CTRL_C_EVENT)` as a SIGTERM equivalent. Document the behavioral difference. The `sysinfo::Process::kill()` function already handles the platform-appropriate default.

### Chrome Tab Management on Non-macOS
- **Risk level:** Low
- **Impact:** Chrome tab enumeration and closing requires different mechanism on Linux/Windows
- **Detail:** macmon currently uses AppleScript to interact with Chrome, which is macOS-only. On Linux and Windows, the Chrome DevTools Protocol (CDP) provides equivalent functionality via a WebSocket API, but requires Chrome to be launched with `--remote-debugging-port` flag or have the debugging port enabled via Chrome flags.
- **Mitigation:** CDP is a well-documented, stable protocol used by Puppeteer, Playwright, and Chrome DevTools itself. The `chromiumoxide` Rust crate provides a client implementation. Implement capability detection: if CDP is unavailable, fall back to process-only management (kill the Chrome process rather than individual tabs) with a user-facing message explaining how to enable CDP.

### Platform-Specific Autostart Fragmentation
- **Risk level:** Low
- **Impact:** Different installation and autostart mechanisms per OS
- **Detail:** macOS uses LaunchAgent plist files, Linux uses systemd user services (with fallback needed for non-systemd distros), and Windows uses Task Scheduler or Registry Run keys. Each has different permission models, logging mechanisms, and failure modes.
- **Mitigation:** Abstract autostart behind a platform trait in Rust. Test on the primary target for each OS (launchd on macOS, systemd on Linux, Task Scheduler on Windows). Defer non-systemd Linux support (e.g., OpenRC, runit) to a future release based on user demand.

---

## 6. Timeline Estimate

### Phase 1: Rust Backend with sysinfo (2-3 weeks)
- Set up Rust workspace with `macmon-core` library crate
- Implement process listing via `sysinfo::System::processes()`
- Implement system metrics (CPU, RAM, swap, disk) via `sysinfo`
- Implement process killing with graceful/force modes
- Implement protected process blocklist (port from current Bash logic)
- Implement YAML config parsing with `serde_yaml`
- Unit tests for all core functions, validate parity with current macOS output
- **Deliverable:** Rust library crate that can be called from CLI or Tauri

### Phase 2: Svelte Frontend + Tauri Shell (2-3 weeks)
- Initialize Tauri v2 project with Svelte frontend
- Build process table with TanStack Table (virtual scrolling, sorting, filtering)
- Build system metrics dashboard (CPU/RAM/swap/disk gauges or charts)
- Implement Tauri IPC commands bridging frontend to Rust backend
- Implement system tray with context menu and tooltip stats
- Implement light/dark theme support
- Implement search and filter UI for process list
- **Deliverable:** Working Tauri app on macOS with process table and tray

### Phase 3: Platform-Specific Features (2 weeks)
- macOS: AppleScript integration for Chrome tab management via Tauri shell plugin
- Linux: CDP client for Chrome tab management, systemd service file generation
- Windows: CDP client for Chrome tabs, Task Scheduler registration, WM_CLOSE graceful quit
- All platforms: Autostart install/uninstall commands
- Cross-platform testing on macOS 14+, Ubuntu 22.04+, Windows 10+
- **Deliverable:** Feature parity across all three platforms for core monitoring

### Phase 4: AI Integration + Localization (1-2 weeks)
- Port AI service to Rust with `reqwest` HTTP client
- Implement API key storage via `keyring` crate (Keychain/SecretService/CredentialManager)
- Implement AI recommendation UI in Svelte (suggestion cards, accept/reject actions)
- Set up `typesafe-i18n` with English and Spanish locales
- Migrate existing `Localizable.strings` content to JSON locale files
- **Deliverable:** AI-powered recommendations and bilingual UI

### Phase 5: Packaging + CI/CD (1 week)
- Configure Tauri bundler for .dmg (macOS), .deb + .AppImage (Linux), .msi (Windows)
- Set up GitHub Actions CI matrix: macOS-14, ubuntu-latest, windows-latest
- Configure Tauri updater plugin with GitHub Releases as update source
- Generate Ed25519 signing keys for update signature verification
- Write platform-specific installation documentation
- Smoke test full install/update/uninstall cycle on all three platforms
- **Deliverable:** Automated release pipeline producing signed installers for all platforms

### Total Estimate: 8-11 weeks for v4.0.0

This estimate assumes a single developer working full-time. Parallel development with a second contributor (e.g., one on Rust backend, one on Svelte frontend) could reduce the timeline to 5-7 weeks. The estimate includes testing but does not include beta testing or community feedback cycles.

---

## References

- Tauri v2 documentation: https://v2.tauri.app
- sysinfo crate: https://crates.io/crates/sysinfo
- NeoHtop (Tauri system monitor): https://github.com/Abdenasser/neohtop
- Wails framework: https://wails.io
- gopsutil library: https://github.com/shirou/gopsutil
- Chrome DevTools Protocol: https://chromedevtools.github.io/devtools-protocol/
- keyring crate: https://crates.io/crates/keyring
- typesafe-i18n: https://github.com/ivanhofer/typesafe-i18n
- Tauri updater plugin: https://v2.tauri.app/plugin/updater/
