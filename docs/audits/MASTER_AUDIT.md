# MASTER_AUDIT.md — Auditoría Unificada OmniMon v6.0.1

> Estado actual 2026-06-29: este documento es una captura histórica del
> 2026-03-08. La app actual está en v6.7.0. En esta continuación pasaron las
> compuertas locales de desktop frontend, Rust workspace, landing, clippy,
> coverage frontend y `bun audit`. El updater `pubkey` ya está poblado y el
> audit frontend de CI ya no es `continue-on-error`. Usar `omnimon_apps/MASTER_SPEC.md`
> y `specs/001-ecosystem-doc-audit/audit/validation/seventy-second-continuation-2026-06-29.md`
> como evidencia actual.

**Fecha:** 2026-03-08
**Coordinador:** Agente Opus (Claude Opus)
**Proyecto:** OmniMon v6.0.1
**Alcance:** Full-stack (Backend Rust + Frontend Svelte + Seguridad + Testing + Documentación + Landing)

---

## Resumen Ejecutivo

Se completó una auditoría multi-agente paralela de OmniMon v6.0.1 con 7 agentes especializados trabajando en worktrees independientes. Se identificaron **78 hallazgos** clasificados por severidad.

### Métricas Globales

| Métrica | Valor |
|---------|-------|
| Issues CRÍTICOS | 1 |
| Issues ALTOS | 6 |
| Issues MEDIOS | 30 |
| Issues BAJOS | 20 |
| Issues INFORMATIVOS | 4 |
| Controles positivos | 15 |
| Tests existentes OK | 699 (476 frontend + 223 Rust) |
| Coverage frontend | 71.35% (meta: 85%) |
| Coverage Rust core | 71.63% (meta: 85%) |
| NIST cumplimiento | 57% cumple, 39% parcial, 9% no cumple |
| CVEs activos | 1 (time 0.3.45 — RUSTSEC-2026-0009) |
| Postura de seguridad | 7/10 |

### Inventario por Agente

| Severidad | OPUS-RUST | OPUS-SECURITY | GEMINI-FRONTEND | GEMINI-ARCH | GPT-TESTING | GPT-DOCS | **TOTAL** |
|-----------|-----------|---------------|-----------------|-------------|-------------|----------|-----------|
| CRÍTICO | 1 | 0 | 0 | 0 | 0 | 0 | **1** |
| ALTO | 3 | 2 | 3 | 2 | 2 | 1 | **13** |
| MEDIO | 5 | 14 | 5 | 4 | 3 | 2 | **33** |
| BAJO | 4 | 12 | 4 | 3 | 3 | 1 | **27** |
| INFO | 0 | 4 | 0 | 0 | 0 | 0 | **4** |
| **TOTAL** | **13** | **32** | **12** | **9** | **8** | **4** | **78** |

---

## Estado de Worktrees y Agentes

| Agente | Worktree | Branch | Estado | Reporte |
|--------|----------|--------|--------|---------|
| OPUS-RUST | main (sin worktree propio) | main | AUDIT_RUST.md sin commit | AUDIT_RUST.md |
| OPUS-SECURITY | worktree-opus-2 | audit/opus-security | Committed | AUDIT_SECURITY.md |
| GEMINI-FRONTEND | worktree-gemini-1 | audit/gemini-frontend | AUDIT sin commit | AUDIT_FRONTEND.md |
| GEMINI-ARCH | worktree-gemini-2 | audit/gemini-arch | Committed | AUDIT_ARCHITECTURE.md |
| GPT-TESTING | worktree-gpt-1 | audit/gpt-testing | 7 archivos modificados SIN commit | AUDIT_TESTING.md |
| GPT-DOCS | worktree-gpt-2 | audit/gpt-docs | 8 archivos modificados + 4 nuevos SIN commit | AUDIT_DOCS.md + CVE_REPORT.md + COMMANDS_REFERENCE.md + NIST_COMPLIANCE.md |
| GEMINI-LANDING | worktree-gemini-3 | audit/gemini-landing | Committed | Landing page funcional |

### Anomalías Detectadas

1. **worktree-opus-1 no existe** — OPUS-RUST trabajó directamente en main, generando `v4/AUDIT_RUST.md` como archivo sin seguimiento
2. **worktree-gpt-1 tiene cambios sin commit** — Tests nuevos y correcciones de tipado pendientes de commit
3. **worktree-gpt-2 tiene cambios sin commit** — Documentación, correcciones de versión y reportes pendientes de commit
4. **worktree-gemini-1 tiene AUDIT sin commit** — El reporte de frontend no fue committeado

