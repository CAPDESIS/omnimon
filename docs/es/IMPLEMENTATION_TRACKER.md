# Seguimiento de Implementacion

Estado de las funcionalidades de macmon y trabajo planeado.

## Completado

- Monitoreo dinamico de procesos con umbrales configurables por proceso
- Soporte de internacionalizacion (ingles y espanol)
- Manejo de archivo PID con proteccion de lock
- Recarga de config al detectar cambios y con SIGUSR1
- Sistema de perfiles con presets (developer, creator, gaming)
- Analisis opcional de procesos via proveedores externos
- Blocklist de procesos protegidos con verificacion de firma Apple

## Planeado

- UI dedicada para editar archivos YAML de perfiles
- Tests de integracion para el ciclo completo del daemon
- Modo dry-run para previsualizar optimizaciones
- Exportar/importar perfiles firmados
- Automatizacion del checklist de release

## Verificacion

```bash
make check                    # dependencias + sintaxis + compilacion
make test                     # tests BATS
bash tests/swift/run_tests.sh # XCTests de Swift
make audit                    # shellcheck + CVEs
```
