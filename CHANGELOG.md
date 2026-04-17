# Changelog

## Unreleased

### Zombie Killer
- Nuevo motor que detecta procesos con CPU o RAM altos sostenidos durante ventanas prolongadas y los ofrece para terminación con confirmación del usuario
- Core `zombie_killer` stateless (`identify_candidates`, `sanitize_config`) con clamps de configuración (umbral de CPU, uptime mínimo, sustained, `never_kill`) e invariantes en compile-time para el throttle de notificaciones
- Motor Tauri stateful con clave compuesta `(pid, start_time)` segura ante reuso de PID, tick body con `catch_unwind`, guard de re-entrada y 5 comandos IPC (`get/set_zombie_killer_config`, `list_zombie_candidates`, `kill_zombie`, `kill_all_zombies`)
- Modal `ZombieKiller.svelte` con atajo Cmd/Ctrl+Shift+Z, i18n EN/ES y evento push `zombie-killer-update`

### Privacidad de IA y presupuesto diario
- `Settings.ai_privacy_mode` (default `false`) activa helpers de redacción en `core/ai.rs`: nombres de proceso, paths, URLs, títulos de pestaña y hostnames/IPs se reemplazan por tokens pseudónimos estables (24-bit SipHash) para que el LLM razone sobre identidad sin ver los strings reales; IPs privadas (RFC 1918, loopback, link-local, `fc00::/7`, `fe80::/10`) colapsan a `<lan>`
- `Settings.ai_daily_limit` (default `200`, `0` = ilimitado) complementa el token bucket de ráfaga con una cubeta diaria UTC compartida por `ai_chat`, `analyze_processes`, `analyze_context` y `validate_api_key`; nuevo IPC `get_ai_daily_usage` devuelve `(usado, límite)`
- Sección "AI Privacy & Budget" en `ProfileSettings` con toggle, input numérico y contador vivo con botón de refresh

### Confirmación frontend de acciones destructivas de IA
- `add_automation_rule` y `remove_automation_rule` ya no se ejecutan server-side en `ai_chat`; devuelven un plan en `details` + payload que `AIChat.svelte` muestra como `pendingAction` e invoca el IPC real sólo tras confirmación explícita (mismo patrón que `kill_process` y `close_tabs`)

### Endurecimiento del keyring
- `get_api_key_with_fallback` ahora borra el almacén legacy en texto plano **antes** de intentar la escritura segura en keyring; si el proceso se interrumpe a mitad del flujo ya no queda la API key legible en disco

### Badge DPI en StatusBar
- Nuevo indicador `role="status" aria-live="polite"` (icono Radar, lucide) que aparece cuando `$networkTelemetryStatus.dpiActive === true`; tooltip explica que OmniMon lee metadatos de paquetes (no payload) y cómo apagar DPI desde settings

### CSP local Ollama
- `tauri.conf.json` permite `http://localhost:11434` y `http://127.0.0.1:11434` en `connect-src` sin relajar el resto de la política

### Refactor de `EvaluatorState` (breaking)
- `crates/core/src/network_alerts.rs`: se elimina el singleton `OnceLock<RwLock<EvaluatorState>>` y las funciones `evaluate_network_alerts` / `evaluate_active_network_alerts` reciben `state: &mut EvaluatorState` como último argumento
- El watcher asigna un `EvaluatorState` una vez al arranque del thread y lo reusa entre ticks (hot path sin asignaciones); cada test ya construye su propio estado local
- Resuelve el flake `active_rules_drive_evaluate_active_network_alerts` que surgía porque el `consecutive_matches` global se contaminaba entre test threads
- Migración: `evaluate_network_alerts(&snap, prev, &rules, &history, &mut state)` — mantener `state` vivo entre ticks para preservar debounce + cooldown

### Calidad
- Rust: 288 core + 95 integration + 53 tauri + 18 + 4 tui tests, `cargo clippy --workspace --all-targets -- -D warnings` limpio bajo Rust 1.95 (nuevos lints `unnecessary_sort_by` y `collapsible_match` arreglados en 10 call sites pre-existentes)
- Frontend: 689 Vitest cases; coverage 70.37–70.80% branches, 86% statements/functions/lines
- Tests nuevos: 17 redacción en `ai.rs`, 5 `DailyBucket`, 8 `zombie_killer` (core) + 8 (tauri), 5 `ProfileSettings` (privacy/budget), 4 `Automations` (errores IPC), 4 `ZombieKiller` (formatters, error paths, blocklist), 7 `validateAiRule`

