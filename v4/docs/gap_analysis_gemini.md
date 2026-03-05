# Gap Analysis: macmon v3 vs v4 (Multiplataforma)

## Resumen Ejecutivo
Este documento analiza las brechas funcionales entre la arquitectura antigua (v3, basada en Bash y AppKit/Swift) y la nueva arquitectura multiplataforma (v4, basada en Rust, Tauri y Svelte), identificando regresiones y proponiendo soluciones técnicas para alcanzar y superar la paridad de características.

---

## 1. Introspección de Navegadores (Gestión de Pestañas)
### Análisis de la Brecha
La v3 utilizaba AppleScript u OSAScript para leer las pestañas abiertas en Safari y Chrome, lo que permitía a los usuarios identificar qué sitios web consumían recursos y cerrarlos específicamente. La v4, al depender de `sysinfo` genérico en Rust, solo observa los procesos a nivel de sistema operativo (ej. `Google Chrome Helper`), perdiendo el contexto de las URLs y títulos.

### Soluciones Propuestas Multiplataforma
- **macOS**: Utilizar el crate `accessibility-sys` o puente `objc2` para invocar las APIs de accesibilidad de macOS (AXUIElement), tal como lo hacen herramientas similares, para leer la jerarquía de ventanas y pestañas del navegador sin requerir extensiones.
- **Windows**: Implementar UI Automation (UIA) a través del crate `windows` (específicamente `Windows.UI.UIAutomation`) para inspeccionar el árbol de elementos de Chrome/Edge y extraer los nombres de las pestañas.
- **Linux**: Utilizar `at-spi2-core` (Accesibility Toolkit) a través de D-Bus para leer los árboles de accesibilidad de navegadores compatibles (Chrome/Firefox).
- **Alternativa General (CDP)**: Como alternativa más robusta para navegadores Chromium, se puede explorar el uso del Chrome DevTools Protocol (CDP), aunque esto requiere iniciar el navegador con flags de depuración habilitados, lo cual impacta la UX.

---

## 2. Integración de Inteligencia Artificial (Smart Optimize)
### Análisis de la Brecha
La v3 implementaba un flujo "Human-in-the-Loop" donde se consultaba a un LLM (OpenRouter/OpenAI) para analizar la lista de procesos y recomendar cuáles finalizar de manera segura, almacenando la API Key en el Keychain del sistema. La v4 actual carece de esta integración y del flujo de autorización de la clave.

### Soluciones Propuestas Multiplataforma
- **Almacenamiento Seguro (Keychain / Credential Manager)**: Implementar el crate `keyring` en `v4/crates/core`, el cual abstrae nativamente el Keychain de macOS, el Credential Manager de Windows y Secret Service (GNOME Keyring/KWallet) en Linux, permitiendo guardar la API Key de forma segura y unificada.
- **Cliente HTTP / LLM**: Utilizar `reqwest` en el core de Rust para construir el cliente que se comunique con OpenAI/OpenRouter, exponiendo un *Tauri Command* (`invoke('smart_optimize')`) para que Svelte consuma el análisis.
- **UI de Autorización**: Crear un modal en Svelte que se dispare cuando el usuario intente usar *Smart Optimize* por primera vez y no haya una clave configurada en el anillo de claves (keyring) del SO.

---

## 3. Gestión Visual y UX
### Análisis de la Brecha
La v3 en AppKit (ej. `ProcessPicker.swift`) contaba con un panel de detalles avanzado (`Cmd+I`) que mostraba información profunda (rutas, ancestros, uso acumulado) y agrupaba visualmente los procesos hijos bajo el proceso padre. La UI actual de la v4 muestra una lista plana genérica.

