# OmniMon v4.6.0 Roadmap

> Status 2026-10-13 (workspace 6.8.0): several "Pending" items below are
> DONE (network telemetry, multi-language UI, TUI, JSON output, landing
> page) — marked inline with evidence. Direct-OpenAI references are stale:
> the `openai` provider was removed fleet-wide; OpenAI models are reached
> via OpenRouter (`openai/...` slugs). License is pure MIT (`LICENSE`), not
> MIT+Commons Clause.

## Status Update (v4.6.0)

The sections below started as a v4.1 roadmap and include historical entries.
Current implementation status for core security/network stack:

- [x] Native network telemetry backends: libpcap (macOS), WinDivert (Windows), eBPF/aya (Linux)
- [x] Per-process network throughput and recent connection events (IP/port/protocol)
- [x] SuperProcess aggregation enriched with network throughput
- [x] MITRE ATT&CK mapping for suspicious memory/network behavior
- [x] NIST 800-53 Security Heartbeat generation + encrypted persistence (AES-256-GCM)
- [x] Dynamic AI Rules Engine with versioned JSON schema (`schema_version: 1`)
- [x] Tauri IPC bridge: `apply_ai_rules`, `get_ai_rules_schema`, and `security-alert` event emission

Reference files:

- `v4/crates/core/AI_RULES_SCHEMA.v1.json`
- `v4/crates/core/src/rules_engine.rs`
- `v4/crates/core/src/network.rs`
- `v4/crates/core/src/watcher.rs`

## Implemented Features (v4.1.0)

### Core Process Monitor
- [x] Real-time process table with Name, Detail, Group, RAM, CPU, Uptime, PID, State columns
- [x] Virtual scroll rendering 2000+ processes at 60 FPS
- [x] Process search/filter by name, PID, or group (150ms debounce)
- [x] Process grouping (collapsible groups with aggregate RAM/CPU)
- [x] Browser process aggregation (all Chrome helpers grouped under "Chrome" with total RAM)
- [x] Select all / select none / multi-select processes
- [x] Kill selected processes with confirmation dialog
- [x] Kill single process with confirmation dialog
- [x] Process detail modal (Cmd+I or double-click)
- [x] Column visibility customization (show/hide columns)
- [x] Column reorder (drag up/down in settings)
- [x] Keyboard shortcuts: Cmd+F (search), Cmd+I (detail), Cmd+=/- (zoom), Del (close)

### Browser Tab Management
- [x] Live browser tab listing (Chrome, Safari, Brave, Edge, Arc)
- [x] Tab display with title, domain, and URL
- [x] Close individual tabs with confirmation
- [x] Select multiple tabs and close selected with confirmation
- [x] Close all tabs for a browser with confirmation
- [x] Click tab title to focus/navigate to it in the browser
- [x] Tab search/filter (shared with process search)
- [x] Browser sections with tab count and RAM usage
- [x] Sticky browser headers and column headers when scrolling
- [x] Resize divider for tab panel height (persisted across restarts)

### AI Integration
- [x] Multi-provider support: OpenRouter, Anthropic, Gemini, Ollama (direct OpenAI removed fleet-wide; `openai/...` models via OpenRouter)
- [x] AI Analyze button in toolbar (process optimization suggestions)
- [x] AI profile selection: General, Developer, Gaming, Battery Saver
- [x] Ask AI in process detail modal (contextual analysis with tab info)
- [x] AI suggestions panel with per-suggestion kill button
- [x] API key validation before saving (test request to provider)
- [x] API key whitespace trimming
- [x] Secure credential storage via native OS keychain

### Settings & Preferences (Persisted)
- [x] Font size A+/A- with actual UI scaling (all components use calc())
- [x] Max font size raised to 24 for accessibility
- [x] Light/Dark/Auto theme toggle
- [x] Configurable IDLE threshold (CPU% to define idle, 0.1-10.0)
- [x] AI provider and model configuration
- [x] Column visibility and order
- [x] Tab panel height
- [x] All preferences saved to disk and restored on launch

### Security
- [x] Per-OS immutable blocklists (macOS/Windows/Linux critical processes)
- [x] Native keychain for API key storage (never plain text)
- [x] IPC type validation on every response
- [x] AppleScript RCE mitigation (positional args, no string interpolation)
- [x] CDP WebSocket path traversal prevention
- [x] User-space execution only (no sudo/root)
- [x] MITRE ATT&CK compliance (T1059, T1552, T1548.002)
- [x] Automated CVE scanning via cargo-audit and Dependabot