## 6.3.0 (2026-03-10)

### Perfiles de Usuario
- Modelo expandido: displayName, profilePreset (minimal/balanced/power), dashboardLayout, refreshInterval, favoriteProcesses, notificationLevel
- `ProfileSettings.svelte`: selector visual de presets, slider de intervalo, gestión de procesos favoritos, reset to defaults
- Preset controla secciones visibles en el dashboard
- Procesos favoritos pinneados al top de ProcessTable
- Filtro de notificaciones por nivel (off/critical/all)
- Persistencia automática con debounce 500ms
- Traducciones EN/ES completas

### E2E Tests
- Migración de WebdriverIO/Tauri a Playwright standalone
- 5 suites E2E: app-loads, process-table, navigation, settings, ai-chat
- Fixtures con mocks de Tauri IPC (métricas, tabs, red, AI)
- Helpers reutilizables para tabla, modales, navegación

### Auditoría Post-Sprint
- 29 imports muertos eliminados en 12 componentes
- Bug corregido: NetworkMap onclick handler inválido fuera de scope
- 3 implicit `any` resueltos (processes.ts, network.svelte.ts)
- `ProcessNetworkThroughput.process_name` sincronizado con Rust
- Rate limiting agregado a 5 IPC commands desprotegidos
- CSP endurecido: `object-src 'none'; base-uri 'self'`
- Virtual scroll buffer: 0 → 3 rows para evitar parpadeo

### Calidad
- 1037 tests (663 Frontend + 367 Rust + 7 E2E)
- Coverage: statements 86.5%, branches 72%, functions 87.7%
- +85 tests unitarios nuevos en 16 archivos
- processIcons 62% → 92% branches, theme 68% → 86%, preferences 69% → 85%, alerts 62% → 87%

### Documentación
- README.md reescrito para v6.3.0 con arquitectura ASCII y badges
- `docs/ARCHITECTURE.md` nuevo: 7 diagramas Mermaid, 13 módulos documentados
- CONTRIBUTING.md actualizado con workflow completo
- COMMANDS_REFERENCE.md: +15 comandos CLI, +3 IPC commands
- CLI_MANUAL.md: 4 secciones nuevas (config, network, rules, release)

## 6.2.0 (2026-03-09)

### Red Avanzada
- Motor de captura de conexiones activas cross-platform (lsof macOS, /proc/net Linux, GetExtendedTcpTable Windows)
- Modelo de datos `NetworkConnection` con protocolo, estado, throughput, hostname y GeoIP
- `NetworkSnapshot` con historial circular de 60 snapshots (5 min)
- DNS reverse lookup asíncrono con cache (TTL 5 min, max 10 lookups concurrentes)
- Sistema de filtrado: protocolo, puerto, proceso, PID, host, throughput, localhost, established
- Integración con watcher daemon: captura cada 6s, eventos push Tauri `network-update`
- 3 comandos IPC: `get_network_connections`, `get_network_history`, `get_filtered_connections`

### Alertas de Red
- Modelo `NetworkAlertRule` con 6 tipos de condición: alto bandwidth, conexión externa, puerto sospechoso, spike de proceso, exceso de conexiones, destino sospechoso
- Motor de evaluación con debounce (3 snapshots consecutivos) y cooldown configurable
- 4 reglas de fábrica: alto bandwidth (>50 MB/s), puertos sospechosos, spike ×5, >200 conexiones
- UI de configuración con toggle on/off, modal de creación, persistencia en preferencias
- Notificaciones integradas con botones "Investigar" y "Preguntarle a IA"
- Evento push Tauri `network-alert`

### Dashboard de Red (Frontend)
- `NetworkDashboard.svelte`: métricas en tiempo real (upload/download, conexiones activas, sparklines)
- `ConnectionsTable.svelte`: tabla ordenable con filtros por protocolo, proceso, dominio, velocidad mínima
- `ProcessNetworkView.svelte`: agrupación por proceso con distribución de bandwidth
- `NetworkMap.svelte` refactorizado: grafo SVG interactivo con nodos remotos posicionados en círculo
- `ConnectionDetail.svelte`: panel de detalle con IP, hostname, país, throughput, botones IA
- Animaciones de tráfico: partículas SVG en líneas de conexión, pulso en nodos activos
- Store reactivo `network.svelte.ts` con Svelte 5 ($state, $derived)

