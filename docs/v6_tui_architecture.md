# OmniMon V6.0 — TUI Architecture (Ratatui + Crossterm)

## Visión General

Motor TUI de alto rendimiento que provee una interfaz gráfica de terminal estilo htop/btop
con métricas del sistema en tiempo real y un panel interactivo de chat con IA. Se ejecuta
mediante `omnimon tui`.

## Crate: `omnimon-tui` (`v4/crates/tui/`)

### Dependencias

| Crate | Versión | Propósito |
|-------|---------|-----------|
| `ratatui` | 0.29 | Framework de widgets TUI (gauges, tables, paragraphs) |
| `crossterm` | 0.28 | Backend de terminal multiplataforma (raw mode, eventos) |
| `core` | workspace | Motor de telemetría compartido con la app Tauri |
| `tokio` | 1.x | Runtime async para las llamadas AI (hilo de chat) |

### Módulos

```
src/
├── lib.rs      # Entry point público: run() → configura terminal + lanza event loop
├── app.rs      # Estado de la aplicación (App struct), sorting, selección
├── event.rs    # Event loop: polling de teclado (2 Hz), dispatch AI chat
└── ui.rs       # Rendering ratatui: header, tabla de procesos, panel de chat
```

## Arquitectura de Concurrencia

```
┌─────────────────────────────────────────────────────────┐
│  Hilo Principal (TUI)                                   │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Event Loop (500ms tick)                          │    │
│  │  1. crossterm::event::poll() — lee teclado       │    │
│  │  2. app.refresh() — lee SystemState (RwLock)     │    │
│  │  3. terminal.draw() — renderiza frame            │    │
│  │  4. poll_ai_response() — revisa canal mpsc       │    │
│  └─────────────────────────────────────────────────┘    │
│                          ↑ RwLock::read (non-blocking)  │
└──────────────────────────│──────────────────────────────┘
                           │
┌──────────────────────────│──────────────────────────────┐
│  Hilo Watcher (core::watcher)                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Tick cada 2 segundos                             │    │
│  │  1. System::refresh_all()                        │    │
│  │  2. NetworkTelemetryEngine::sample()             │    │
│  │  3. Evaluación de reglas dinámicas               │    │
│  │  4. Arc<RwLock<SystemState>> ← write lock        │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│  Hilo AI Chat (spawned per-request)                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │ tokio::runtime::Builder::new_current_thread()    │    │
│  │  → core::ai::chat_with_tools()                   │    │
│  │  → mpsc::Sender<String> → respuesta al main     │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Sincronización sin Race Conditions

| Recurso | Mecanismo | Garantía |
|---------|-----------|----------|
| `SystemState` | `Arc<RwLock<>>` (OnceLock global) | Múltiples lectores, 1 escritor. TUI lee, watcher escribe. |
| AI Chat response | `std::sync::mpsc::channel` | Transferencia segura entre hilo AI y main. `try_recv()` non-blocking. |
| AI request state | `thread_local! RefCell` | Sólo accesible desde el hilo principal (single-threaded TUI). |
| Watcher start | `AtomicBool` (SeqCst) | Garantiza un solo spawn del hilo watcher. |

## Layout de la Interfaz

```
┌──────────── OmniMon v6.0 — TUI ─────────────────┐
│ CPU ████████░░ 67%  MEM █████░░ 62%  NET ↓1.5M/s│
│                              Procs: 342  Swap: 1G│
├──────── Processes [MEM ↓] ───────────────────────┤
│ ►   PID  NAME           CPU%   MEMORY  NET  NRG │
│    1234  Chrome          12.3  1.5 GB  1.2M  4.2│
│    5678  node            45.1  890 MB   0 B  3.8│
│    9012  Xcode            8.7  2.1 GB   0 B  2.1│
│    ...                                           │
├──────── AI Chat ─────────────────────────────────┤
│ sys: OmniMon AI Chat                             │
│ ► ¿qué proceso consume más RAM?                  │
│ AI: Chrome lidera con 1.5 GB distribuidos en...  │
│ ❯ █                                              │
└──────────────────────────────────────────────────┘
```

## Controles de Teclado

| Tecla | Acción |
|-------|--------|
| `Tab` | Cambiar panel (Procesos ↔ Chat) |
| `↑/↓` o `j/k` | Navegar tabla de procesos |
| `PgUp/PgDn` | Scroll rápido (20 filas) |
| `Home/End` | Ir al inicio/final |
| `s` | Cambiar columna de ordenamiento |
| `r` | Invertir dirección de orden |
| `K` (shift+k) | Matar proceso seleccionado |
| `q` / `Esc` | Salir (en panel procesos) |
| `Ctrl+C` | Salir (global) |
| `Enter` | Enviar mensaje AI (en panel chat) |
| `Backspace` | Borrar carácter en chat |

## Optimización de Memoria (<2 MB)

1. **Zero-alloc refresh**: `sorted_processes` se reutiliza con `clear()` + `extend()` cada tick
2. **Sin iconos ni imágenes**: La TUI no carga `icon_data_url` ni assets gráficos
3. **Viewport clipping**: Sólo se renderizan las filas visibles en pantalla
4. **Pre-allocated buffers**: `String::with_capacity()` para input y formato
5. **Sin duplicación de snapshots**: Se lee directamente del `Arc<RwLock<SystemState>>` compartido
6. **Chat thread efímero**: El hilo AI se crea bajo demanda y termina tras cada respuesta
7. **Overhead estimado**: ~800 KB base (ratatui widgets + crossterm buffer + process Vec)

## Resolución de Proveedor AI

La TUI auto-detecta el proveedor disponible en este orden de prioridad:
1. **Ollama** (local, sin API key) — ideal para uso offline
2. **Anthropic** (Claude Haiku) — rápido y económico
3. **OpenRouter** (free tier) — acceso gratuito a Llama
4. **OpenAI** (GPT-4o-mini) — fallback

## Integración con el CLI

```
omnimon tui        # Lanza la interfaz de terminal
omnimon status     # Snapshot estático (ya existente)
omnimon chat       # Chat one-shot (ya existente)
```

El subcomando `tui` se añadió al enum `Commands` del CLI (`v4/crates/cli/src/main.rs`)
y delega a `omnimon_tui::run()`.
