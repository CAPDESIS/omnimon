# Arquitectura

macmon se compone de CLI + daemon + UI nativa AppKit con libreria compartida:

- `src/daemon/macmond.sh`: monitoreo continuo, alertas y limpieza.
- `src/cli/macmon.sh`: comandos de usuario (`status`, `start`, `config`, `export`, etc.).
- `src/gui/ProcessPicker.swift`: interfaz para seleccionar/cerrar procesos.
- `src/gui/MacmonStatusBar.swift`: icono de menu bar con acciones rapidas.
- `lib/macmon-core.sh`: funciones compartidas de seguridad, parser, recoleccion y kill.

## Confiabilidad

- Recarga en caliente de config por cambio de archivo.
- PID file con lock + escritura atomica para evitar race conditions.
- Verificacion de procesos y protecciones de sistema antes de enviar señales.

## Rendimiento

- Recoleccion por lotes con `ps`/`lsof`.
- Cache de configuracion dinamica.
- Actualizacion del menu bar fuera del hilo principal y aplicacion de UI en Main Thread.

## UX / DX

- Process Picker navegable por teclado.
- Labels basicos de accesibilidad (VoiceOver).
- Config editable desde GUI y CLI.
