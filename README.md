# OmniMon v4

[![CI/CD](https://github.com/chochy2001/omnimon/actions/workflows/omnimon-ci.yml/badge.svg)](https://github.com/chochy2001/omnimon/actions) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_+_Svelte-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#) [![Sponsor](https://img.shields.io/badge/Sponsor-💖-ff69b4)](https://github.com/sponsors/chochy2001)

<img width="1540" height="961" alt="image" src="https://github.com/user-attachments/assets/686dcab2-46c3-4c8d-9d03-ab9b2dd8005e" />


*Scroll down for Spanish / Desplázate hacia abajo para Español.*

OmniMon is a next-generation system monitor rewritten from scratch in a modern monorepo. It replaces the legacy AppKit/Bash architecture with a hyper-optimized native core and a reactive UI.

## What's New in v4.6

* **Native network telemetry (DPI Lite):** platform collectors for **libpcap** (macOS), **WinDivert** (Windows), and **eBPF/aya** (Linux).
* **Per-process network context:** throughput by PID, recent connections (IP/port/protocol), and backend health telemetry.
* **MITRE + NIST security pipeline:** behavioral mapping (e.g. T1055/T1043/T1571), plus encrypted NIST 800-53 heartbeat snapshots.
* **AI Rules Engine:** versioned JSON contract for dynamic alert rules (GeoIP, CIDR, port/protocol, process memory threshold).
* **Real-time IPC alerts:** backend emits `security-alert` events via Tauri to power reactive UI cards/notifications.

## Architecture

The project follows a strict modular approach, separating the native backend from the presentation layer, communicating through Tauri's IPC bus:

* **Native Core (`v4/crates/core`):** Written in Rust. Uses `sysinfo` for process/system telemetry, native network backends (libpcap/WinDivert/eBPF), CDP for browser analysis, and direct FFI calls to Win32/libc for low-level OS operations.
* **Presentation Layer (`v4/apps/desktop`):** Compact interface built with Svelte 5 and TypeScript on Tauri. Guarantees minimal memory footprint and a strictly controlled component lifecycle. Virtual scroll renders 2000+ processes at 60 FPS.
* **CLI & Tools (`v4/crates/cli`):** High-performance terminal interface for headless control and server automation.

## Key Features

* **Smart Optimize (AI Flow):** Predictive resolution and AI-powered resource optimization. Built-in support for leading providers (OpenAI, Anthropic, OpenRouter).
* **Integrated Cross-Platform Security (Native Keychain):** Credentials and API keys are *never* stored in plain text. OmniMon delegates storage to the native system (macOS Keychain, Windows Credential Manager, Linux Secret Service).
* **Secure Blocklists:** Dynamic and immutable per-OS block lists that prevent accidental termination of critical processes (e.g. `smss.exe` on Windows or `launchd` on macOS).
* **Feature Parity:** Consistent experience across macOS/Windows/Linux, including native telemetry backends and shared AI-rules contract.

## AI Rules JSON Contract (v1)

The backend accepts a versioned payload through Tauri command `apply_ai_rules(payload: String)`.

* **Schema endpoint (IPC):** `get_ai_rules_schema()`
* **Canonical example file:** `v4/crates/core/AI_RULES_SCHEMA.v1.json`
* **Supported rule kinds:** `process_country`, `process_ip`, `process_cidr`, `process_port`, `process_memory`

Example:

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "proc-mem-004",
      "name": "Alert if node > 1GB",
      "enabled": true,
      "kind": "process_memory",
      "process_contains": "node",
      "country_code": null,
      "destination_ip": null,
      "destination_cidr": null,
      "destination_port": null,
      "protocol": "any",
      "process_memory_mb_gt": 1024,
      "mitre_technique_id": "T1499"
    }
  ]
}
```

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
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash

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

## Support and Sponsorship (Funding)

OmniMon is open-source software under the MIT license. It is completely free to compile from source for your own use. However, if you wish to support the project's development or prefer the convenience of pre-packaged installers and premium support, we invite you to become a sponsor:

💖 **[Sponsor OmniMon on GitHub Sponsors](https://github.com/sponsors/chochy2001)**

Sponsors get access to pre-built premium installers (.exe, .dmg, .deb) and prioritized support, which helps keep this project sustainable long-term.

---

# OmniMon v4 (Español)

OmniMon es un monitor de sistema de próxima generación reescrito desde cero en un monorepositorio moderno. Reemplaza la antigua arquitectura de AppKit/Bash con un núcleo nativo hiperoptimizado y una interfaz reactiva.

## Novedades en v4.6

* **Telemetría de red nativa (DPI Lite):** colectores por plataforma con **libpcap** (macOS), **WinDivert** (Windows) y **eBPF/aya** (Linux).
* **Contexto de red por proceso:** throughput por PID, conexiones recientes (IP/puerto/protocolo) y salud del backend de captura.
* **Pipeline de seguridad MITRE + NIST:** mapeo de comportamiento (ej. T1055/T1043/T1571) y snapshots cifrados de heartbeat NIST 800-53.
* **Motor de reglas IA:** contrato JSON versionado para alertas dinámicas (GeoIP, CIDR, puerto/protocolo, umbral de memoria).
* **Alertas IPC en tiempo real:** emisión de evento `security-alert` vía Tauri para UI reactiva.

## Arquitectura

El proyecto sigue un enfoque modular estricto, separando el backend nativo de la capa de presentación, comunicándose a través del bus IPC de Tauri:

* **Núcleo Nativo (`v4/crates/core`):** Escrito en Rust. Utiliza `sysinfo` para telemetría de sistema/procesos, backends de red nativos (libpcap/WinDivert/eBPF), CDP para análisis de pestañas y FFI Win32/libc para operaciones de bajo nivel.
* **Capa de Presentación (`v4/apps/desktop`):** Interfaz compacta construida con Svelte 5 y TypeScript sobre Tauri. Garantiza una huella de memoria mínima y un ciclo de vida de componentes estrictamente controlado. El desplazamiento virtual renderiza más de 2000 procesos a 60 FPS.
* **CLI y Herramientas (`v4/crates/cli`):** Interfaz de terminal de alto rendimiento para control sin cabeza (headless) y automatización de servidores.

## Características Clave

* **Optimización Inteligente (Flujo de IA):** Resolución predictiva y optimización de recursos impulsada por IA. Soporte integrado para los principales proveedores (OpenAI, Anthropic, OpenRouter).
* **Seguridad Multiplataforma Integrada (Llavero Nativo):** Las credenciales y claves API *nunca* se almacenan en texto plano. OmniMon delega el almacenamiento al sistema nativo (Llavero de macOS, Administrador de Credenciales de Windows, Servicio Secreto de Linux).
* **Listas de Bloqueo Seguras:** Listas de bloqueo dinámicas e inmutables por sistema operativo que evitan la finalización accidental de procesos críticos (por ejemplo, `smss.exe` en Windows o `launchd` en macOS).
* **Paridad de Funciones:** Experiencia consistente en macOS/Windows/Linux, incluyendo backends nativos y contrato de reglas IA compartido.

## Contrato JSON de Reglas IA (v1)

El backend acepta un payload versionado mediante el comando Tauri `apply_ai_rules(payload: String)`.

* **Endpoint IPC del schema:** `get_ai_rules_schema()`
* **Archivo canónico de ejemplo:** `v4/crates/core/AI_RULES_SCHEMA.v1.json`
* **Tipos soportados:** `process_country`, `process_ip`, `process_cidr`, `process_port`, `process_memory`

## Uso Rápido

### macOS (Homebrew)

```bash
brew tap chochy2001/omnimon
brew install --cask omnimon
```

La aplicación se inicia automáticamente después de la instalación. También puedes encontrarla en **Spotlight** (`Cmd + Space` → "OmniMon") o en `/Applications/OmniMon.app`.

### Windows

Descarga el instalador `.msi` desde el [último lanzamiento](https://github.com/chochy2001/omnimon/releases/latest).

### Linux

```bash
# Debian/Ubuntu — instalación en una línea
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash

# O descarga el .deb / .AppImage desde el último lanzamiento
```

### Compilar desde el Código Fuente

```bash
git clone https://github.com/chochy2001/omnimon.git
cd omnimon/v4
./setup-dev.sh
make dev
```

## Uso del CLI (Compilar desde el Código Fuente)

```bash
cd v4
cargo run -p cli -- --help
cargo run -p cli -- optimize --ai anthropic --target browsers
cargo run -p cli -- status --format json
```

## Licencia

[MIT](LICENSE)

## Apoyo y Patrocinio (Sponsors)

OmniMon es de código abierto (Open Source) bajo la licencia MIT. Es completamente gratis compilarlo desde la fuente para tu propio uso. Sin embargo, si deseas apoyar el desarrollo del proyecto o prefieres la comodidad de instaladores preempaquetados y soporte premium, te invitamos a convertirte en patrocinador:

💖 **[Patrocina OmniMon en GitHub Sponsors](https://github.com/sponsors/chochy2001)**

Los patrocinadores obtienen acceso a instaladores premium precompilados (.exe, .dmg, .deb) y soporte prioritario, lo que ayuda a mantener este proyecto sostenible a largo plazo.
## Core Modules (v4)

* `v4/crates/core/src/network.rs` - native network collectors + per-process throughput/events.
* `v4/crates/core/src/security.rs` - behavior mapping to MITRE ATT&CK techniques.
* `v4/crates/core/src/audit.rs` - CVE matching + NIST heartbeat generation/persistence.
* `v4/crates/core/src/crypto.rs` - AES-256-GCM encryption/decryption helpers.
* `v4/crates/core/src/rules_engine.rs` - AI rules JSON contract + dynamic rule evaluation.
* `v4/crates/core/src/watcher.rs` - cached system snapshot + network/security state exposure.
## Core Modules Documentation

Autogenerated from Rust source code (`v4/crates/core/src`):

### `core::ai`
Artificial Intelligence integration module. Handles communication with various LLM providers (OpenAI, Anthropic, Gemini, OpenRouter) for predictive system optimization and context analysis.

### `core::audit`
Security auditing and NIST compliance module. Compares active processes against CVE databases and generates encrypted security heartbeats.

### `core::audit_trail`
Audit trail logging. Maintains a secure, tamper-evident record of all critical actions taken by the system or the user.

### `core::browser`
Browser integration. Uses the Chrome DevTools Protocol (CDP) to track, focus, and manage individual browser tabs across Chrome, Safari, Brave, Edge, and Arc.

### `core::crypto`
Cryptographic utilities. Provides AES-256-GCM encryption and secure payload handling for sensitive security reports and audit logs.

### `core::killer`
Process management and termination. Implements safe process killing with strict, immutable OS-specific blocklists to prevent accidental termination of critical system services.

### `core::lib`
OmniMon Core Library. This crate contains all the high-performance native logic, including system telemetry, network capture, and AI processing, completely decoupled from the UI.

### `core::metrics`
System telemetry and metrics collection. Gathers real-time data on CPU, memory, swap, and identifies the top resource-consuming processes.

### `core::network`
Cross-platform network traffic capture. Utilizes native drivers (libpcap on macOS, WinDivert on Windows, eBPF on Linux) to monitor connections and correlate them with PIDs.

### `core::os_native`
Low-level Operating System FFI bindings. Handles direct interactions with Windows API, macOS frameworks, and Linux syscalls.

### `core::rules_engine`
Dynamic AI Rules Engine. Processes JSON-based rule sets generated by the AI to evaluate real-time network and system events, enabling dynamic MITRE ATT&CK detection.

### `core::security`
Core security abstractions. Manages secure credential storage using native OS keyrings (Keychain, Credential Manager, Secret Service).

### `core::watcher`
Background monitoring daemon. Periodically aggregates system metrics, network flows, and dynamically evaluates AI-driven security rules.