### Documentation
- [x] Bilingual documentation (English + Spanish) for README, CONTRIBUTING, ARCHITECTURE, CHANGELOG, SECURITY
- [x] License: MIT (`LICENSE`; the old "MIT + Commons Clause" line was stale)
- [x] No AI attribution in repository

### Testing
- [x] 182 frontend tests (vitest + testing-library/svelte)
- [x] 57 Rust tests (unit + integration + mock servers)
- [x] Zero build warnings (vite build)
- [x] CI/CD: cargo fmt, clippy, test on macOS/Windows/Linux

---

## Pending / Future Features

### High Priority

#### Safari Tab Detection Debugging
Safari tabs are fully implemented in the Rust backend (`browser.rs`) but may not appear in the frontend for some users. Error logging has been added to the tab cache refresh. Common causes: AppleScript permissions not granted (System Preferences > Privacy & Security > Automation), Safari's "Allow JavaScript from Apple Events" not enabled (Safari > Develop menu).

#### AI-Powered Contextual Tab Closing
Allow users to describe what they were working on (e.g., "I was studying React") and have the AI analyze all open tabs to suggest which ones to close. The AI sees tab titles and URLs, groups them by relevance, and presents a selectable list for user confirmation.

#### Window Resizing
Allow the main window to be resized horizontally so users can see truncated text (domains, URLs, process names) without text overlapping.

#### Text Overlap Prevention
Audit all UI components for text overflow issues. Ensure `text-overflow: ellipsis` and proper `min-width` constraints prevent text from overlapping adjacent elements, especially at small font sizes or narrow widths.

### Medium Priority

#### Tab Enrichment
- Show how long each tab has been open
- Show if the tab is actively loading or idle
- Show estimated memory usage per tab (when available via CDP)

#### Network Traffic Monitoring — DONE (v6.2.0)
Per-process network throughput, live connections, DNS/GeoIP, alerts, and a network dashboard (`NetworkDashboard.svelte`, `omnimon network`). Remaining: deeper packet forensics (see "Deep Network Analysis").

#### Multi-Language UI — DONE
EN/ES UI via `src/lib/i18n.ts` + `src/locales/` with a language selector in settings. Remaining: a third locale was never requested; Zombie Killer strings would need translation if one is added.

#### Export Functionality
Export process list and tab data as JSON or CSV for analysis. Include peak tracking and historical data.

#### Custom Application Logo — VERIFY
An icon set exists (`v4/apps/desktop/src-tauri/icons/`: `icon.png/.icns/.ico`, referenced by `tauri.conf.json`) and the landing uses a gear+cyan mark. Still open: confirm the desktop icons are the OmniMon brand (not the Tauri default) before closing this.

### Low Priority / Future Releases

#### SaaS Hosting — PARTIAL
- [x] Domain and landing page for the service (`omnimon_landing/`, omnimon.com.mx)
- [ ] Hosted version where users pay a subscription instead of managing their own API keys (still pending)
- [ ] Managed AI provider integration (user just pays monthly, no API key needed) (still pending)
- [ ] Self-hosting option for advanced users (still pending)

#### App Store Distribution
- macOS App Store (requires Apple Developer account, notarization)
- Microsoft Store (Windows)
- Snap Store / Flathub (Linux)
- Free download with optional subscription for premium features

#### iOS / Android Support
Tauri v2 supports mobile targets. Explore feasibility of a mobile companion app for remote system monitoring.

#### Deep Network Analysis — PARTIAL
Connection-level analysis is done (capture backends, per-process throughput, endpoints, DNS/GeoIP, MITRE mapping, AI presets). Still pending: Wireshark-like per-packet payload inspection in the process view (DPI reads metadata only, by design).

#### CLI Enhancements — PARTIAL
- [x] Interactive TUI mode for headless servers (`omnimon tui`, `crates/tui/`)
- [x] JSON output for scripting (`--format json` on `status`/`network`)
- [ ] Remote monitoring via SSH tunnel (still pending)
- [ ] Daemon mode with alerts (still pending; automations engine covers in-app alerts)

#### Linux Distribution Testing
Verify compatibility across major distros: Ubuntu, Fedora, Arch, Debian. Document any distro-specific setup requirements (libwebkit2gtk versions, Secret Service availability).

---

# Hoja de Ruta de OmniMon v4.1.0 (Espanol)

