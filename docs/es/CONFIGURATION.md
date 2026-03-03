# Configuracion

macmon usa un archivo YAML en `~/.config/macmon/macmon.yaml`.

## Gestion rapida

```bash
macmon config         # Mostrar config activa
macmon config edit    # Editar en $EDITOR
macmon config reset   # Restaurar valores por defecto
macmon config path    # Ver ruta del archivo
```

## Estructura recomendada

```yaml
thresholds:
  ram_free_percent: 25
  swap_used_mb: 2048
  process_ram_min_kb: 102400
  idle_cpu_percent: 1.0
  idle_ram_trigger_percent: 40

custom_processes:
  - name: "flutter_tester"
    max_instances: 10
  - name: "gradlew"
    max_ram_mb: 2048
  - name: "SourceKitService"
    max_cpu_percent: 90

intervals:
  check: 60
  idle_check: 600
  cooldown: 300
  kill_grace: 3
```

## Reglas de `custom_processes`

1. `name` es obligatorio.
2. `max_instances`, `max_ram_mb` y `max_cpu_percent` son opcionales.
3. Si falta un campo, se asume `0` (sin limite).
4. El daemon cachea la lista y la invalida al recargar config.

## Perfiles

Los presets se cargan desde `~/.config/macmon/profiles/`.

1. `developer.yaml`
2. `creator.yaml`
3. `gaming-performance.yaml`

Comandos:

```bash
macmon profile list
macmon profile current
macmon profile use developer
```

El cambio de perfil dispara recarga del daemon en tiempo real.

## Hot Reload

1. El daemon detecta cambios en `macmon.yaml` y recarga automaticamente.
2. Desde GUI tambien puedes forzar recarga con `Reload Configuration`.

## Fallback seguro

Si el YAML tiene tabs o sintaxis invalida en `custom_processes`, macmon:

1. Registra advertencia en logs.
2. Notifica en UI.
3. Mantiene una configuracion segura por defecto en memoria sin crash.