---

## Issues Consolidados por Severidad

### CRÍTICO (1 issue) — Bloquea Release

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| **CRIT-01** | **Pubkey vacío en updater de Tauri** — Updates automáticos sin verificación de firma Ed25519. MITM puede servir binarios maliciosos | `tauri.conf.json:40` | OPUS-RUST (C-01) + OPUS-SECURITY (SEC-10) | **OPUS-RUST** |

**Acción inmediata:** Ejecutar `tauri signer generate`, configurar pubkey en `tauri.conf.json`, configurar la private key en CI secrets.

---

### ALTO (6 issues) — Pre-Release

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| **HIGH-01** | **Clave de encriptación hardcodeada en CLI** — `let key = [42u8; 32]` en security-scan. Reportes "encriptados" son desencriptables por cualquiera | `cli/main.rs:691` | OPUS-RUST (A-01) | **OPUS-RUST** |
| **HIGH-02** | **Fallback plain-text para API keys** — Si el keyring falla, las API keys se guardan sin cifrar en JSON | `lib.rs:372-382` | OPUS-RUST (A-02) + OPUS-SECURITY (SEC-09) | **OPUS-SECURITY** |
| **HIGH-03** | **Cache de respuestas IA sin límite** — HashMap crece sin límite, potencial OOM en sesiones largas | `ai.rs:15-18` | OPUS-RUST (A-03) + OPUS-SECURITY (SEC-19) | **OPUS-RUST** |
| **HIGH-04** | **Técnica MITRE T1043 obsoleta** — Revocada en ATT&CK v8 (Oct 2020). Reportes generados son inválidos para SIEM/SOAR modernos | `security.rs:88` | OPUS-SECURITY (SEC-01) | **OPUS-SECURITY** |
| **HIGH-05** | **apply_ai_rules sin rate limiting** — Frontend comprometido puede inyectar/reemplazar reglas sin límite | `lib.rs:395` | OPUS-SECURITY (SEC-02) | **OPUS-SECURITY** |
| **HIGH-06** | **CVE-2026-25727 en time 0.3.45** — DoS por stack exhaustion en parsing RFC 2822 (CVSS 6.8) | `Cargo.lock` (transitiva) | GPT-DOCS (CVE_REPORT) | **OPUS-RUST** |

---

### MEDIO (30 issues)

#### Seguridad y Criptografía (12)

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| MED-01 | Sin zeroización de claves criptográficas en memoria | `crypto.rs` | OPUS-SECURITY (SEC-03) | OPUS-SECURITY |
| MED-02 | Ausencia de Key Derivation Function (PBKDF2/HKDF/Argon2) | `crypto.rs` | OPUS-SECURITY (SEC-04) | OPUS-SECURITY |
| MED-03 | `upsert_rules_from_ai_json` reemplaza TODAS las reglas (no merge) | `rules_engine.rs:216` | OPUS-SECURITY (SEC-05) | OPUS-RUST |
| MED-04 | Prompt injection por blocklist trivialmente evadible | `ai.rs:27-46` | OPUS-SECURITY (SEC-06) | OPUS-SECURITY |
| MED-05 | `analyze_with_ai()` no aplica `check_prompt_injection()` | `ai.rs:366` | OPUS-SECURITY (SEC-07) | OPUS-SECURITY |
| MED-06 | Tool calling no valida estructura de argumentos | `ai.rs:649` | OPUS-SECURITY (SEC-08) | OPUS-SECURITY |
| MED-07 | `kill_processes` consume 1 token de rate limit para N PIDs ilimitados | `lib.rs:316-330` | OPUS-SECURITY (SEC-11) | OPUS-RUST |
| MED-08 | Base de datos CVE local sin verificación de integridad/firma | `audit.rs:78` | OPUS-SECURITY (SEC-12) | OPUS-SECURITY |
| MED-09 | GitHub Actions no pinneadas por SHA (supply chain) | `omnimon-ci.yml` | OPUS-SECURITY (SEC-13) | GPT-TESTING |
| MED-10 | Artefactos de release sin firma Ed25519 | `omnimon-ci.yml` | OPUS-SECURITY (SEC-14) | OPUS-SECURITY |
| MED-11 | Cobertura MITRE incompleta — faltan T1059, T1071, T1053 | `security.rs` | OPUS-SECURITY (SEC-15) | OPUS-SECURITY |
| MED-12 | Comandos de automations/plugins sin rate limiting | `automations.rs` | OPUS-SECURITY (SEC-16) | OPUS-RUST |