## Funcionalidades Implementadas (v4.1.0)

### Monitor de Procesos
- [x] Tabla de procesos en tiempo real con columnas: Nombre, Detalle, Grupo, RAM, CPU, Tiempo, PID, Estado
- [x] Scroll virtual renderizando 2000+ procesos a 60 FPS
- [x] Busqueda/filtro de procesos por nombre, PID o grupo (debounce 150ms)
- [x] Agrupacion de procesos (grupos colapsables con RAM/CPU agregado)
- [x] Agregacion de procesos del navegador (todos los helpers de Chrome agrupados bajo "Chrome" con RAM total)
- [x] Seleccionar todo / deseleccionar / multi-seleccion de procesos
- [x] Cerrar procesos seleccionados con dialogo de confirmacion
- [x] Cerrar proceso individual con dialogo de confirmacion
- [x] Modal de detalle de proceso (Cmd+I o doble clic)
- [x] Personalizacion de visibilidad de columnas (mostrar/ocultar)
- [x] Reordenamiento de columnas (mover arriba/abajo en ajustes)
- [x] Atajos de teclado: Cmd+F (buscar), Cmd+I (detalle), Cmd+=/- (zoom), Del (cerrar)

### Gestion de Pestanas del Navegador
- [x] Listado de pestanas en vivo (Chrome, Safari, Brave, Edge, Arc)
- [x] Visualizacion de pestanas con titulo, dominio y URL
- [x] Cerrar pestanas individuales con confirmacion
- [x] Seleccionar multiples pestanas y cerrar seleccionadas con confirmacion
- [x] Cerrar todas las pestanas de un navegador con confirmacion
- [x] Clic en titulo de pestana para navegar/enfocar en el navegador
- [x] Busqueda/filtro de pestanas (compartido con busqueda de procesos)
- [x] Secciones por navegador con conteo de pestanas y uso de RAM
- [x] Encabezados fijos (sticky) al hacer scroll
- [x] Divisor redimensionable para panel de pestanas (persistido entre reinicios)

### Integracion con IA
- [x] Soporte multi-proveedor: OpenRouter, Anthropic, Gemini, Ollama (OpenAI directo eliminado; modelos `openai/...` vía OpenRouter)
- [x] Boton "AI Analyze" en barra de herramientas (sugerencias de optimizacion)
- [x] Seleccion de perfil de IA: General, Desarrollador, Gaming, Ahorro de Bateria
- [x] "Ask AI" en modal de detalle (analisis contextual con info de pestanas)
- [x] Panel de sugerencias de IA con boton de cerrar por sugerencia
- [x] Validacion de API key antes de guardar (peticion de prueba al proveedor)
- [x] Limpieza de espacios en API key
- [x] Almacenamiento seguro de credenciales via keychain nativo del SO

### Ajustes y Preferencias (Persistidas)
- [x] Tamano de fuente A+/A- con escalado real de UI (todos los componentes usan calc())
- [x] Tamano maximo de fuente aumentado a 24 para accesibilidad
- [x] Selector de tema Claro/Oscuro/Automatico
- [x] Umbral IDLE configurable (% de CPU para definir inactivo, 0.1-10.0)
- [x] Configuracion de proveedor y modelo de IA
- [x] Visibilidad y orden de columnas
- [x] Altura del panel de pestanas
- [x] Todas las preferencias guardadas en disco y restauradas al iniciar

### Seguridad
- [x] Blocklists inmutables por SO (procesos criticos de macOS/Windows/Linux)
- [x] Keychain nativo para almacenamiento de API keys (nunca en texto plano)
- [x] Validacion de tipos IPC en cada respuesta
- [x] Mitigacion de RCE en AppleScript (argumentos posicionales, sin interpolacion)
- [x] Prevencion de Path Traversal en WebSocket CDP
- [x] Ejecucion en espacio de usuario (sin sudo/root)
- [x] Cumplimiento MITRE ATT&CK (T1059, T1552, T1548.002)
- [x] Escaneo automatizado de CVEs via cargo-audit y Dependabot

### Documentacion
- [x] Documentacion bilingue (ingles + espanol) para README, CONTRIBUTING, ARCHITECTURE, CHANGELOG, SECURITY
- [x] Licencia: MIT (`LICENSE`; la línea vieja "MIT + Commons Clause" estaba desactualizada)
- [x] Sin atribucion de IA en el repositorio

