# macmon v3 -> v4 Gap Analysis (GPT)

## Scope

Comparativa funcional uno-a-uno entre:

- **v3 (actual en producción):** Bash + Swift/AppKit (`lib/`, `scripts/`, `src/gui/`)
- **v4 (arquitectura nueva):** Rust Core + Tauri + Svelte (`v4/crates/`, `v4/apps/desktop/`)

Este análisis se centra en las 4 áreas críticas solicitadas: introspección de navegadores, IA Human-in-the-Loop, UX/paridad visual y hardening/blocklist.

## Evidence Base (files revisados)

- v3: `lib/macmon-core.sh`, `lib/macmon-security.sh`, `scripts/chrome-tabs.sh`, `scripts/graceful-quit.sh`, `src/gui/AIService.swift`, `src/gui/PreferencesWindow.swift`, `src/gui/ProcessPicker.swift`, `src/gui/ProcessPickerModel.swift`, `src/gui/MacmonStatusBar.swift`
- v4: `v4/crates/core/src/{metrics.rs,watcher.rs,killer.rs,os_native.rs}`, `v4/apps/desktop/src-tauri/src/lib.rs`, `v4/apps/desktop/src/{App.svelte,stores/processes.ts,components/*.svelte}`

## Comparative Matrix

| Feature | Estado v3 | Estado v4 | Solución Propuesta para Multiplataforma |
|---|---|---|---|
| Chrome tabs: enumeración título+URL | `scripts/chrome-tabs.sh` usa AppleScript (`Google Chrome` windows/tabs), devuelve `id,title,url` | No existe proveedor real de tabs; UI solo ve procesos | Crear `core::browser` con trait `TabProvider` y adapters por OS (`macos_applescript`, `cdp`). |
| Chrome tab -> PID mapping | v3 extrae `--renderer-client-id` y lo cruza con tab id (`lib/macmon-core.sh`) | No mapeo tab/PID; aparece `Google Chrome Helper` | En Rust: parsear cmdline (`sysinfo::Process::cmd`) para renderer id + join con targets CDP/AppleScript. |
| Close tab individual (graceful) | v3 cierra por URL/ID vía AppleScript (`graceful-quit.sh`), fallback `SIGTERM` | v4 mata PID del helper (`kill_process_safe`) sin semántica de tab | Implementar `close_tab_safe(tab_ref)` en `core::browser`; usar `Target.closeTarget` (CDP) y AppleScript en macOS. |
| Vista “Chrome Tab” con contexto humano | v3 `ProcessPicker` enriquece `detail/group/cwd` con título, dominio, URL | `ChromeTabManager.svelte` no recibe tabs reales; además filtra `group === "Browser"` pero backend manda `group: ""` | Definir tipo IPC `BrowserTabEntry`; separar tabla de tabs de tabla de procesos crudos. |
| Safari tabs | No implementado de forma completa en v3 | No implementado | macOS-only adapter: AppleScript Safari; en Win/Linux no aplica. UI con capability flag por OS. |
| Browser introspection cross-platform | v3 depende de AppleScript (macOS) | Inexistente | Estrategia híbrida: 1) CDP universal (Chrome/Edge/Brave), 2) AppleScript fallback en macOS, 3) opcional extensión browser para cero-config. |
| Privacidad URL -> IA | v3 tiene toggle `Allow sending browser URLs...` (default OFF), `AIService` elimina URL del payload | v4 no tiene IA ni toggle | Mantener mismo contrato: `allow_browser_urls=false` por defecto; enviar solo `title/domain` salvo opt-in explícito. |
| Smart Optimize (flujo completo) | v3 botón + spinner + sugerencias con razones + apply/review | No existe en v4 | Agregar `core::ai` + comando Tauri `smart_optimize`; modal HITL (sugerencias, razones, confirmación). |
| Proveedores de IA | v3 soporta OpenAI/Anthropic/OpenRouter/Gemini | No existe | Reusar misma matriz de proveedores con `reqwest` y builders por provider/model. |
| API key storage seguro | v3 usa Keychain (`Security.framework`) | No existe | `keyring` crate en Rust/Tauri (Keychain, Credential Manager, Secret Service). |
| Prompt contract JSON estricto | v3 exige formato `{"suggestions":[{"pid", "reason"}]}` y backward compat | No existe | Definir schema estable y parser robusto en `core::ai::parse_suggestions`. |
| Sanitización de sugerencias IA | v3 filtra PID inválido/no vivo/protegido y dedup | Parcial: blocklist existe en killer, pero no IA | Reusar `killer` + verificación de vida PID antes de exponer recomendaciones. |
| AI tab summary | v3 resume tabs con providers y privacidad URL | No existe | Añadir comando `summarize_tabs(tabs, settings)` tras tener `TabProvider`. |
| Tabla principal: columnas avanzadas | v3 incluye Name, Group, Detail, RAM, CPU, Uptime, PID, Disk R/W, Idle, CWD, State, TTY | v4 muestra subset mínimo (sin Detail/Uptime/CWD/TTY/Disk) | Extender DTO `ProcessEntry` y tabla Svelte para columnas parity + toggles de visibilidad. |
| Grouping (headers colapsables) | v3 agrupa por `group`, con headers expand/collapse | v4 no tiene agrupación real | Implementar grouping en store/UI (group rows + collapse state). |
| Inspector / detalle (Cmd+I) | v3 modal de detalle por proceso + atajo Cmd+I + doble clic | v4 no tiene panel de detalle | Añadir DetailsDrawer/Modal con PID, cmdline, uptime, cwd/url, idle, state, disk io. |
| Search UX (debounce y alcance) | v3 debounce + busca por name/pid/detail/cwd/group | v4 busca name/pid/group | Añadir debounce 200ms y ampliar búsqueda a detail/cwd/exec_name/state. |
| Quick actions de selección | v3: idle, stale, top RAM, top CPU, all/none | v4 solo all/none | Portar estrategias al store (`selectIdle`, `selectStale`, `selectTopRam`, `selectTopCpu`). |
| Command palette / operaciones | v3: export, status, update, restart daemon, config GUI, tabs, etc. | v4 muy básico (kill + polling) | Añadir menú acciones y comandos Tauri equivalentes por plataforma/capability. |
| Menu bar / tray richness | v3 tray con RAM/swap/procesos, perfiles, export, prefs | v4 tray con solo Show/Quit | Extender tray dinámico con stats y accesos rápidos. |
| Perfil y reglas de configuración | v3 perfiles + editor de reglas (thresholds/intervals) | v4 no tiene sistema de perfiles/config UI | Definir `settings` persistentes + profile presets compartidos entre CLI/UI. |
| Datos de swap | v3 calcula swap real y lo muestra | v4 backend devuelve `swap_used_mb: 0` fijo | Implementar swap cross-platform en `core` (`sysinfo`/OS APIs) y exponerlo en watcher. |
| Estado/idle fidelity | v3 usa `ps`/umbrales y state real | v4 state simplificado (`R`/`S`) por heurística | Exponer estado real por plataforma y centralizar regla idle en core configurable. |
| Blocklist inmutable (procesos críticos) | v3 lista extensa (~20+) + capa extra en `macmon-security.sh` | v4 lista reducida (10) | Migrar superset de v3 + listas específicas por OS (Windows critical + Linux PID1/systemd). |
| Verificación anti-spoof de procesos sistema | v3 valida Apple-signed (`codesign`) para nombres sensibles | v4 no tiene verificación de firma/ruta | Añadir validación por OS: macOS codesign/team id, Windows image path + signer, Linux owner/path/cgroup heuristics. |
| Re-check de blocklist en escalación SIGKILL | v3 revalida antes de SIGKILL de stragglers | v4 valida al inicio y fallback nativo, sin fase de grace/escalation loop equivalente | Implementar pipeline kill en 2 fases (TERM->wait->KILL) con re-check de identidad + blocklist. |
| PID reuse protection | v3 `verify_pid` compara nombre esperado antes de matar | v4 no verifica identidad esperada externa | API kill debe aceptar `expected_name/hash` opcional y verificar antes de actuar. |
| Protección de comandos peligrosos | v3 bloquea patrones (`rm -rf`, `sudo`, etc.) en capa shell | v4 no ejecuta shell dinámico (riesgo menor) | Mantener principio: sin ejecución de comandos arbitrarios desde UI; sólo comandos tipados IPC. |
| Cierre graceful de apps GUI | v3 AppleScript `quit` para `.app` | v4 solo kill PID | Añadir estrategia per-OS: macOS Apple Events, Windows `WM_CLOSE`, Linux SIGTERM + timeout. |
| Telemetría de decisiones (AI/manual) | v3 guarda snapshots y motivos | v4 no equivalente | Añadir `telemetry` opcional anon/local para auditoría de acciones y UX. |
| i18n/paridad textos | v3 en/es con Localizable.strings extensa | v4 UI hardcoded EN | Adoptar i18n desde inicio (en/es) y llaves de producto como en v3. |

