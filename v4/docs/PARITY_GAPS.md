# CLI ↔ GUI Parity Gaps

Documento generado por la auditoría de paridad CLI/GUI.
Input para el Sprint 2 (agente frontend).

## Funcionalidades en CLI que faltan en GUI

### 1. Security Scan (`omnimon security-scan`)
- **CLI:** Escanea procesos contra base de datos CVE, genera heartbeat NIST, persiste reporte encriptado.
- **Comando:** `omnimon security-scan [--cve-db <path>]`
- **Core usado:** `core::audit::audit_processes_against_cves`, `core::audit::build_security_heartbeat`, `core::audit::persist_encrypted_security_heartbeat`
- **Componente GUI sugerido:** `SecurityScanView.svelte` — botón "Scan Now" que invoca un nuevo comando IPC `run_security_scan`, muestra findings en tabla con severity badges, y permite exportar el reporte encriptado.
- **IPC necesario:** `run_security_scan(cve_db: Option<String>) -> SecurityScanResult`

### 2. Doctor / Health Check (`omnimon doctor`)
- **CLI:** Verifica OS, arquitectura, drivers de red (libpcap/WinDivert/eBPF), acceso al keyring.
- **Comando:** `omnimon doctor`
- **Core usado:** Verificaciones directas del OS (paths, keyring).
- **Componente GUI sugerido:** Sección en Settings o modal "System Health" con checklist visual (iconos verde/rojo por cada check).
- **IPC necesario:** `run_health_check() -> Vec<HealthCheckItem>`

### 3. Auth Login (`omnimon auth login`)
- **CLI:** Guarda API Key de CrabNebula en keyring del OS.
- **Comando:** `omnimon auth login <key>`
- **Core usado:** `keyring::Entry` directamente.
- **Componente GUI sugerido:** Campo en Settings > Cloud con input para CN API Key, que invoque `save_cloud_key`.
- **Nota:** Ya existe `save_cloud_key` / `get_cloud_key` en el GUI, pero no hay UI visible para login. Solo falta el componente frontend.

### 4. Cloud Sync (`omnimon cloud sync`)
- **CLI:** Sube reporte de seguridad encriptado a CrabNebula Cloud.
- **Comando:** `omnimon cloud sync --report-path <path>`
- **Core usado:** Verificación de keyring + upload simulado.
- **Componente GUI sugerido:** Botón "Sync to Cloud" en `SecurityReportView.svelte` que tome el último reporte y lo suba via IPC.
- **IPC necesario:** `cloud_sync_report(report_path: String) -> Result<(), String>`

## Funcionalidades en GUI que no aplican al CLI

Estas funcionalidades están correctamente excluidas del CLI por ser inherentes a la UI:

| Funcionalidad | Razón de exclusión |
|---|---|
| Window visibility (`get_window_visible`) | Control de ventana desktop |
| System tray menu | Componente visual desktop |
| Automations engine (CPU/RAM thresholds) | Requiere Tauri Store + notifications nativas; el CLI tiene `rules` como alternativa |
| Plugin system (Lua) | Requiere Tauri AppHandle para dirs + persistencia; posible extensión futura al CLI |

## Estado de paridad post-fix

| Funcionalidad | CLI | GUI |
|---|---|---|
| Monitoreo (status/metrics) | `status` | `get_metrics` |
| Kill proceso | `kill` | `kill_process` / `kill_processes` |
| Browser tabs | `tabs` | `get_browser_tabs` / `close_browser_tab` / `focus_browser_tab` |
| AI optimize | `optimize` | `analyze_processes` |
| AI chat | `chat` | `ai_chat` / `analyze_context` |
| API key mgmt | `apikey` | `save_ai_config` / `check_api_key` / `validate_api_key` |
| Settings | `settings` | Tauri Store |
| **Network telemetry** | `network` | `get_network_data` |
| **AI security rules** | `rules` | `apply_ai_rules` / `get_ai_rules_schema` |
| Security scan | `security-scan` | **PENDIENTE** |
| Doctor | `doctor` | **PENDIENTE** |
| Auth | `auth` | **PENDIENTE (UI)** |
| Cloud sync | `cloud` | **PENDIENTE** |
| Automations | N/A (usar `rules`) | `get/add/remove_automation_rules` |
| Plugins (Lua) | N/A | `list/install/set_enabled/remove_plugin` |
