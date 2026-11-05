# OmniMon Future Roadmap & Pending Enhancements (v4.x)

This document outlines the strategic roadmap and pending technical debt for future releases of OmniMon. While the v4.0.0 core is stable and production-ready, these enhancements will further elevate the enterprise capabilities of the application.

## 1. Graphical Interface (GUI) for CrabNebula Cloud
*   **Context:** Cloud authentication and security report syncing (`omnimon cloud sync`) are currently CLI-exclusive.
*   **Task:** Implement a new panel in the Desktop application's Settings view (Svelte/Tauri).
*   **Requirements:**
    *   An input field to securely paste the `CN_API_KEY`.
    *   Visual status indicator showing whether the Keyring validation was successful.
    *   A manual "Sync Reports" button triggering the Tauri IPC command to invoke the Rust backend upload mechanism.
*   **Status (2026-11-05):** DONE in the app. The Settings panel (`v4/apps/desktop/src/components/CloudSync.svelte`) calls IPC `sync_cloud` (`v4/apps/desktop/src-tauri/src/lib.rs`, registered in `generate_handler!`). The real CrabNebula endpoint and contract remain owner-unverified. The 2026-10-14 note that called this a `syncNotImplemented` stub is historical.

## 2. Temporal Correlation in AiConfigBridge (Rules Engine)
*   **Status (2026-10-14):** DONE — `rules_engine.rs` has `TemporalCorrelation` + schema. No longer pending.
*   **Context:** The current AI Rules Engine successfully blocks or alerts on geographic IP connections and standard process behavior.
*   **Task:** Upgrade the engine to support time-based correlation rules.
*   **Requirements:**
    *   Implement stateful memory in `rules_engine.rs` to track sequential events.
    *   **Example Rule:** "Trigger CRITICAL alert IF process reads memory from Chrome AND connects to an external IP WITHIN 5 seconds."
*   **Status (2026-10-14):** DONE — `TemporalCorrelation{rule_id, within_seconds}` with stateful match memory in `v4/crates/core/src/rules_engine.rs:31,356` plus test coverage.

## 3. Cross-Platform End-to-End (E2E) Testing
*   **Status (2026-11-05):** the Linux job `E2E (Playwright, desktop)` in `.github/workflows/omnimon-ci.yml` installs Chromium and runs `bun run test:e2e` on pull request, push, and schedule. Run `41895931744` on `3471010` concluded success. macOS and Windows UI runs are not in that matrix (`workflow_dispatch` only, and those platforms are marked experimental). The 2026-10-14 note that said no CI job ran Playwright is historical.
*   **Context:** The Rust core has strong unit and integration coverage (>85%), but the UI layer relies on manual testing.
*   **Task:** Introduce an E2E testing framework for the Tauri application.
*   **Requirements:**
    *   Integrate **WebDriverIO** or **Playwright** configured for Tauri.
    *   Add a CI/CD job that spins up a virtual display (Xvfb on Linux, standard runners on Mac/Win) to click through the UI and verify that virtual scrolling and process killing work visually.
*   **Status (2026-11-05):** same as the line above. Chromium is installed in that job. Do not read the 2026-10-14 "browsers not installed" clause as current.

## 4. Apple Notary Service Integration (macOS Gatekeeper)
*   **Context:** The current macOS `.dmg` is signed with `codesign`, but strict Gatekeeper policies require explicit notarization from Apple to avoid the "Unidentified Developer" warning.
*   **Task:** Automate macOS Notarization in GitHub Actions.
*   **Status (2026-10-14):** still pending — the pipeline (`.github/workflows/omnimon-ci.yml`) only does Ed25519/SHA-256 artifact signing plus Tauri updater signing; no `notarytool` step exists yet. Apple Developer secrets are owner-provisioned.
*   **Requirements:**
    *   Add `xcrun notarytool submit` to the `.github/workflows/omnimon-ci.yml` pipeline.
    *   Wait for Apple's servers to validate the binary, and then staple the ticket using `xcrun stapler staple`.

---

# Hoja de Ruta y Mejoras Pendientes de OmniMon (Español)

Este documento describe la hoja de ruta estratégica y la deuda técnica pendiente para futuras versiones de OmniMon. Aunque el núcleo v4.0.0 es estable y está listo para producción, estas mejoras elevarán aún más las capacidades empresariales de la aplicación.