### Soluciones Propuestas Multiplataforma
- **Agrupación de Procesos (Árboles)**: En el backend (Rust), usar `sysinfo` para reconstruir la jerarquía de procesos (Padre -> Hijos) utilizando los PIDs y PPIDs. Serializar esta estructura arbórea hacia el frontend.
- **Componentes Svelte**: Implementar un componente de lista anidada (ej. `<TreeView>`) en Svelte para agrupar procesos bajo aplicaciones principales (ej. agrupar todos los `Chrome Helper` bajo `Google Chrome`).
- **Panel de Detalles (Cmd+I)**: Desarrollar un componente de panel lateral o modal en Svelte. Escuchar atajos de teclado globales o locales (usando Tauri Global Shortcut plugin o manejadores de teclado de Svelte) para expandir los detalles del proceso seleccionado.

---

## 4. Hardening y Blocklist (Seguridad)
### Análisis de la Brecha
La v3 incorporaba listas estrictas (Blocklists / Safelists) para evitar que el usuario, o la IA, mataran procesos críticos del sistema (ej. `kernel_task`, `launchd`, `WindowServer`), evitando "kernel panics" o cuelgues del SO. La v4 necesita replicar esta protección a nivel multiplataforma, ya que un error de matar un proceso crítico de Windows (`svchost.exe`, `csrss.exe`) resultará en un "Blue Screen".

### Soluciones Propuestas Multiplataforma
- **Safelists Multiplataforma**: Definir un módulo estático en `v4/crates/core/src/watcher.rs` (o un nuevo `safelist.rs`) que contenga conjuntos de procesos bloqueados para cada SO usando `cfg` macros:
  - `#[cfg(target_os = "macos")]`: `kernel_task`, `launchd`, `WindowServer`, etc.
  - `#[cfg(target_os = "windows")]`: `System`, `smss.exe`, `csrss.exe`, `wininit.exe`, `services.exe`, `lsass.exe`, `svchost.exe`, etc.
  - `#[cfg(target_os = "linux")]`: `systemd`, `init`, `kthreadd`, `Xorg`, `wayland`, etc.
- **Validación Estricta en Core**: Modificar la función `kill` del core de Rust para que retorne un `Error::ProcessProtected` si el PID objetivo está asociado a un ejecutable listado en la Safelist.

---

## Tabla Comparativa Exhaustiva

| Feature | Estado v3 (Bash/Swift) | Estado v4 (Tauri/Rust/Svelte) | Solución Propuesta Multiplataforma (Rust/Tauri) |
| :--- | :--- | :--- | :--- |
| **Monitoreo de Procesos** | Básico (`ps`, `top`) | Activo (`sysinfo` genérico) | Mejorar el polling en Rust para extraer íconos nativos y agrupar por jerarquía de PPID. |
| **Introspección de Pestañas** | Sí, vía AppleScript (macOS solo) | No, solo procesos crudos | **macOS**: `accessibility-sys` <br> **Windows**: UIAutomation (`windows` crate) <br> **Linux**: `at-spi2-core` (D-Bus) |
| **Smart Optimize (IA)** | Sí (OpenRouter, interactivo) | No implementado | Integrar `reqwest` en Core + endpoints en Tauri. Retener el enfoque "Human-in-the-Loop". |
| **Gestión de API Keys** | macOS Keychain | Inexistente | Usar crate `keyring` para acceso seguro al manejador de credenciales nativo del SO. |
| **Agrupación Visual** | Árboles jerárquicos (Padre/Hijo) | Lista plana básica | Reconstruir el AST de procesos en Rust y renderizar un `<TreeView>` en Svelte. |
| **Panel de Detalles (Cmd+I)** | Activo (AppKit views) | No implementado | Crear componente lateral en Svelte y vincular atajos de teclado locales. |
| **Safelists del SO** | Activo (macOS only limits) | Falta hardening cruzado | Implementar `safelist.rs` en el Core de Rust con listas estrictas divididas por macro `#[cfg(target_os)]`. |
| **Bloqueo a nivel Kernel** | Prevenía matar `kernel_task` | Vulnerable en Win/Linux | El `kill_process` en Rust debe verificar contra el blocklist del SO antes de emitir la señal SIGKILL. |

---
*Generado por Auditoría de IA (Gemini)*