# OmniMon v4.1.0 Roadmap

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
- [x] Multi-provider support: OpenAI, Anthropic, OpenRouter, Gemini
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
- [x] License: MIT + Commons Clause (free to use, cannot sell)
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

#### Network Traffic Monitoring
Add network I/O metrics per process (bytes sent/received). Similar to Activity Monitor's Network tab. Requires `sysinfo` network data or platform-specific APIs.

#### Multi-Language UI
Extend bilingual support from documentation to the actual application UI. Start with English and Spanish, with a language selector in settings. Use i18n framework or simple key-value translations.

#### Export Functionality
Export process list and tab data as JSON or CSV for analysis. Include peak tracking and historical data.

#### Custom Application Logo
Design and implement a proper OmniMon logo to replace the default Tauri icon. Needed for app store submissions and branding.

### Low Priority / Future Releases

#### SaaS Hosting
- Hosted version where users pay a subscription instead of managing their own API keys
- Domain and landing page for the service
- Managed AI provider integration (user just pays monthly, no API key needed)
- Self-hosting option for advanced users

#### App Store Distribution
- macOS App Store (requires Apple Developer account, notarization)
- Microsoft Store (Windows)
- Snap Store / Flathub (Linux)
- Free download with optional subscription for premium features

#### iOS / Android Support
Tauri v2 supports mobile targets. Explore feasibility of a mobile companion app for remote system monitoring.

#### Deep Network Analysis
Wireshark-like packet inspection integrated into the process view. Show which processes are sending data to the internet, to which endpoints, and how much traffic. AI can analyze patterns for suspicious activity.

#### CLI Enhancements
- Interactive TUI mode for headless servers
- JSON output for scripting
- Remote monitoring via SSH tunnel
- Daemon mode with alerts

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
- [x] Soporte multi-proveedor: OpenAI, Anthropic, OpenRouter, Gemini
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
- [x] Licencia: MIT + Commons Clause (uso libre, no se puede vender)
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

#### Monitoreo de Trafico de Red
Agregar metricas de I/O de red por proceso (bytes enviados/recibidos). Similar a la pestana Red del Monitor de Actividad. Requiere datos de red de `sysinfo` o APIs especificas de plataforma.

#### UI Multi-Idioma
Extender el soporte bilingue de la documentacion a la interfaz de la aplicacion. Comenzar con ingles y espanol, con selector de idioma en ajustes.

#### Funcionalidad de Exportacion
Exportar lista de procesos y datos de pestanas como JSON o CSV para analisis.

#### Logo Personalizado
Disenar e implementar un logo propio de OmniMon para reemplazar el icono predeterminado de Tauri. Necesario para tiendas de aplicaciones y branding.

### Baja Prioridad / Futuras Versiones

#### Hosting SaaS
- Version hosteada donde los usuarios pagan suscripcion en vez de gestionar sus propias API keys
- Dominio y landing page del servicio
- Opcion de auto-hosting para usuarios avanzados

#### Distribucion en Tiendas de Aplicaciones
- macOS App Store, Microsoft Store, Snap Store / Flathub
- Descarga gratuita con suscripcion opcional para funciones premium

#### Soporte iOS / Android
Tauri v2 soporta targets moviles. Explorar viabilidad de app companera movil.

#### Analisis Profundo de Red
Inspeccion de paquetes estilo Wireshark integrada en la vista de procesos.

#### Mejoras del CLI
- Modo TUI interactivo para servidores headless
- Salida JSON para scripting
- Monitoreo remoto via tunel SSH

#### Pruebas en Distribuciones Linux
Verificar compatibilidad en distros principales: Ubuntu, Fedora, Arch, Debian.