#### Rendimiento Backend (5)

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| MED-13 | Clonación profunda de `SystemState` en cada tick (50-100KB/2s) | `watcher.rs:333-339` | OPUS-RUST (M-01) | OPUS-RUST |
| MED-14 | Clonación O(n*m) de `SuperProcess` en telemetría | `telemetry.rs:48-53` | OPUS-RUST (M-02) | OPUS-RUST |
| MED-15 | Substring matching en automations causa falsos positivos | `automations.rs:155-156` | OPUS-RUST (M-03) | OPUS-RUST |
| MED-16 | Separador invisible U+001F sin constante documentada | `browser.rs:275` | OPUS-RUST (M-04) | OPUS-RUST |
| MED-17 | Poison recovery con `eprintln!` en vez de `tracing::error!` | Múltiples | OPUS-RUST (M-05) | OPUS-RUST |

#### Frontend y UX (7)

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| MED-18 | App.svelte (~55KB) demasiado grande — mezcla routing, layout, modales | `App.svelte` | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-19 | AIChat.svelte sin auto-scroll al último mensaje | `AIChat.svelte` | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-20 | AIChat.svelte muestra markdown crudo sin renderizar | `AIChat.svelte` | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-21 | AIChat.svelte no respeta el idioma configurado (i18n) | `AIChat.svelte` | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-22 | ProcessTable sin agrupación de procesos por nombre | `ProcessTable.svelte` | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-23 | Faltan empty states ilustrados, loaders fluidos | Múltiples componentes | GEMINI-FRONTEND | GEMINI-FRONTEND |
| MED-24 | CSS inline repetitivo — falta sistema de temas robusto | Múltiples componentes | GEMINI-FRONTEND | GEMINI-FRONTEND |

#### Arquitectura y CI/CD (6)

| ID | Descripción | Archivo | Origen | Agente Fix |
|----|-------------|---------|--------|------------|
| MED-25 | Divergencia CLI ↔ GUI — features exclusivos en cada lado | cli vs desktop | GEMINI-ARCH | OPUS-RUST + GEMINI-FRONTEND |
| MED-26 | Polling Pull model (setInterval) → debería ser Push (Tauri events) | `processes.ts` + `lib.rs` | GEMINI-ARCH | OPUS-RUST + GEMINI-FRONTEND |
| MED-27 | Coverage frontend global 71.35% — meta 85% no cumplida | Tests | GPT-TESTING | GPT-TESTING |
| MED-28 | Coverage Rust core global 71.63% — meta 85% no cumplida | Tests | GPT-TESTING | GPT-TESTING |
| MED-29 | CI matrix incompleta — frontend solo se valida en Linux | `omnimon-ci.yml` | GPT-TESTING | GPT-TESTING |
| MED-30 | Versiones inconsistentes en runtime (6.0.3 vs 6.0.1 en UI) | App.svelte, main.rs, etc. | GPT-DOCS | GPT-DOCS |

---

### BAJO (20 issues)

#### Backend (4)

| ID | Descripción | Archivo | Origen |
|----|-------------|---------|--------|
| LOW-01 | `unwrap_or(0)` en `usize::try_from(read_len)` — defensivo pero innecesario | `network.rs:807` | OPUS-RUST (B-01) |
| LOW-02 | `let _ = fs::remove_file` ignora errores en rotación audit trail | `audit_trail.rs:98` | OPUS-RUST (B-02) |
| LOW-03 | Sleep 300ms no-configurable en kill graceful | `killer.rs:244` | OPUS-RUST (B-03) |
| LOW-04 | `System::new_all()` + `refresh_all()` en metrics fallback (cold-start) | `metrics.rs:83-84` | OPUS-RUST (B-04) |

#### Seguridad Menor (12)

