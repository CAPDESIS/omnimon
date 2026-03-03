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

- `name` es obligatorio.
- `max_instances`, `max_ram_mb` y `max_cpu_percent` son opcionales.
- Si falta un campo, se asume `0` (sin limite).
- El daemon cachea la lista y la invalida al recargar config.

## Hot Reload

- El daemon detecta cambios en `macmon.yaml` y recarga automaticamente.
- Desde GUI tambien puedes forzar recarga con `Reload Configuration`.

## Fallback seguro

Si el YAML tiene tabs o sintaxis invalida en `custom_processes`, macmon:

- registra advertencia en logs,
- notifica en UI,
- y mantiene una configuracion segura por defecto en memoria (sin crash).