## Focused Findings by Critical Area

### 1) Introspección de Navegadores

- **Brecha principal:** v4 no tiene canal de tabs, solo procesos crudos.
- **Recomendación de implementación multiplataforma (orden de robustez):**
  1. **CDP provider (Windows/Linux/macOS)** para Chrome/Edge/Brave.
  2. **AppleScript provider (macOS)** como fallback de cero-config cuando CDP no está disponible.
  3. **Browser extension relay (opcional)** para escenarios enterprise donde no se permite `--remote-debugging-port`.

### 2) Integración de IA Human-in-the-Loop

- **Brecha principal:** v4 no implementa AI settings, key storage, ni flujo HITL.
- **Implementación segura propuesta:**
  - `core::ai` (prompting, providers, parsing, sanitización)
  - `keyring` para credenciales por OS
  - toggles de privacidad (URL opt-in)
  - en UI, confirmación explícita antes de matar.

### 3) Gestión Visual y UX

- **Brecha principal:** v4 tiene UI funcional mínima, pero lejos de la paridad de v3 (detalles, grouping, comandos, inspector, shortcuts).
- **Prioridad de cierre de brechas:**
  1. Details panel + Cmd+I + double-click
  2. Grouping colapsable
  3. Tabs manager real con título/URL
  4. Smart optimize UI.

### 4) Hardening y Blocklist

- **Brecha principal:** v4 trae blocklist base y fallback nativo, pero perdió defensas profundas de v3 (verificación de firma Apple, re-check en escalación, identidad PID esperada, lista más extensa).
- **Recomendación:** migrar la política de seguridad de v3 a `core::policy` con reglas por plataforma, manteniendo lista inmutable + defensas anti-spoof + kill pipeline en fases.

## Veredicto Ejecutivo

v4 ya aporta una base sólida (core Rust, watcher asíncrono, kill nativo/fallback), pero **aún no alcanza paridad funcional 1:1 con v3** en las áreas que más percibe el usuario final:

1. **Contexto de navegador (tabs reales)**
2. **Smart Optimize con IA y control humano**
3. **UX de operaciones avanzadas (details/grouping/comandos)**
4. **Hardening completo heredado de v3**

Para recuperar la experiencia de v3 y escalar a multiplataforma, el orden recomendado es:

1) `browser` provider stack (CDP + AppleScript fallback),
2) `ai` stack seguro (keyring + privacy by default),
3) parity UX (details/grouping/shortcuts),
4) security policy migration completa.