### IA de Red
- Preset "Analizar tráfico de red": top 10 conexiones, puertos abiertos, procesos con más tráfico
- Preset "Anomalías de red": detección de IPs desconocidas, puertos inusuales, tráfico excesivo
- Contexto de red inyectado en `build_chat_system_prompt` (ai.rs)
- Botón "¿Qué es esto?" en ConnectionDetail → consulta IA contextual
- Tool calling `close_connection` con confirmación en frontend
- Traducciones EN/ES para herramientas de red

### Cross-platform Hardening
- Windows: `GetExtendedTcpTable`/`GetExtendedUdpTable` con fallback a netstat
- Linux: parsing refactorizado de /proc/net/tcp|udp con inode→PID map, fallback a `ss`
- Linux: fix IPv6 hex little-endian por grupos de 4 bytes
- macOS: detección automática de ruta lsof, timeout 10s en comandos, timeout 3s en DNS
- macOS: skip de IPs privadas en DNS reverse lookup

### CLI
- `omnimon network --connections`: tabla de conexiones activas
- `omnimon network --filter tcp --port 443`: filtrado por protocolo y puerto
- `omnimon network --alerts`: listar alertas activas
- `omnimon network --top`: top 10 procesos por tráfico
- `omnimon network --watch`: modo live con refresh configurable

### Calidad
- 941 tests (351 Rust + 590 Frontend)
- +20 tests de parsing mock (lsof, /proc/net, netstat, ss)
- Tests de NetworkFilter, DNS cache, evaluación de alertas
- Tests frontend: NetworkDashboard, ConnectionsTable, ProcessNetworkView, NetworkAlertConfig, network store
- Tests CLI: network subcommands

### Documentación
- `docs/NETWORK_ANALYSIS.md`: guía completa de análisis de red
- `COMMANDS_REFERENCE.md` actualizado con comandos de red

### CI
- Fix Windows: `network-capture` feature solo en non-Windows (cli, tui)
- Coverage Gates: depende de lint (no de test), evita bloqueo por plataforma
- Rename script: manejo de assets duplicados al re-publicar tag

## 6.1.0 (2026-03-08)

### Seguridad
- Pubkey Ed25519 real en auto-updater (reemplaza placeholder)
- Eliminado fallback plain-text de API keys, solo keyring nativo del OS
- Zeroización de claves en memoria con `zeroize` crate
- HKDF-SHA256 para derivación de claves con contextos de dominio
- Rotación de claves: `omnimon config rotate-key`
- Rate limiting en `apply_ai_rules`
- Firma digital de releases Ed25519 + SHA-256 checksums
- Prompt injection defense: 25 patrones en frontend y backend
- GitHub Actions pinneadas a SHA hashes
- `cargo audit` + `bun audit` integrados en CI
- NIST SC-12 (Key Management) y SI-7 (Software Integrity) cumplidos

### Correcciones
- MITRE ATT&CK: T1043→T1071 (técnica obsoleta corregida)
- CVE-2026-25727 documentado (time 0.3.45, bloqueado por upstream)
- Contexto de CPU corregido en reportes de IA (per-core vs total)
- Markdown renderizado correctamente en alertas y chat de IA
- AIChat: scroll automático y respeto de idioma (i18n)
- Alertas de salud: debounce de picos transitorios (3 lecturas consecutivas)
- Alertas agrupadas por proceso (no más duplicados apilados)
- Porcentaje CPU >100% muestra equivalencia en cores
- Tests flaky de cache AI eliminados (race condition en TTL global)

### Nuevas funcionalidades
- Agrupación de procesos por nombre (Chrome ×15 → 1 fila expandible)
- Iconos de aplicaciones en tabla de procesos
- Dashboards clickeables con consulta IA contextual
- Streaming de tokens en chat de IA
- 7 presets de prompts para análisis rápido
- 5 herramientas de tool calling: get_process_details, get_network_details, run_security_scan, explain_process, get_system_summary
- Cache IA con TTL configurable (0–60 min, default 5)
- Sistema de temas: Dark, Light, Cyberpunk
- Microanimaciones, transiciones suaves, loading skeletons
- Empty states ilustrados en listas vacías
- Input numérico para tamaño de fuente
- Paridad CLI↔GUI: comandos `network` y `rules` en CLI
- Botón "Cerrar todas" en alertas (máximo 5 visibles)

