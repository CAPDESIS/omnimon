# OmniMon v4

*Lea esto en otros idiomas: [English](README.md)*

[![CI/CD](https://github.com/chochy2001/omnimon/actions/workflows/omnimon-ci.yml/badge.svg)](https://github.com/chochy2001/omnimon/actions) [![Rust Core](https://img.shields.io/badge/core-Rust_v1.75+-orange)](#) [![Tauri UI](https://img.shields.io/badge/ui-Tauri_+_Svelte-blue)](#) [![Platform](https://img.shields.io/badge/platform-macOS_|_Windows_|_Linux-lightgray)](#)

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

## Apoyo y Patrocinio (Sponsors)

OmniMon es de código abierto (Open Source) bajo la licencia MIT. Es completamente gratis compilarlo desde la fuente para tu propio uso. Sin embargo, si deseas apoyar el desarrollo del proyecto o prefieres la comodidad de instaladores preempaquetados y soporte premium, te invitamos a convertirte en patrocinador:

💖 **[Patrocina OmniMon en GitHub Sponsors](https://github.com/sponsors/chochy2001)**

Los patrocinadores obtienen acceso a instaladores premium precompilados (.exe, .dmg, .deb) y soporte prioritario, lo que ayuda a mantener este proyecto sostenible a largo plazo.