### Pruebas
- [x] 182 pruebas frontend (vitest + testing-library/svelte)
- [x] 57 pruebas Rust (unitarias + integracion + servidores mock)
- [x] Cero advertencias de build (vite build)
- [x] CI/CD: cargo fmt, clippy, test en macOS/Windows/Linux

---

## Funcionalidades Pendientes / Futuras

### Alta Prioridad

#### Depuracion de Deteccion de Pestanas de Safari
Las pestanas de Safari estan completamente implementadas en el backend Rust (`browser.rs`) pero pueden no aparecer para algunos usuarios. Se agrego registro de errores al refresco de cache de pestanas. Causas comunes: permisos de AppleScript no otorgados (Preferencias del Sistema > Privacidad y Seguridad > Automatizacion), "Permitir JavaScript de Apple Events" no habilitado en Safari (menu Desarrollador de Safari).

#### Cierre Contextual de Pestanas con IA
Permitir a los usuarios describir en que estaban trabajando (ej. "Estaba estudiando React") y que la IA analice todas las pestanas abiertas para sugerir cuales cerrar. La IA ve titulos y URLs, los agrupa por relevancia, y presenta una lista seleccionable para confirmacion del usuario.

#### Redimensionamiento de Ventana
Permitir que la ventana principal se redimensione horizontalmente para que los usuarios puedan ver texto truncado (dominios, URLs, nombres de proceso) sin superposicion de texto.

#### Prevencion de Superposicion de Texto
Auditar todos los componentes de UI para problemas de desbordamiento de texto. Asegurar `text-overflow: ellipsis` y restricciones `min-width` adecuadas para prevenir que el texto se superponga a elementos adyacentes.

### Prioridad Media

#### Enriquecimiento de Pestanas
- Mostrar cuanto tiempo lleva abierta cada pestana
- Mostrar si la pestana esta cargando activamente o inactiva
- Mostrar uso estimado de memoria por pestana (cuando este disponible via CDP)

#### Monitoreo de Trafico de Red — HECHO (v6.2.0)
Throughput por proceso, conexiones vivas, DNS/GeoIP, alertas y dashboard de red (`NetworkDashboard.svelte`, `omnimon network`). Resta: forense de paquetes más profundo (ver "Analisis Profundo de Red").

#### UI Multi-Idioma — HECHO
UI EN/ES vía `src/lib/i18n.ts` + `src/locales/` con selector de idioma en ajustes. Resta: nunca se pidió un tercer idioma; habría que traducir las cadenas del Zombie Killer si se suma uno.

#### Funcionalidad de Exportacion
Exportar lista de procesos y datos de pestanas como JSON o CSV para analisis.

#### Logo Personalizado — VERIFICAR
Existe un set de iconos (`v4/apps/desktop/src-tauri/icons/`: `icon.png/.icns/.ico`, referenciado por `tauri.conf.json`) y el landing usa marca gear+cyan. Aún abierto: confirmar que los iconos desktop son la marca OmniMon (no el default de Tauri) antes de cerrar esto.

### Baja Prioridad / Futuras Versiones

#### Hosting SaaS — PARCIAL
- [x] Dominio y landing page del servicio (`omnimon_landing/`, omnimon.com.mx)
- [ ] Version hosteada donde los usuarios pagan suscripcion en vez de gestionar sus propias API keys (aún pendiente)
- [ ] Opcion de auto-hosting para usuarios avanzados (aún pendiente)

#### Distribucion en Tiendas de Aplicaciones
- macOS App Store, Microsoft Store, Snap Store / Flathub
- Descarga gratuita con suscripcion opcional para funciones premium

#### Soporte iOS / Android
Tauri v2 soporta targets moviles. Explorar viabilidad de app companera movil.

#### Analisis Profundo de Red — PARCIAL
El análisis a nivel conexión está hecho (captura, throughput por proceso, endpoints, DNS/GeoIP, MITRE, presets IA). Aún pendiente: inspección de paquetes estilo Wireshark en la vista de procesos (DPI lee solo metadatos, por diseño).

#### Mejoras del CLI — PARCIAL
- [x] Modo TUI interactivo para servidores headless (`omnimon tui`, `crates/tui/`)
- [x] Salida JSON para scripting (`--format json` en `status`/`network`)
- [ ] Monitoreo remoto via tunel SSH (aún pendiente)

#### Pruebas en Distribuciones Linux
Verificar compatibilidad en distros principales: Ubuntu, Fedora, Arch, Debian.
