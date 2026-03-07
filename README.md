# OmniMon v4

[![CI/CD](https://github.com/chochy2001/omnimon/actions/workflows/omnimon-ci.yml/badge.svg)](https://github.com/chochy2001/omnimon/actions) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_+_Svelte-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#)

<img width="1540" height="961" alt="image" src="https://github.com/user-attachments/assets/686dcab2-46c3-4c8d-9d03-ab9b2dd8005e" />


*Scroll down for Spanish / Desplázate hacia abajo para Español.*

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

## Support and Sponsorship (Funding)

OmniMon is open-source software under the MIT license. It is completely free to compile from source for your own use. However, if you wish to support the project's development or prefer the convenience of pre-packaged installers and premium support, we invite you to become a sponsor:

💖 **[Sponsor OmniMon on GitHub Sponsors](https://github.com/sponsors/chochy2001)**

Sponsors get access to pre-built premium installers (.exe, .dmg, .deb) and prioritized support, which helps keep this project sustainable long-term.

---

# OmniMon v4 (Español)

OmniMon es un monitor de sistema de próxima generación reescrito desde cero en un monorepositorio moderno. Reemplaza la antigua arquitectura de AppKit/Bash con un núcleo nativo hiperoptimizado y una interfaz reactiva sin fugas de memoria.

## Arquitectura

El proyecto sigue un enfoque modular estricto, separando el backend nativo de la capa de presentación, comunicándose a través del bus IPC de Tauri:

* **Núcleo Nativo (`v4/crates/core`):** Escrito en Rust. Utiliza `sysinfo` para la telemetría a nivel de hardware, el CDP (Protocolo de Herramientas de Desarrollo de Chrome) para el análisis granular de pestañas del navegador y llamadas directas FFI a Win32/libc para operaciones de sistema operativo de bajo nivel.
* **Capa de Presentación (`v4/apps/desktop`):** Interfaz compacta construida con Svelte 5 y TypeScript sobre Tauri. Garantiza una huella de memoria mínima y un ciclo de vida de componentes estrictamente controlado. El desplazamiento virtual renderiza más de 2000 procesos a 60 FPS.
* **CLI y Herramientas (`v4/crates/cli`):** Interfaz de terminal de alto rendimiento para control sin cabeza (headless) y automatización de servidores.

## Características Clave

* **Optimización Inteligente (Flujo de IA):** Resolución predictiva y optimización de recursos impulsada por IA. Soporte integrado para los principales proveedores (OpenAI, Anthropic, OpenRouter).
* **Seguridad Multiplataforma Integrada (Llavero Nativo):** Las credenciales y claves API *nunca* se almacenan en texto plano. OmniMon delega el almacenamiento al sistema nativo (Llavero de macOS, Administrador de Credenciales de Windows, Servicio Secreto de Linux).
* **Listas de Bloqueo Seguras:** Listas de bloqueo dinámicas e inmutables por sistema operativo que evitan la finalización accidental de procesos críticos (por ejemplo, `smss.exe` en Windows o `launchd` en macOS).
* **Paridad de Funciones:** Experiencia consistente independientemente de la plataforma subyacente (.exe, .dmg, .deb).

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
curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/install-web.sh | bash

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
## Core Modules Documentation

Autogenerated from Rust source code (`v4/crates/core/src`):

### `core::ai`
*(No module-level documentation provided)*

### `core::audit`
*(No module-level documentation provided)*

### `core::audit_trail`
*(No module-level documentation provided)*

### `core::browser`
*(No module-level documentation provided)*

### `core::crypto`
*(No module-level documentation provided)*

### `core::killer`
*(No module-level documentation provided)*

### `core::lib`
*(No module-level documentation provided)*

### `core::metrics`
*(No module-level documentation provided)*

### `core::network`
*(No module-level documentation provided)*

### `core::os_native`
*(No module-level documentation provided)*

### `core::security`
*(No module-level documentation provided)*

### `core::watcher`
*(No module-level documentation provided)*

