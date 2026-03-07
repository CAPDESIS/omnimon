# OS Telemetry Engine

## Objetivo

Este motor unifica la recoleccion nativa de procesos en una sola fuente de verdad para Rust, CLI y GUI. El objetivo es reducir ruido visual, enriquecer cada proceso con identidad estable e iconografia real del sistema operativo, y exponer metricas de memoria, CPU, disco, red y energia con el mismo contrato en todas las superficies.

## Componentes principales

- `crates/core/src/process_identity.rs`
  - Normaliza nombres ruidosos como `Chrome Helper (Renderer)`.
  - Resuelve una identidad de grupo estable por familia de navegador, bundle, ejecutable o nombre normalizado.
  - Clasifica grupos de alto nivel como `Browser` y `System`.

- `crates/core/src/watcher.rs`
  - Mantiene el snapshot vivo de procesos con metadata extendida: `exe_path`, `bundle_id`, disco por proceso, red por PID y `energy_impact_score`.
  - Mezcla la muestra de `NetworkTelemetryEngine` con el cache de procesos sin bloquear el hilo de UI.

- `crates/core/src/app_icons.rs`
  - Implementa resolucion de iconos nativos con cache en memoria.
  - macOS: detecta el bundle `.app`, resuelve `Info.plist`, convierte `.icns` a PNG y devuelve `data:image/...`.
  - Linux: busca iconos del tema del sistema y `pixmaps` por nombre de ejecutable/proceso.
  - Windows: usa PowerShell + `System.Drawing.Icon::ExtractAssociatedIcon` como fallback pragmatico para extraer PNG asociado al ejecutable.

- `crates/core/src/telemetry.rs`
  - Publica `TelemetrySnapshot`, el contrato unificado para CLI y GUI.
  - Expone procesos enriquecidos y `super_processes` agregados desde el mismo cache del watcher.

- `crates/core/src/metrics.rs`
  - Mantiene la agregacion por `SuperProcess`.
  - Suma memoria, CPU, disco y red por identidad estable.
  - Calcula un `energy_impact_score` heuristico basado en CPU, memoria, disco y red cuando el SO no ofrece una API uniforme.

## Estrategia de agrupacion

El agrupador sigue este orden:

1. Familia de navegador (`Chrome`, `Safari`, `Brave`, `Edge`, `Arc`, `Firefox`)
2. Bundle nativo (`.app` en macOS)
3. Nombre de ejecutable / basename de ruta
4. Nombre de proceso normalizado

Esto evita que helpers, renderers o web contents aparezcan como ruido separado y garantiza que la RAM/CPU/disco/red se sumen sobre la misma identidad visible.

## Pipeline de iconos

1. El watcher conserva `exe_path` y `bundle_id`.
2. `telemetry_snapshot()` resuelve `icon_data_url` por proceso con cache en memoria.
3. La GUI intenta renderizar primero el icono nativo; si no existe, usa el fallback SVG previo.
4. La CLI mantiene paridad semantica aunque no renderiza imagenes.

## Paridad CLI/GUI

La paridad ya no depende de dos caminos distintos:

- GUI (`apps/desktop/src-tauri/src/lib.rs`) consume `TelemetrySnapshot` para `get_metrics()`.
- CLI (`crates/cli/src/main.rs`) consume el mismo `TelemetrySnapshot` en `status` texto/JSON.

Ambos reciben:

- nombre de grupo y clave de agrupacion
- cantidad de procesos agregados
- memoria y CPU
- I/O de disco
- trafico de red por proceso y por grupo
- energy impact score
- metadata nativa (`bundle_id`, `exe_path`, icono)

## Consideraciones de rendimiento y leaks

- El watcher usa `OnceLock + Arc<RwLock<SystemState>>` y reusa un solo hilo de monitoreo.
- El motor de red mantiene un buffer acotado y un canal con profundidad fija.
- El cache de iconos usa `RwLock<HashMap<...>>` y evita regenerar iconos repetidos.
- No se introducen ciclos de `Arc`; el modelo sigue siendo snapshots clonables y estructuras owning simples.

## Limitaciones actuales

- `energy_impact_score` es heuristico y no una lectura oficial del scheduler/OS.
- Windows usa un fallback de extraccion pragmatico por PowerShell; es funcional pero se puede reemplazar luego por Win32 puro.
- La GUI ya recibe los nuevos campos, pero aun puede ampliarse para mostrar columnas dedicadas de disco/red/energia en mas vistas.

## Verificacion

- `cargo check --workspace`