| ID | Descripción | Archivo | Origen |
|----|-------------|---------|--------|
| LOW-05 | Comparación SHA-256 sin tiempo constante | `crypto.rs:136` | OPUS-SECURITY (SEC-17) |
| LOW-06 | Cobertura incompleta de procesos protegidos del OS | `killer.rs:9-68` | OPUS-SECURITY (SEC-18) |
| LOW-07 | `lsof`/`netstat` invocados sin ruta absoluta | `network.rs:476` | OPUS-SECURITY (SEC-20) |
| LOW-08 | Ruta eBPF configurable via env var sin verificación de integridad | `network.rs` | OPUS-SECURITY (SEC-21) |
| LOW-09 | `apply_ai_rules` sin límite de tamaño ni número de reglas | `lib.rs:395-397` | OPUS-SECURITY (SEC-22) |
| LOW-10 | `tool_call` en `ipcAiChat` no se valida en profundidad | `ipc.ts:388-389` | OPUS-SECURITY (SEC-23) |
| LOW-11 | CDP usa HTTP sin cifrar (inherente al protocolo, solo localhost) | `browser.rs` | OPUS-SECURITY (SEC-24) |
| LOW-12 | Funciones CDP públicas aceptan `base_url` arbitrario | `browser.rs` | OPUS-SECURITY (SEC-25) |
| LOW-13 | `save_cloud_key` IPC sin validación de formato | `lib.rs:530` | OPUS-SECURITY (SEC-26) |
| LOW-14 | Falta directiva `img-src` explícita en CSP | `tauri.conf.json:31` | OPUS-SECURITY (SEC-27) |
| LOW-15 | Estado global sin autenticación de escritor | `watcher.rs:17` | OPUS-SECURITY (SEC-28) |
| LOW-16 | `security` job en CI con `continue-on-error: true` | `omnimon-ci.yml` | GPT-TESTING |

#### Testing y Documentación (4)

| ID | Descripción | Archivo | Origen |
|----|-------------|---------|--------|
| LOW-17 | Tests frontend con stderr ruidoso que oculta errores reales | Tests | GPT-TESTING |
| LOW-18 | No hay SAST/CodeQL en pipeline | `omnimon-ci.yml` | GPT-TESTING |
| LOW-19 | No hay `bun audit` en pipeline | `omnimon-ci.yml` | GPT-TESTING |
| LOW-20 | No hay fuzzing automatizado para parsers | `omnimon-ci.yml` | GPT-TESTING |

---

### INFORMATIVO (4 issues)

| ID | Descripción | Origen |
|----|-------------|--------|
| INFO-01 | Campo `public_key_b64` en `ReleaseSignature` potencialmente confuso | OPUS-SECURITY |
| INFO-02 | Confidence score estático (0.7/0.9) sin contexto multi-señal | OPUS-SECURITY |
| INFO-03 | Job `security` con `continue-on-error: true` en CI | OPUS-SECURITY |
| INFO-04 | Ollama local funciona correctamente via proxy del backend Rust | OPUS-SECURITY |

---

## NIST SP 800-53 — Resumen de Cumplimiento

| Estado | Controles | % |
|--------|-----------|---|
| Cumple | AC-3, AC-5, AC-17, AU-2, AU-3, AU-8, IA-7, IA-9, SC-8, SC-23, SI-3, SI-5, SI-10 (parcial) | 57% |
| Parcial | AC-6, AU-9, AU-10, IA-5, SC-13, SC-17, SC-28, SI-2, CA-7, CM-8, IR-4 | 39% |
| No cumple | **SC-12** (Key Management), **SI-7** (Software/Firmware Integrity) | 9% |

**Controles críticos faltantes:**
- **SC-12:** Sin KDF, sin zeroización de claves → Requiere `zeroize` + HKDF
- **SI-7:** Updater sin pubkey, releases sin firma → Requiere pipeline de firma Ed25519

---

## Controles Positivos Destacados (15)

| # | Control | Módulo |
|---|---------|--------|
| 1 | API keys en keyring nativo del OS | ai.rs, lib.rs |
| 2 | TLS obligatorio para APIs cloud | ai.rs |
| 3 | Rate limiting Token Bucket en IPC | rate_limit.rs |
| 4 | Blocklist inmutable de procesos del OS | killer.rs |
| 5 | Acciones destructivas diferidas (confirmación) | lib.rs |
| 6 | AES-256-GCM con nonces CSPRNG | crypto.rs |
| 7 | Ed25519 para verificación de releases | crypto.rs |
| 8 | Sanitización de tabs CDP | browser.rs |
| 9 | CSP estricto sin unsafe-inline/unsafe-eval | tauri.conf.json |
| 10 | Capabilities Tauri mínimas | capabilities/default.json |
| 11 | `catch_unwind()` en watcher | watcher.rs |
| 12 | Aritmética saturante en red | network.rs |
| 13 | TruffleHog para secretos en PRs | CI/CD |
| 14 | Sandbox Lua (256KB, 1MB, 150ms) | plugins.rs |
| 15 | Validación runtime IPC exhaustiva | ipc.ts |

