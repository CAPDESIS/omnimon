# Seguimiento de Implementacion

Este archivo resume hitos completados y siguientes pasos de macmon.

## Completado

1. Monitoreo dinamico de procesos desde YAML custom_processes.
2. Base de i18n para UI y mensajes CLI.
3. Endurecimiento de confiabilidad para PID y fallback de config.
4. Sistema de perfiles con cambio en caliente desde CLI y AppKit.
5. Integracion de analisis IA con API keys en Keychain.
6. Flujo human in the loop con aprobacion explicita del usuario.
7. Blocklist inmutable y protecciones de procesos Apple.
8. Mitigacion de alucinaciones LLM con extraccion de PIDs por regex.
9. Saneamiento de PIDs contra blocklist y validacion de proceso vivo.

## En Progreso

1. Diagnosticos de UI para calidad de respuesta IA.
2. Modo dry run para previsualizacion de optimizacion.

## Siguientes Pasos

1. UI dedicada para ver y editar YAML de perfiles.
2. Pruebas de integracion con respuestas mock de proveedores.
3. Telemetria local opcional para mejorar sugerencias.
4. Formato firmado para exportar e importar perfiles.
5. Automatizacion de checklist de release para docs y capturas.

## Checklist de Validacion

1. `make check`
2. `make test`
3. `bash tests/swift/run_tests.sh`
4. `make audit`
5. `make verify-authors`
