# OmniMon Future Roadmap & Pending Enhancements (v4.x)

> Status 2026-10-13 (workspace 6.8.0): items 2 and 3 are DONE (see notes).
> Item 1 is PARTIAL (key UI exists, sync not wired). Only item 4 (Apple
> notarization) is still fully pending. Verified against
> `v4/crates/core/src/rules_engine.rs`, `v4/apps/desktop/src/components/CloudSync.svelte`,
> `v4/apps/desktop/e2e/`, and `.github/workflows/omnimon-ci.yml`.

This document outlines the strategic roadmap and pending technical debt for future releases of OmniMon. While the v4.0.0 core is stable and production-ready, these enhancements will further elevate the enterprise capabilities of the application.

## 1. Graphical Interface (GUI) for CrabNebula Cloud — PARTIAL
*   **Context:** Cloud authentication (`omnimon auth login`) and security report syncing (`omnimon cloud sync`) were CLI-exclusive.
*   **Done:** `CloudSync.svelte` renders the API-key input and persists it via IPC `save_cloud_key` / `get_cloud_key`.
*   **Still pending:** the "Sync Reports" button only shows `cloudSync.syncNotImplemented`; no `cloud_sync_report` IPC exists yet.
*   **Task:** wire the sync button to a new Tauri IPC command invoking the Rust backend upload mechanism.

## 2. Temporal Correlation in AiConfigBridge (Rules Engine) — DONE
*   **Context:** The AI Rules Engine blocked or alerted on geographic IP connections and standard process behavior.
*   **Done:** `v4/crates/core/src/rules_engine.rs` implements `TemporalCorrelation { rule_id, within_seconds }` with stateful event memory and 60s pruning; the JSON schema (`get_ai_rules_schema`) documents it.
*   **Example Rule:** "Trigger CRITICAL alert IF process reads memory from Chrome AND connects to an external IP WITHIN 5 seconds."

## 3. Cross-Platform End-to-End (E2E) Testing — DONE
*   **Context:** The Rust core has strong unit and integration coverage, but the UI layer relied on manual testing.
*   **Done:** Playwright standalone (`v4/apps/desktop/e2e/`, `bun run test:e2e`): 5 suites (`app-loads`, `process-table`, `navigation`, `settings`, `ai-chat`) with Tauri IPC fixtures and helpers. (WebDriverIO was evaluated and dropped.)

## 4. Apple Notary Service Integration (macOS Gatekeeper) — PENDING
*   **Context:** The current macOS `.dmg` is signed with `codesign`, but strict Gatekeeper policies require explicit notarization from Apple to avoid the "Unidentified Developer" warning.
*   **Task:** Automate macOS Notarization in GitHub Actions.
*   **Requirements:**
    *   Add `xcrun notarytool submit` to the `.github/workflows/omnimon-ci.yml` release pipeline (the old `ci-cd.yml` name no longer exists).
    *   Wait for Apple's servers to validate the binary, and then staple the ticket using `xcrun stapler staple`.

---

# Hoja de Ruta y Mejoras Pendientes de OmniMon (Español)

> Estado 2026-10-13 (workspace 6.8.0): puntos 2 y 3 TERMINADOS. Punto 1
> PARCIAL (existe la UI de clave, falta cablear sync). Solo el punto 4
> (notarización Apple) sigue totalmente pendiente.

Este documento describe la hoja de ruta estratégica y la deuda técnica pendiente para futuras versiones de OmniMon. Aunque el núcleo v4.0.0 es estable y está listo para producción, estas mejoras elevarán aún más las capacidades empresariales de la aplicación.

## 1. Interfaz Gráfica (GUI) para CrabNebula Cloud — PARCIAL
*   **Contexto:** La autenticación en la nube y la sincronización de reportes de seguridad (`omnimon cloud sync`) eran exclusivas del CLI.
*   **Hecho:** `CloudSync.svelte` muestra el campo de clave y la persiste vía IPC `save_cloud_key` / `get_cloud_key`.
*   **Aún pendiente:** el botón "Sincronizar Reportes" solo muestra `cloudSync.syncNotImplemented`; no existe el IPC `cloud_sync_report`.
*   **Tarea:** cablear el botón a un nuevo comando IPC de Tauri que invoque el mecanismo de subida en el backend de Rust.

## 2. Correlación Temporal en AiConfigBridge (Motor de Reglas) — HECHO
*   **Contexto:** El Motor de Reglas de IA bloqueaba o alertaba sobre conexiones IP geográficas y comportamientos estándar de procesos.
*   **Hecho:** `v4/crates/core/src/rules_engine.rs` implementa `TemporalCorrelation { rule_id, within_seconds }` con memoria de estado y purga a 60s; el esquema JSON (`get_ai_rules_schema`) lo documenta.
*   **Regla de Ejemplo:** "Disparar alerta CRÍTICA SI un proceso lee la memoria de Chrome Y se conecta a una IP externa EN MENOS DE 5 segundos".

## 3. Pruebas End-to-End (E2E) Multiplataforma — HECHO
*   **Contexto:** El núcleo de Rust tiene fuerte cobertura unitaria y de integración, pero la capa de UI dependía de pruebas manuales.
*   **Hecho:** Playwright standalone (`v4/apps/desktop/e2e/`, `bun run test:e2e`): 5 suites (`app-loads`, `process-table`, `navigation`, `settings`, `ai-chat`) con fixtures IPC y helpers. (WebDriverIO se evaluó y se descartó.)

## 4. Integración de Notarización de Apple (Gatekeeper en macOS) — PENDIENTE
*   **Contexto:** El `.dmg` actual de macOS está firmado con `codesign`, pero las políticas estrictas de Gatekeeper requieren notarización explícita de Apple para evitar la advertencia de "Desarrollador no identificado".
*   **Tarea:** Automatizar la notarización de macOS en GitHub Actions.
*   **Requisitos:**
    *   Añadir `xcrun notarytool submit` al pipeline de release `.github/workflows/omnimon-ci.yml` (el nombre viejo `ci-cd.yml` ya no existe).
    *   Esperar a que los servidores de Apple validen el binario y luego adjuntar el ticket usando `xcrun stapler staple`.