---

## Plan de Remediación por Fases

### Fase 1 — Inmediata (1-2 semanas) — Bloquea Release

| Sprint | Issue | Acción | Agente | Esfuerzo |
|--------|-------|--------|--------|----------|
| 1.1 | CRIT-01 | Generar keypair Ed25519, configurar pubkey en tauri.conf.json | OPUS-RUST | 30 min |
| 1.2 | HIGH-06 | `cargo update -p time` para resolver CVE-2026-25727 | OPUS-RUST | 5 min |
| 1.3 | HIGH-04 | Reemplazar T1043 → T1071 en security.rs | OPUS-SECURITY | 10 min |
| 1.4 | HIGH-05 | Agregar rate limiting a `apply_ai_rules` | OPUS-SECURITY | 15 min |
| 1.5 | MED-07 | Limitar batch size en `kill_processes` (max 50 PIDs) | OPUS-RUST | 15 min |
| 1.6 | MED-30 | Normalizar versiones a 6.0.1 en todos los archivos | GPT-DOCS | 30 min |

### Fase 2 — Corto Plazo (2-4 semanas) — Seguridad y Rendimiento

| Sprint | Issue | Acción | Agente | Esfuerzo |
|--------|-------|--------|--------|----------|
| 2.1 | HIGH-01 | Reemplazar key hardcodeada con keyring + generación | OPUS-RUST | 2 hrs |
| 2.2 | HIGH-02 | Cifrar fallback de API keys con crypto::encrypt_json | OPUS-SECURITY | 3 hrs |
| 2.3 | HIGH-03 | Implementar LRU bounded cache (256 entradas) | OPUS-RUST | 1 hr |
| 2.4 | MED-01 | Agregar `zeroize` para claves criptográficas | OPUS-SECURITY | 2 hrs |
| 2.5 | MED-02 | Implementar HKDF para derivación de claves | OPUS-SECURITY | 4 hrs |
| 2.6 | MED-03 | Implementar merge real en `upsert_rules` | OPUS-RUST | 2 hrs |
| 2.7 | MED-09 | Pinnear GitHub Actions por SHA | GPT-TESTING | 1 hr |
| 2.8 | MED-10 | Firmar releases con Ed25519 en CI | OPUS-SECURITY | 4 hrs |
| 2.9 | MED-12 | Rate limiting en automations/plugins IPC | OPUS-RUST | 30 min |
| 2.10 | MED-13 | Retornar `Arc<SystemState>` en vez de clonar | OPUS-RUST | 3 hrs |
| 2.11 | MED-14 | Usar índices para `SuperProcess` en telemetría | OPUS-RUST | 1 hr |

### Fase 3 — Mediano Plazo (1-2 meses) — UX, Testing, Arquitectura

| Sprint | Issue | Acción | Agente | Esfuerzo |
|--------|-------|--------|--------|----------|
| 3.1 | MED-18 | Descomponer App.svelte en layout + controllers | GEMINI-FRONTEND | 1 día |
| 3.2 | MED-19/20/21 | Fix AIChat: auto-scroll + markdown + i18n | GEMINI-FRONTEND | 1 día |
| 3.3 | MED-22 | Agrupación de procesos por nombre en ProcessTable | GEMINI-FRONTEND | 4 hrs |
| 3.4 | MED-26 | Migrar polling Pull → Push (Tauri events) | OPUS-RUST + GEMINI-FRONTEND | 2 días |
| 3.5 | MED-27/28 | Subir coverage a 85%+ (frontend y Rust) | GPT-TESTING | 1 semana |
| 3.6 | MED-29 | Expandir CI matrix (frontend en macOS, workspace en Windows) | GPT-TESTING | 4 hrs |
| 3.7 | MED-04/05 | Mejorar detección de prompt injection | OPUS-SECURITY | 2 días |
| 3.8 | MED-11 | Ampliar cobertura MITRE ATT&CK | OPUS-SECURITY | 1 día |
| 3.9 | MED-25 | Paridad CLI ↔ GUI (security-scan a GUI, network a CLI) | OPUS-RUST + GEMINI-FRONTEND | 1 semana |
| 3.10 | MED-23/24 | Empty states, loaders, sistema de temas | GEMINI-FRONTEND | 3 días |

### Fase 4 — Largo Plazo (2-3 meses) — Hardening y Polish