### Rendimiento
- Polling migrado a eventos push de Tauri (modelo push vs pull)
- Eliminados ~500 clones de String por tick en watcher hot path
- App.svelte descompuesto de 1757→~200 líneas (layout 2 columnas)

### Calidad
- 857 tests (282 Rust + 575 Frontend)
- Coverage: frontend 84% statements, killer 87%, rules_engine 96%, browser 91%
- Coverage gates en CI (75% frontend, 70% Rust)
- Migración de reactive statements $: a $derived de Svelte 5

### Documentación
- KEY_MANAGEMENT.md — Política NIST SC-12
- RELEASE_SIGNING.md — Proceso completo de firma de releases
- SECURITY_KEYS.md — Configuración de claves para Tauri Updater
- CVE_REPORT.md — Tracking de vulnerabilidades conocidas
- PARITY_GAPS.md — Roadmap CLI↔GUI

## 6.0.1 (2026-03-08)

### Documentation
- Refresh main README to align the public docs with OmniMon 6.0.1
- Add `AUDIT_DOCS.md`, `COMMANDS_REFERENCE.md`, `CVE_REPORT.md`, and `NIST_COMPLIANCE.md`
- Expand `docs/CLI_MANUAL.md` with the full CLI surface and practical examples

### Compliance
- Capture `cargo audit` results, remediation guidance, and dependency risk status
- Map implemented safeguards against selected NIST SP 800-53 controls and identify coverage gaps

### Versioning
- Align runtime-visible version strings in the desktop footer, Tauri About metadata, CLI banner, TUI title, and cloud user agent

## 4.0.7 (2026-03-05)

### Branding
- Rename product to OmniMon across all configs (tauri.conf.json, Cargo.toml, package.json)
- Translate README and all scripts to English for international audience
- Update Homebrew tap from `chochy2001/tap` to `chochy2001/omnimon`

### Frontend
- IPC security hardening: runtime type guards on every IPC response (`src/lib/ipc.ts`)
- Virtual scroll in ProcessTable: 60 FPS with 2000+ processes (97.5% DOM reduction)
- 150ms search debounce to avoid per-keystroke O(n) filtering
- Test infrastructure: vitest + testing-library/svelte + happy-dom
- 69 tests across 4 test files (91% statement coverage, 96% line coverage)

## 4.0.6 (2026-03-05)

### Distribution
- Homebrew Cask formula for macOS desktop app
- Cross-platform release: .dmg (macOS), .msi (Windows), .deb + .AppImage (Linux)
- Universal web installer (`install-web.sh`)

### CI/CD
- Relax coverage threshold to 80%, exclude os_native.rs
- Fix formatting in killer.rs
- OS-aware killer tests, fix sleep termination on Linux

## 4.0.4 (2026-03-05)

### Performance
- Expand core resilience tests and add watcher micro-benchmark
- CLI integration tests and coverage pipeline with llvm-cov

## 4.0.2 (2026-03-05)

### Security
- Harden kill identity checks with macOS native memory parity
- IPC security, WCAG accessibility, architecture guide

## 4.0.0 (2026-03-05)

### Complete Rewrite
- Rust native core replacing Bash/AppKit (sysinfo, CDP, FFI)
- Tauri + Svelte 5 desktop app with reactive UI
- Rust CLI with clap for headless/server usage
- Cross-platform: macOS, Windows, Linux
- AI-powered optimization flow (OpenAI, Anthropic, OpenRouter)
- Native keychain integration for credential security
- Per-OS secure blocklists for critical process protection

---

## 1.2.0 (2026-03-03)

### Features
- Menu bar monitor (`MacmonStatusBar`) with live RAM/swap display
- MVVM refactor for testable model layer
- Homebrew formula for tap-based distribution
- Release workflow for automatic GitHub Releases on version tags

### Testing
- XCTest suite: 12 tests covering JSON parsing, filter/sort, grouping, selection

## 1.1.0 (2026-03-03)

### Features
- Orphan build daemon detection
- Per-process disk I/O metrics
- Export command (JSON/CSV) with peak tracking

### Security
- Code signature verification, input validation, bash 3.2 compatibility

### Testing
- BATS test suite: 46 tests with GitHub Actions CI

## 1.0.0 (2026-03-03)

Initial open-source release with background daemon, native AppKit process picker, CLI, YAML config, and LaunchAgent auto-start.
