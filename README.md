# OmniMon v4.0.0 🚀

[![CI/CD](https://img.shields.io/badge/build-passing-success)](#) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_+_Svelte-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#)

OmniMon es un monitor de sistema y navegador de próxima generación, reescrito desde cero en un monorepo moderno. Sustituye la antigua arquitectura de AppKit/Bash por un núcleo nativo hiper-optimizado y una interfaz reactiva con cero fugas de memoria.

## 🏗 Arquitectura

El proyecto adopta un enfoque modular estricto separando el backend nativo de la capa de presentación, comunicados a través del bus IPC de Tauri:

* **Core Nativo (`v4/crates/core`):** Escrito en Rust. Utiliza `sysinfo` para telemetría a nivel de hardware, el protocolo CDP (Chrome DevTools Protocol) para el análisis granular de pestañas de navegador, y llamadas FFI directas a Win32/libc para operaciones de bajo nivel del SO.
* **Capa de Presentación (`v4/apps/desktop`):** Interfaz compacta construida con Svelte y TypeScript sobre Tauri. Garantiza un footprint de memoria mínimo y un ciclo de vida de componentes estrictamente controlado.
* **CLI & Herramientas (`v4/crates/cli`):** Interfaz de terminal de alto rendimiento para control headless y automatización de servidores.

## ✨ Características Principales

* **Smart Optimize (Flujo de IA):** Resolución predictiva y optimización de recursos impulsada por IA. Soporte integrado para los proveedores líderes (OpenAI, Anthropic, OpenRouter).
* **Seguridad Multiplataforma Integrada (Keychain Nativo):** Las credenciales y claves de API *nunca* se almacenan en texto plano. OmniMon delega el almacenamiento al sistema nativo (macOS Keychain, Windows Credential Manager, Linux Secret Service).
* **Blocklists Seguras:** Listas de bloqueo dinámicas e inmutables por sistema operativo que previenen la terminación accidental de procesos críticos (ej. `smss.exe` en Windows o `launchd` en macOS).
* **Paridad de Características:** Experiencia consistente sin importar la plataforma subyacente (.exe, .dmg, .deb).

## 🚀 Quick Start (Instalación en un paso)

Para entornos de desarrollo, hemos abstraído la configuración de dependencias de Rust, Node y dependencias del sistema operativo en un único script.

```bash
curl -fsSL https://raw.githubusercontent.com/omnimon/omnimon/main/setup-dev.sh | bash
```

## 💻 Uso por CLI (macmon)

Para integraciones CI o uso en terminal, el binario `macmon` expone toda la funcionalidad del core:

```bash
macmon --help
macmon optimize --ai anthropic --target browsers
macmon status --format json
```