| Sprint | Issue | Acción | Agente | Esfuerzo |
|--------|-------|--------|--------|----------|
| 4.1 | LOW-05 | Comparación SHA-256 con tiempo constante | OPUS-SECURITY | 30 min |
| 4.2 | LOW-07 | Rutas absolutas para lsof/netstat | OPUS-RUST | 30 min |
| 4.3 | LOW-16/18/19/20 | SAST (CodeQL), bun audit, fuzzing, fix continue-on-error | GPT-TESTING | 1 día |
| 4.4 | MED-08 | Firmar base de datos CVE con Ed25519 | OPUS-SECURITY | 4 hrs |
| 4.5 | MED-15 | Soportar regex en automations (no solo substring) | OPUS-RUST | 2 hrs |
| 4.6 | AI Streaming | Implementar streaming real de tokens en GUI | OPUS-RUST + GEMINI-FRONTEND | 3 días |

---

## Redistribución de Agentes para Fase de Implementación (Fase 4 del Plan)

### OPUS-RUST — Worktree: `fix/opus-rust`
```
Foco: Fixes críticos de backend, rendimiento, y CVE
Issues asignados: CRIT-01, HIGH-01, HIGH-03, HIGH-06, MED-03, MED-07,
                  MED-12, MED-13, MED-14, MED-15, MED-16, MED-17, MED-26 (backend)
Archivos a modificar:
  - apps/desktop/src-tauri/tauri.conf.json (pubkey)
  - crates/core/src/ai.rs (LRU cache)
  - crates/core/src/watcher.rs (Arc<SystemState>)
  - crates/core/src/telemetry.rs (indices SuperProcess)
  - crates/cli/src/main.rs (key hardcodeada)
  - apps/desktop/src-tauri/src/lib.rs (batch kill limit, rate limit automations)
  - apps/desktop/src-tauri/src/automations.rs (regex matching)
  - crates/core/src/browser.rs (constante FIELD_SEP)
  - crates/core/src/rules_engine.rs (merge real)
  - Cargo.lock (cargo update -p time)
  - Múltiples archivos (tracing::error! en poison recovery)
Comandos de validación:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
```

### OPUS-SECURITY — Worktree: `fix/opus-security`
```
Foco: Criptografía, MITRE, firma de releases, prompt injection
Issues asignados: HIGH-02, HIGH-04, HIGH-05, MED-01, MED-02, MED-04,
                  MED-05, MED-06, MED-08, MED-10, MED-11
Archivos a modificar:
  - crates/core/src/crypto.rs (zeroize, HKDF)
  - crates/core/src/security.rs (T1043→T1071, nuevas técnicas MITRE)
  - crates/core/src/ai.rs (check_prompt_injection mejorado, analyze_with_ai protection)
  - crates/core/src/audit.rs (firma CVE DB)
  - apps/desktop/src-tauri/src/lib.rs (rate limit apply_ai_rules, cifrar fallback API keys)
  - Cargo.toml core (agregar zeroize, hkdf, subtle)
  - .github/workflows/omnimon-ci.yml (firma de releases)
Comandos de validación:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
  - cargo audit
```

### GEMINI-FRONTEND — Worktree: `fix/gemini-frontend`
```
Foco: UX, componentes Svelte, temas, auto-scroll, markdown
Issues asignados: MED-18, MED-19, MED-20, MED-21, MED-22, MED-23,
                  MED-24, MED-26 (frontend)
Archivos a modificar:
  - apps/desktop/src/App.svelte (descomposición)
  - apps/desktop/src/components/AIChat.svelte (auto-scroll, markdown, i18n)
  - apps/desktop/src/components/ProcessTable.svelte (agrupación, iconos)
  - apps/desktop/src/components/NetworkMap.svelte (AI panel)
  - apps/desktop/src/stores/processes.ts (push events listener)
  - Nuevos archivos: MainLayout.svelte, ProcessGroup.svelte, theme.ts
Dependencias nuevas: marked, dompurify
Comandos de validación:
  - bun run build
  - bun run test
  - bunx tsc --noEmit
```

### GPT-TESTING — Worktree: `fix/gpt-testing`
```
Foco: Coverage, CI/CD hardening, SAST, gates
Issues asignados: MED-09, MED-27, MED-28, MED-29, LOW-16, LOW-18, LOW-19, LOW-20
Archivos a modificar:
  - .github/workflows/omnimon-ci.yml (SHA pinning, coverage gates, SAST, bun audit, matrix)
  - Nuevos tests para: ConfirmDialog, CloudSync, Plugins, Automations, SmartAlerts
  - crates/core/src/killer.rs (tests adicionales ≥85%)
  - crates/core/src/ai.rs (tests de errores/retries)
Comandos de validación:
  - bun run test:coverage (verificar ≥85%)
  - cargo test --workspace
  - cargo llvm-cov (verificar ≥85%)
```

