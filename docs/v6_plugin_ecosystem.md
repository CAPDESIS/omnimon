# Ecosistema de Plugins V6.0

## Objetivo

OmniMon V6 incorpora un ecosistema de plugins embebidos para que operadores y equipos SRE puedan extender la recoleccion de metricas sin recompilar el producto. La primera iteracion se apoya en Lua embebido con `mlua` dentro del backend Tauri/Rust y un gestor visual en Svelte 5.

## Arquitectura

### 1. Runtime embebido en Rust

- Modulo: `v4/apps/desktop/src-tauri/src/plugins.rs`
- Motor: `mlua` con runtime `lua54` vendorizado.
- Modelo de ejecucion:
  - Cada plugin se ejecuta en una VM Lua fresca por corrida.
  - No se exponen APIs de filesystem, red ni subprocess al script.
  - El backend solo entrega un contexto serializado con telemetria segura y de solo lectura.
  - Cada ejecucion usa limites de memoria e instrucciones para evitar cuelgues.

### 2. Sandbox y contencion

- Límite de memoria por VM: `1 MiB`.
- Límite de tiempo por corrida: `150 ms`.
- Hook de instrucciones: aborta loops infinitos o cargas de CPU prolongadas.
- Sin FFI, sin bindings a shell, sin acceso a Tauri desde Lua.
- VM efimera por poll: un plugin no puede mantener estado mutable compartido entre corridas ni contaminar el host.

### 3. Contrato publico de plugins

Cada script debe exportar:

```lua
function manifest()
  return {
    name = "Docker Monitor",
    version = "1.0.0",
    description = "Reports Docker-related metrics"
  }
end

function collect(ctx)
  return {
    metrics = {
      {
        name = "docker.containers.running",
        label = "Running containers",
        kind = "gauge",
        value = 4,
        unit = "count",
        tags = { source = "docker" }
      }
    }
  }
end
```

### 4. Contexto entregado al plugin

`collect(ctx)` recibe un snapshot seguro con:

- `timestamp_ms`
- `cpu_usage_percent`
- `total_memory_bytes`
- `used_memory_bytes`
- `free_memory_bytes`
- `swap_used_mb`
- `net_rx_bytes_per_sec`
- `net_tx_bytes_per_sec`
- `process_count`
- `top_processes[]` con `pid`, `name`, `exec_name`, `cpu_pct`, `memory_mb`, `net_rx_bytes_per_sec`, `net_tx_bytes_per_sec`

No se entrega acceso a APIs privilegiadas ni handles del sistema.

### 5. Persistencia

- Carpeta de datos: `app_data_dir/plugins/`
- Scripts: `app_data_dir/plugins/scripts/*.lua`
- Manifest de instalacion: `app_data_dir/plugins/index.json`

## Flujo UI

### Vista Plugins en Svelte 5

- Componente: `v4/apps/desktop/src/components/Plugins.svelte`
- Capacidades:
  - cargar scripts `.lua`
  - validar al instalar
  - activar/desactivar plugins
  - eliminar plugins
  - refrescar estado y metricas emitidas
  - inspeccionar errores de sandbox/validacion

### IPC expuesto al frontend

- `list_plugins`
- `install_plugin(fileName, source)`
- `set_plugin_enabled(pluginId, enabled)`
- `remove_plugin(pluginId)`

## Seguridad

- Los plugins solo devuelven datos; no ejecutan acciones destructivas.
- Cada salida se valida en Rust antes de entrar al estado de la app.
- Maximos operativos:
  - 32 plugins instalados
  - 256 KiB por script
  - 64 metricas por plugin
  - 12 tags por metrica
- Errores y timeouts se encapsulan por plugin y no derriban la aplicacion principal.

## Integracion Rust <-> Svelte

- Rust corre el loop de polling y persiste el registro de plugins.
- Svelte consulta por IPC y renderiza el estado en tiempo real.
- Las metricas personalizadas se muestran dinamicamente por plugin dentro de la vista Plugins.

## Limitaciones actuales

- La primera iteracion expone una API de lectura, no una API de acciones.
- El aislamiento es fuerte a nivel de VM embebida, memoria y tiempo, pero no reemplaza un proceso separado o WASM capability-based para escenarios de amenaza extrema.
- No existe aun firma criptografica ni marketplace de plugins.

## Evolucion sugerida V6.x

1. Firmas Ed25519 para plugins confiables.
2. Capacidades declarativas por manifest (`telemetry.read`, `alerts.emit`, etc.).
3. Canal de eventos para metricas historicas y alertas derivadas.
4. Migracion opcional a WASM para aislamiento aun mas estricto.
