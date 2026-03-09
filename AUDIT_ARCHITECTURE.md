# 🏗️ Auditoría Arquitectónica Integral — OmniMon
**Reporte de Arquitectura y Análisis de Codebase**

Generado por: **Agente 4 (Gemini-Arch)**
Worktree: `worktree-gemini-2`
Scope: Full Codebase (Core, CLI, TUI, Desktop/GUI)

---

## 1. 🗺️ Mapeo y Diagrama de Arquitectura End-to-End

El proyecto se estructura bajo un workspace de Rust monorepo que alimenta múltiples interfaces de usuario (Desktop, CLI, TUI).

```mermaid
graph TD
    %% Frontend Layer
    subgraph GUI [App Desktop Svelte 5]
        UI[Componentes UI - ProcessTable, AiCommandBar]
        Stores[Stores Globales - processes, alerts]
        IPC_Bridge[Capa IPC - @tauri-apps/api]
    end

    %% Tauri / IPC Layer
    subgraph TauriHost [Tauri Host Application]
        Commands[Tauri Commands]
        Automations[Automations Module]
        Plugins[Tauri Plugins]
    end

    %% CLI / TUI Layer
    subgraph Terminals [Terminal Interfaces]
        CLI[OmniMon CLI - clap]
        TUI[Terminal UI - ratatui/omnimon-tui]
    end

    %% Core Logic Layer (Shared)
    subgraph CoreBackend [Crate: core]
        Telemetry[Módulo de Telemetría - sysinfo]
        Killer[Módulo de Procesos/Killer]
        AI[Integración AI - ai.rs]
        Browser[Monitoreo de Tabs]
        Network[Monitoreo de Red]
    end

    %% Flujos de datos
    UI -->|Store Subscription| Stores
    Stores -->|setInterval 2s| IPC_Bridge
    IPC_Bridge -->|invoke()| Commands
    Commands -->|Delegación| CoreBackend
    CLI -->|Delegación| CoreBackend
    TUI -->|Delegación| CoreBackend
    AI -->|HTTP/REST| API[LLM Providers: OpenAI/Anthropic/Gemini/Ollama]
```

---

## 2. ⚖️ Consistencia CLI ↔ GUI

Existe una divergencia funcional entre el cliente de línea de comandos y la aplicación gráfica. 

| Funcionalidad / Comando | Presente en CLI | Presente en GUI | Estado / Comentarios |
| :--- | :---: | :---: | :--- |
| **Status/Métricas Básicas** | ✅ `status` | ✅ `get_metrics` | Mismas métricas (sysinfo). |
| **Matar Procesos** | ✅ `kill` | ✅ `kill_process(es)` | GUI soporta mutiple-kill vía Array. |
| **Monitoreo de Navegador** | ✅ `tabs` | ✅ `get_browser_tabs` | Totalmente consistente. |
| **Chat / AI Assistant** | ✅ `chat` | ✅ `ai_chat` | Integración base idéntica en core. |
| **Reglas Automáticas AI** | ❌ (Ausente) | ✅ `apply_ai_rules` | Exclusivo del frontend. |
| **Gestión de Red** | ❌ (Ausente) | ✅ `get_network_data`| CLI no expone visibilidad de red. |
| **Security Scan & Cloud** | ✅ `securityscan`, `cloud`| ❌ (Ausente) | Features de seguridad y sync a CrabNebula exclusivos de CLI. |

**Recomendaciones:**
- Implementar los comandos de `cloud` y `securityscan` en el frontend Tauri.
- Añadir el comando `network` al CLI para paridad con la GUI.

---

## 3. 📦 Análisis de Dependencias

### Backend (Rust / Cargo)
- **Crates repetidos o pesados**: Se usa `sysinfo` para lecturas en real-time. Es eficiente pero pesado de compilar.
- **Crypto y Auth**: Se usa `aes-gcm`, `sha2`, `keyring` (integrado bien con OS nativo). No hay redundancias graves.
- **Tauri Plugins**: Se emplean los plugins estándar (`store`, `notification`, `autostart`).

### Frontend (Desktop Svelte 5 / package.json)
- **Vite & Svelte 5**: Configuración moderna y extremadamente rápida. El uso de Runes (`$state`, `$derived`) está bien implementado.
- **Gráficos**: Uso de `lightweight-charts` es óptimo, evita el peso de d3.js o Chart.js.
- **Dependencias No Usadas/Redundantes**: `happy-dom` se incluye para testing, pero con la migración a testing más exhaustivo, no parece haber paquetes "zombies". 

---

## 4. 🔄 Flujo de Datos IPC (Tauri)

El polling loop frontend está configurado a `2000ms` por defecto y realiza peticiones iterativas hacia Tauri:
`setInterval -> fetchMetrics -> invoke('get_metrics') -> core::telemetry`

**Bottlenecks identificados en el IPC:**
1. **Falta de Rate Limiting en Comandos Pesados**: Comandos como `analyze_processes` envían arrays gigantescos al backend por IPC. Falta debounce nativo.
2. **Serialización**: Retornar colecciones grandes completas (`Vec<ProcessObservation>`) requiere un parsing JSON alto en la capa IPC.

---

## 5. ⚡ Optimización de Performance

1. **Virtual Scrolling Frontend (Procesos)**: Está implementado manualmente mediante `$derived` para aplanar los nodos y calcular el index visible (`Math.floor(scrollTop / ROW_HEIGHT)`). Esto es **muy eficiente** en Svelte 5.
2. **Bundle Size**: Vite, combinado con `lightweight-charts` en lugar de librerías de UI complejas, mantiene el bundle ligero. 
3. **Optimización Propuesta**: En vez de que Svelte haga polling a Tauri (Pull model), migrar a un **Push model** donde Rust emita un evento global a la ventana Tauri cada X segundos (`app.emit_all("metrics", data)`). Esto evitará la latencia del ida-y-vuelta en cada frame de 2s.

---

## 6. 🧠 Evaluación de AI Integration

El módulo `ai.rs` es robusto, soportando **OpenAI, Anthropic, Gemini, OpenRouter y Ollama**.

**Fortalezas:**
- ✅ Implementa in-memory caching (`OnceLock<RwLock<HashMap>>`) para evitar peticiones repetidas.
- ✅ Mecanismos anti-Prompt Injection básicos en Rust.
- ✅ Retries exponenciales para fallos de red (`MAX_RETRIES: 1`).
- ✅ Ollama validado como provider local (HTTP GET al servidor para validar que está vivo).

**Áreas de Mejora (Gaps detectados en `AiCommandBar.svelte` / `ai.rs`):**
- **Streaming UI**: Actualmente no hay un verdadero server-sent streaming de los tokens en la GUI desde los comandos Tauri. Las respuestas grandes provocan una pequeña congelación en el frontend. (Sugiero implementar Tauri events para streaming de chunks).
- **Rate Limits locales**: No hay control estricto que evite que el usuario machaque la API repetidamente en la GUI si hace spam del input.

---

### Conclusión Final
La arquitectura es limpia, aprovechando las máximas características del runtime de Rust (seguridad/rendimiento) y Svelte 5 (reactividad granular). Las mayores refactorizaciones deberían centrarse en:
1) Lograr **paridad de comandos** (Security/Cloud al GUI, Red al CLI).
2) Cambiar de Pull (setInterval en GUI) a **Push (Events de Tauri)** para telemetría.
3) Implementar **Streaming** de respuesta en los LLMs para mayor fluidez.