## 1. Interfaz Gráfica (GUI) para CrabNebula Cloud
*   **Contexto:** La autenticación en la nube y la sincronización de reportes de seguridad (`omnimon cloud sync`) son actualmente exclusivas del CLI.
*   **Tarea:** Implementar un nuevo panel en la vista de Configuración (Settings) de la aplicación de escritorio (Svelte/Tauri).
*   **Requisitos:**
    *   Un campo de texto para pegar de forma segura la clave `CN_API_KEY`.
    *   Indicador visual de estado que muestre si la validación en el Keyring nativo fue exitosa.
    *   Un botón manual de "Sincronizar Reportes" que active el comando IPC de Tauri para invocar el mecanismo de subida en el backend de Rust.
*   **Estado (2026-11-05):** HECHO en la app. El panel (`CloudSync.svelte`) llama al IPC `sync_cloud` (`src-tauri/src/lib.rs`, registrado en `generate_handler!`). El endpoint y el contrato reales de CrabNebula siguen sin verificación del operador. La nota del 2026-10-14 sobre el stub `syncNotImplemented` es histórica.

## 2. Correlación Temporal en AiConfigBridge (Motor de Reglas)
*   **Estado (2026-10-14):** HECHO — `rules_engine.rs` tiene `TemporalCorrelation` + esquema. Ya no pendiente.
*   **Contexto:** El actual Motor de Reglas de IA bloquea o alerta con éxito sobre conexiones IP geográficas y comportamientos estándar de procesos.
*   **Tarea:** Actualizar el motor para soportar reglas de correlación basadas en el tiempo.
*   **Requisitos:**
    *   Implementar memoria de estado en `rules_engine.rs` para rastrear eventos secuenciales.
    *   **Regla de Ejemplo:** "Disparar alerta CRÍTICA SI un proceso lee la memoria de Chrome Y se conecta a una IP externa EN MENOS DE 5 segundos".
*   **Estado (2026-10-14):** HECHO — `TemporalCorrelation` con memoria de estado en `v4/crates/core/src/rules_engine.rs:31,356` más pruebas.

## 3. Pruebas End-to-End (E2E) Multiplataforma
*   **Estado (2026-11-05):** el job de Linux `E2E (Playwright, desktop)` en `.github/workflows/omnimon-ci.yml` instala Chromium y ejecuta `bun run test:e2e` en pull request, push y schedule. El run `41895931744` sobre `3471010` terminó en success. macOS y Windows no entran en esa matriz (solo `workflow_dispatch`, y el workflow los marca experimentales). La nota del 2026-10-14 que decía que CI no lo corría es histórica.
*   **Contexto:** El núcleo de Rust tiene una fuerte cobertura unitaria y de integración (>85%), pero la capa de UI depende de pruebas manuales.
*   **Tarea:** Introducir un framework de pruebas E2E para la aplicación Tauri.
*   **Requisitos:**
    *   Integrar **WebDriverIO** o **Playwright** configurado para Tauri.
    *   Añadir un job en CI/CD que levante una pantalla virtual (Xvfb en Linux, runners estándar en Mac/Win) para hacer clics en la UI y verificar visualmente que el scroll virtual y el cierre de procesos funcionan.
*   **Estado (2026-11-05):** igual que la línea de arriba. Ese job instala Chromium. La cláusula del 2026-10-14 no describe el pipeline actual.

## 4. Integración de Notarización de Apple (Gatekeeper en macOS)
*   **Contexto:** El `.dmg` actual de macOS está firmado con `codesign`, pero las políticas estrictas de Gatekeeper requieren notarización explícita de Apple para evitar la advertencia de "Desarrollador no identificado".
*   **Tarea:** Automatizar la notarización de macOS en GitHub Actions.
*   **Estado (2026-10-14):** aún pendiente — el pipeline (`.github/workflows/omnimon-ci.yml`) solo firma artefactos (Ed25519/SHA-256) y el updater de Tauri; no existe paso `notarytool`. Los secretos de Apple Developer los provisiona el operador.
*   **Requisitos:**
    *   Añadir `xcrun notarytool submit` al pipeline `.github/workflows/omnimon-ci.yml`.
    *   Esperar a que los servidores de Apple validen el binario y luego adjuntar el ticket usando `xcrun stapler staple`.