### GPT-DOCS — Worktree: `fix/gpt-docs`
```
Foco: Versionamiento, documentación actualizada
Issues asignados: MED-30
Archivos a modificar:
  - v4/apps/desktop/src/App.svelte (versión visible)
  - v4/crates/cli/src/main.rs (versión CLI)
  - v4/crates/core/src/cloud.rs (versión cloud)
  - v4/crates/tui/src/ui.rs (versión TUI)
  - README.md, CHANGELOG.md, docs/CLI_MANUAL.md
  - COMMANDS_REFERENCE.md (nuevo)
  - NIST_COMPLIANCE.md (nuevo)
Comandos de validación:
  - cargo check --workspace
  - bun run build
```

### GEMINI-LANDING — Worktree: `fix/gemini-landing` (completado)
```
Estado: La landing page base ya fue construida y committeada.
Pendiente: Integrar tutorial interactivo con COMMANDS_REFERENCE.md de GPT-DOCS.
```

---

## Orden de Merge Recomendado

```bash
# Desde main, mergear en orden de dependencia:
git checkout main

# 1. Primero: fixes de backend (otros dependen de esto)
git merge fix/opus-rust --no-ff -m "fix: remediación backend Rust — pubkey, cache, performance"

# 2. Segundo: seguridad (depende de cambios en core)
git merge fix/opus-security --no-ff -m "fix: remediación seguridad — crypto, MITRE, firma releases"

# 3. Tercero: testing (valida los fixes anteriores)
git merge fix/gpt-testing --no-ff -m "chore: mejoras testing y CI/CD — coverage gates, SAST"

# 4. Cuarto: documentación (refleja estado final)
git merge fix/gpt-docs --no-ff -m "docs: normalización versiones, compliance, referencia de comandos"

# 5. Quinto: frontend (puede tocar archivos compartidos con backend)
git merge fix/gemini-frontend --no-ff -m "feat: mejoras UI/UX — auto-scroll, markdown, temas"

# 6. Último: landing (proyecto independiente)
git merge fix/gemini-landing --no-ff -m "feat: landing page OmniMon"
```

**Nota sobre conflictos:** Los archivos con mayor probabilidad de conflicto son:
- `lib.rs` (tocado por OPUS-RUST, OPUS-SECURITY, GPT-DOCS)
- `ai.rs` (tocado por OPUS-RUST, OPUS-SECURITY)
- `App.svelte` (tocado por GEMINI-FRONTEND, GPT-DOCS)
- `omnimon-ci.yml` (tocado por GPT-TESTING, OPUS-SECURITY)

Opus es el mejor agente para resolver conflictos de merge en Rust.

---

## Worktrees Pendientes de Limpieza

Después de completar los merges:
```bash
git worktree remove ../worktree-opus-2
git worktree remove worktree-gemini-1
git worktree remove worktree-gemini-2
git worktree remove worktree-gpt-1
git worktree remove worktree-gpt-2
git worktree remove worktree-gemini-3
```

---

## Resumen Visual

