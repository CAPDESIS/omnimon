# Motor de Automatizaciones (Automations Engine)

## Misión
Extender OmniMon v4 con capacidades de SRE (Site Reliability Engineering) proactivas, permitiendo a los usuarios configurar reglas condicionales que evalúen y actúen automáticamente sobre el rendimiento del sistema (ej. matar procesos desbocados).

## Arquitectura

1. **Frontend (Visual Builder - Svelte):**
   - **Componente:** `Automations.svelte`
   - **Responsabilidad:** Interfaz para crear, listar y eliminar reglas. Las reglas tienen un patrón (Regex/String), una métrica (RAM/CPU), un umbral (MB/%), un tiempo de sostenimiento y una acción (Alerta o Matar Proceso).

2. **Backend (Motor de Reglas Activas - Rust):**
   - **Módulo:** `watcher.rs` (O módulo dedicado)
   - **Responsabilidad:** Un hilo asíncrono en background (usando `tokio::spawn`) que evalúa periódicamente la lista de procesos contra las reglas activas.
   - **Ejecución:** Usa un `Arc<RwLock<Vec<Rule>>>` para mantener el estado seguro entre hilos sin bloquear a Tauri.

3. **Notificaciones Nativas:**
   - **Plugin:** `@tauri-apps/plugin-notification`
   - **Integración:** El motor de Rust emite un evento o llama directamente a la API de Tauri para invocar una alerta nativa al SO cuando se ejecuta una regla de `kill`.

## Flujo de Datos
- UI -> Llama comando Tauri `add_automation_rule` -> Rust guarda en memoria/disco.
- Rust Watcher -> Loop infinito cada X segs -> Filtra sysinfo -> Ejecuta acción -> Envía Notificación.
