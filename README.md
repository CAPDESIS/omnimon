# OmniMon v4

[![CI/CD](https://github.com/chochy2001/omnimon/actions/workflows/omnimon-ci.yml/badge.svg)](https://github.com/chochy2001/omnimon/actions) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_+_Svelte-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#)

OmniMon is a next-generation system monitor rewritten from scratch in a modern monorepo. It replaces the legacy AppKit/Bash architecture with a hyper-optimized native core and a reactive UI with zero memory leaks.

## Architecture

The project follows a strict modular approach, separating the native backend from the presentation layer, communicating through Tauri's IPC bus:

* **Native Core (`v4/crates/core`):** Written in Rust. Uses `sysinfo` for hardware-level telemetry, the CDP (Chrome DevTools Protocol) for granular browser tab analysis, and direct FFI calls to Win32/libc for low-level OS operations.
* **Presentation Layer (`v4/apps/desktop`):** Compact interface built with Svelte 5 and TypeScript on Tauri. Guarantees minimal memory footprint and a strictly controlled component lifecycle. Virtual scroll renders 2000+ processes at 60 FPS.
* **CLI & Tools (`v4/crates/cli`):** High-performance terminal interface for headless control and server automation.

## Key Features

* **Smart Optimize (AI Flow):** Predictive resolution and AI-powered resource optimization. Built-in support for leading providers (OpenAI, Anthropic, OpenRouter).
* **Integrated Cross-Platform Security (Native Keychain):** Credentials and API keys are *never* stored in plain text. OmniMon delegates storage to the native system (macOS Keychain, Windows Credential Manager, Linux Secret Service).
* **Secure Blocklists:** Dynamic and immutable per-OS block lists that prevent accidental termination of critical processes (e.g. `smss.exe` on Windows or `launchd` on macOS).
* **Feature Parity:** Consistent experience regardless of the underlying platform (.exe, .dmg, .deb).

## Quick Start

### macOS (Homebrew)

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon
```

The app launches automatically after install. You can also find it in **Spotlight** (`Cmd + Space` → "OmniMon") or in `/Applications/OmniMon.app`.

### Windows

Download the `.msi` installer from the [latest release](https://github.com/chochy2001/omnimon/releases/latest).

### Linux

```bash
# Debian/Ubuntu — one-liner
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/install-web.sh | bash

# Or download the .deb / .AppImage from the latest release
```

### Build from Source

```bash
git clone https://github.com/chochy2001/omnimon.git
cd omnimon/v4
./setup-dev.sh
make dev
```

## CLI Usage (Build from Source)

```bash
cd v4
cargo run -p cli -- --help
cargo run -p cli -- optimize --ai anthropic --target browsers
cargo run -p cli -- status --format json
```

## License

[MIT](LICENSE)