```
┌─────────────────────────────────────────────────────────────────┐
│              FASE 1 COMPLETADA: ANÁLISIS PARALELO               │
│                                                                 │
│  OPUS-RUST ✅    GEMINI-FRONTEND ✅    GPT-TESTING ✅            │
│  OPUS-SECURITY ✅ GEMINI-ARCH ✅       GPT-DOCS ✅               │
│                  GEMINI-LANDING ✅                               │
├─────────────────────────────────────────────────────────────────┤
│              FASE 2 COMPLETADA: UNIFICACIÓN                     │
│                                                                 │
│  docs/audits/MASTER_AUDIT.md ← Este archivo                    │
│  docs/audits/AUDIT_RUST.md                                     │
│  docs/audits/AUDIT_SECURITY.md                                 │
│  docs/audits/AUDIT_FRONTEND.md                                 │
│  docs/audits/AUDIT_ARCHITECTURE.md                             │
│  docs/audits/AUDIT_TESTING.md                                  │
│  docs/audits/AUDIT_DOCS.md                                     │
│  docs/audits/CVE_REPORT.md                                     │
│  docs/audits/COMMANDS_REFERENCE.md                             │
│  docs/audits/NIST_COMPLIANCE.md                                │
├─────────────────────────────────────────────────────────────────┤
│              FASE 3 COMPLETADA: PRIORIZACIÓN                    │
│                                                                 │
│  CRÍTICO: 1 issue  (bloquea release)                           │
│  ALTO:    6 issues (pre-release)                               │
│  MEDIO:  30 issues (post-release, por sprints)                 │
│  BAJO:   20 issues (nice-to-have)                              │
│  INFO:    4 issues (observaciones)                              │
├─────────────────────────────────────────────────────────────────┤
│          FASE 4 SIGUIENTE: IMPLEMENTACIÓN POR SPRINTS           │
│                                                                 │
│  Sprint 1 (1-2 sem): CRIT-01 + HIGH-01..06 + MED-07,30        │
│  Sprint 2 (2-4 sem): Seguridad + Rendimiento (11 issues)       │
│  Sprint 3 (1-2 mes): UX + Testing + Arquitectura (10 issues)   │
│  Sprint 4 (2-3 mes): Hardening + Polish (6 issues)             │
│                                                                 │
│  Agentes redistribuidos:                                        │
│  ├── OPUS-RUST        → fix/opus-rust       (13 issues)        │
│  ├── OPUS-SECURITY    → fix/opus-security   (11 issues)        │
│  ├── GEMINI-FRONTEND  → fix/gemini-frontend  (8 issues)        │
│  ├── GPT-TESTING      → fix/gpt-testing      (8 issues)        │
│  ├── GPT-DOCS         → fix/gpt-docs         (1 issue)         │
│  └── GEMINI-LANDING   → completado                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Herramientas Utilizadas por Cada Agente

| Agente | Herramienta | Comando | Resultado |
|--------|-------------|---------|-----------|
| OPUS-RUST | cargo fmt --check | `cargo fmt --check` | Limpio |
| OPUS-RUST | cargo clippy | `cargo clippy --workspace -- -D warnings` | Falla (build Tauri) |
| OPUS-RUST | cargo test | `cargo test --workspace` | 95 tests OK |
| OPUS-SECURITY | cargo audit | `cargo audit` | 1 CVE (time 0.3.45) |
| GEMINI-FRONTEND | bun build | `bun run build` | OK (-15% bundle) |
| GEMINI-FRONTEND | bun test | `bun run test` | 100% pass |
| GPT-TESTING | bun test | `bun run test` | 476 tests OK |
| GPT-TESTING | cargo test | `cargo test --workspace` | 223 tests OK |
| GPT-TESTING | tsc check | `bunx tsc --noEmit` | OK (post-fix) |
| GPT-TESTING | coverage frontend | `bun run test:coverage` | 71.35% stmts |
| GPT-TESTING | coverage Rust | `cargo llvm-cov` | 71.63% lines |
| GPT-DOCS | cargo audit | `cargo audit --json` | 1 CVE confirmada |
| GPT-DOCS | cargo check | `cargo check --workspace` | OK |
| GEMINI-LANDING | bun build | `bun run build` | OK (<1s) |

---

## Apéndice: Reportes Individuales

Todos los reportes originales están disponibles en `docs/audits/`:

1. [AUDIT_RUST.md](./AUDIT_RUST.md) — Backend Rust (OPUS-RUST)
2. [AUDIT_SECURITY.md](./AUDIT_SECURITY.md) — Seguridad (OPUS-SECURITY)
3. [AUDIT_FRONTEND.md](./AUDIT_FRONTEND.md) — Frontend Svelte (GEMINI-FRONTEND)
4. [AUDIT_ARCHITECTURE.md](./AUDIT_ARCHITECTURE.md) — Arquitectura (GEMINI-ARCH)
5. [AUDIT_TESTING.md](./AUDIT_TESTING.md) — Testing y CI/CD (GPT-TESTING)
6. [AUDIT_DOCS.md](./AUDIT_DOCS.md) — Documentación (GPT-DOCS)
7. [CVE_REPORT.md](./CVE_REPORT.md) — Vulnerabilidades CVE (GPT-DOCS)
8. [COMMANDS_REFERENCE.md](./COMMANDS_REFERENCE.md) — Referencia de Comandos (GPT-DOCS)
9. [NIST_COMPLIANCE.md](./NIST_COMPLIANCE.md) — Cumplimiento NIST (GPT-DOCS)

---

*Generado por el Agente Coordinador Opus — 2026-03-08*
