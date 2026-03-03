# Arquitectura

macmon se compone de CLI + daemon + UI nativa AppKit con libreria compartida:

1. `src/daemon/macmond.sh`: monitoreo continuo, alertas y limpieza.
2. `src/cli/macmon.sh`: comandos de usuario (`status`, `start`, `config`, `export`, etc.).
3. `src/gui/ProcessPicker.swift`: interfaz para seleccionar/cerrar procesos.
4. `src/gui/MacmonStatusBar.swift`: icono de menu bar con acciones rapidas.
5. `lib/macmon-core.sh`: funciones compartidas de seguridad, parser, recoleccion y kill.

## Confiabilidad

1. Recarga en caliente de config por cambio de archivo.
2. PID file con lock + escritura atomica para evitar race conditions.
3. Verificacion de procesos y protecciones de sistema antes de enviar señales.

## Rendimiento

1. Recoleccion por lotes con `ps` y `lsof`.
2. Cache de configuracion dinamica.
3. Actualizacion del menu bar fuera del hilo principal y aplicacion de UI en Main Thread.

## UX / DX

1. Process Picker navegable por teclado.
2. Labels basicos de accesibilidad para VoiceOver.
3. Config editable desde GUI y CLI.
