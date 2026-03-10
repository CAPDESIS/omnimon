# OmniMon

[![CI](https://github.com/chochy2001/omnimon/actions/workflows/omnimon-ci.yml/badge.svg)](https://github.com/chochy2001/omnimon/actions) [![Version](https://img.shields.io/badge/version-6.3.0-brightgreen)](#) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_2_+_Svelte_5-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#) [![License](https://img.shields.io/badge/license-MIT-green)](LICENSE) [![Sponsor](https://img.shields.io/badge/Sponsor-💖-ff69b4)](https://github.com/sponsors/chochy2001)

```
  ___  __  __ _   _ ___ __  __  ___  _   _
 / _ \|  \/  | \ | |_ _|  \/  |/ _ \| \ | |
| | | | |\/| |  \| || || |\/| | | | |  \| |
| |_| | |  | | |\  || || |  | | |_| | |\  |
 \___/|_|  |_|_| \_|___|_|  |_|\___/|_| \_|
```

*Scroll down for Spanish / Desplázate hacia abajo para Español.*

OmniMon is a next-generation, cross-platform system monitor built with Rust and Svelte. It provides real-time process telemetry, AI-powered optimization, network security analysis, and browser tab management — all from a single desktop app, CLI, or TUI.

## Highlights

- **Real-time system monitoring** — CPU, memory, disk I/O, network throughput, and energy impact per process with 2-second refresh
- **AI-powered optimization** — Multi-provider support (OpenAI, Anthropic, Gemini, OpenRouter, Ollama) with tool calling for hands-free process management
- **Network intelligence** — Native packet capture (libpcap/eBPF/WinDivert), connection analysis, DNS enrichment, GeoIP, and MITRE ATT&CK correlation
- **Browser tab control** — List, close, and focus tabs across Chrome, Safari, Brave, Edge, Arc, and Firefox via CDP and AppleScript
- **Security & compliance** — CVE auditing, NIST SP 800-53 heartbeats, encrypted audit trails, Ed25519 release signing
- **User profiles** — Preset configurations (minimal/balanced/power) with customizable dashboard layouts, refresh intervals, and favorite processes
- **SRE automations** — User-defined rules for alerts and auto-actions with native OS notifications
- **Plugin system** — Extend monitoring with Lua plugins (sandboxed, 150ms timeout, 1MB memory limit)
- **Three interfaces** — Desktop GUI, CLI, and interactive TUI (htop-style)

## Screenshots

> Screenshots available in [GitHub Releases](https://github.com/chochy2001/omnimon/releases/latest).

## Quick Start

### macOS (Homebrew)

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon
```

The app launches automatically after install. Find it in **Spotlight** (`Cmd + Space` → "OmniMon") or in `/Applications/OmniMon.app`.

### Windows

Download the `.msi` installer from the [latest release](https://github.com/chochy2001/omnimon/releases/latest).

### Linux

```bash
# Debian/Ubuntu — one-liner
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash

# Or download the .deb / .AppImage / .rpm from the latest release
```

### Build from Source

```bash
git clone https://github.com/chochy2001/omnimon.git
cd omnimon/v4
./setup-dev.sh   # Installs Rust, bun, Tauri CLI, OS deps
make dev          # Launches dev mode (Vite + Tauri hot-reload)
```

## Usage

### Desktop GUI

Launch OmniMon from your applications menu or system tray. The app runs in the background with a tray icon.

### CLI

```bash
omnimon status                                    # System overview
omnimon status --format json                      # Machine-readable output
omnimon optimize --ai anthropic --target browsers # AI optimization
omnimon network --top                             # Network throughput
omnimon network --connections --filter tcp         # TCP connections
omnimon network --alerts --watch                  # Live alert stream
omnimon chat --ai openai "What is using the most RAM?"
omnimon security-scan --cve-db ./cves.json        # CVE audit
omnimon rules list                                # Security rules
omnimon release sign --version 6.3.0 ./omnimon    # Sign binary
omnimon doctor                                    # Health check
```

### TUI (Terminal UI)

```bash
omnimon tui
```

Interactive htop-style dashboard with process table, system gauges, and built-in AI chat. Keyboard-driven: `s` to sort, `K` to kill, `Tab` to switch panels.

## AI Providers

| Provider | Model (default) | API Key Required |
|----------|----------------|-----------------|
| OpenAI | gpt-4o-mini | Yes |
| Anthropic | claude-haiku-4-5 | Yes |
| Gemini | gemini-2.0-flash | Yes |
| OpenRouter | llama-3.2-3b (free) | Yes |
| Ollama | llama3.2 (local) | No |

Store keys securely: `omnimon apikey --ai <provider> <key>` (stored in OS Keyring).

## Documentation

| Document | Description |
|----------|-------------|
| [CLI Manual](docs/CLI_MANUAL.md) | Complete CLI reference with all commands and options |
| [Architecture](docs/ARCHITECTURE.md) | System architecture, data flow, and design decisions |
| [Network Analysis](docs/NETWORK_ANALYSIS.md) | Network capture, alerts, and CLI workflows |
| [Commands Reference](COMMANDS_REFERENCE.md) | AI chat actions, CLI commands, and IPC catalog |
| [Contributing](CONTRIBUTING.md) | Development setup, PR guidelines, and conventions |
| [Changelog](CHANGELOG.md) | Version history |
| [Security](SECURITY.md) | Security policy and vulnerability reporting |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Desktop App                     │
│  ┌──────────────────────┐  ┌──────────────────────────┐ │
│  │   Svelte 5 Frontend  │  │   Rust Backend (Tauri)   │ │
│  │  ┌────────────────┐  │  │  ┌──────────────────┐   │ │
│  │  │ Stores/State   │◄─┼──┼─►│ IPC Commands     │   │ │
│  │  │ Components     │  │  │  │ Automations      │   │ │
│  │  │ Virtual Scroll │  │  │  │ Plugins (Lua)    │   │ │
│  │  └────────────────┘  │  │  └────────┬─────────┘   │ │
│  └──────────────────────┘  └───────────┼─────────────┘ │
└────────────────────────────────────────┼───────────────┘
                                         │
┌────────────────────────────────────────▼───────────────┐
│                   macmon_core (Rust)                    │
│  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌────────────┐  │
│  │ metrics │ │ network │ │ security │ │     ai     │  │
│  │ watcher │ │ analysis│ │ audit    │ │ rules_eng. │  │
│  │ killer  │ │ alerts  │ │ crypto   │ │ browser    │  │
│  └─────────┘ └─────────┘ └──────────┘ └────────────┘  │
└────────────────────────────────────────────────────────┘
```

Four Cargo crates: `core` (engine), `cli` (terminal), `tui` (interactive), `desktop` (Tauri).

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full architecture document.

## License

[MIT](LICENSE)

## Support and Sponsorship

OmniMon is open-source software under the MIT license. Free to compile from source for personal use. To support development or get pre-built installers and premium support:

💖 **[Sponsor OmniMon on GitHub Sponsors](https://github.com/sponsors/chochy2001)**

---

# OmniMon (Español)

OmniMon es un monitor de sistema multiplataforma de nueva generación construido con Rust y Svelte. Proporciona telemetría de procesos en tiempo real, optimización con IA, análisis de seguridad de red y gestión de pestañas del navegador — todo desde una app de escritorio, CLI o TUI.

## Características

- **Monitoreo en tiempo real** — CPU, memoria, disco, red y consumo energético por proceso con refresco de 2 segundos
- **Optimización con IA** — Soporte multi-proveedor (OpenAI, Anthropic, Gemini, OpenRouter, Ollama) con tool calling para gestión automática
- **Inteligencia de red** — Captura nativa (libpcap/eBPF/WinDivert), análisis de conexiones, DNS, GeoIP y correlación MITRE ATT&CK
- **Control de pestañas** — Listar, cerrar y enfocar pestañas en Chrome, Safari, Brave, Edge, Arc y Firefox
- **Seguridad y cumplimiento** — Auditoría CVE, heartbeats NIST SP 800-53, trails de auditoría cifrados, firma Ed25519
- **Perfiles de usuario** — Presets configurables (minimal/balanced/power) con layouts de dashboard, intervalos de refresco y procesos favoritos
- **Automatizaciones SRE** — Reglas definidas por el usuario para alertas y acciones automáticas
- **Sistema de plugins** — Extiende el monitoreo con plugins Lua (sandboxed)
- **Tres interfaces** — GUI de escritorio, CLI e interfaz TUI interactiva

## Inicio Rápido

### macOS (Homebrew)

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon
```

### Windows

Descarga el instalador `.msi` desde el [último release](https://github.com/chochy2001/omnimon/releases/latest).

### Linux

```bash
# Debian/Ubuntu
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash

# O descarga .deb / .AppImage / .rpm desde el último release
```

### Compilar desde el Código Fuente

```bash
git clone https://github.com/chochy2001/omnimon.git
cd omnimon/v4
./setup-dev.sh
make dev
```

## Uso del CLI

```bash
omnimon status                                    # Vista general del sistema
omnimon optimize --ai anthropic --target browsers # Optimización con IA
omnimon network --top                             # Throughput de red
omnimon chat --ai openai "¿Qué está usando más RAM?"
omnimon security-scan                             # Auditoría de seguridad
omnimon doctor                                    # Diagnóstico del sistema
```

## Proveedores de IA

| Proveedor | Modelo (default) | Requiere API Key |
|-----------|-----------------|-----------------|
| OpenAI | gpt-4o-mini | Sí |
| Anthropic | claude-haiku-4-5 | Sí |
| Gemini | gemini-2.0-flash | Sí |
| OpenRouter | llama-3.2-3b (gratis) | Sí |
| Ollama | llama3.2 (local) | No |

Guarda claves de forma segura: `omnimon apikey --ai <proveedor> <clave>` (almacenado en Keyring del OS).

## Documentación

| Documento | Descripción |
|-----------|-------------|
| [Manual CLI](docs/CLI_MANUAL.md) | Referencia completa del CLI |
| [Arquitectura](docs/ARCHITECTURE.md) | Arquitectura del sistema y flujo de datos |
| [Análisis de Red](docs/NETWORK_ANALYSIS.md) | Captura de red, alertas y workflows |
| [Referencia de Comandos](COMMANDS_REFERENCE.md) | Acciones AI, comandos CLI y catálogo IPC |
| [Contribuir](CONTRIBUTING.md) | Setup de desarrollo y convenciones |

## Licencia

[MIT](LICENSE)

## Apoyo y Patrocinio

OmniMon es software de código abierto bajo licencia MIT. Gratis para compilar desde la fuente. Para apoyar el desarrollo u obtener instaladores pre-compilados y soporte premium:

💖 **[Patrocina OmniMon en GitHub Sponsors](https://github.com/sponsors/chochy2001)**
