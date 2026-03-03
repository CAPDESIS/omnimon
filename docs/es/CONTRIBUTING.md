# Contribuir

Gracias por tu interes en macmon. Este es un proyecto personal, pero las contribuciones son bienvenidas.

## Como Contribuir

1. Haz un fork del repositorio y crea una rama
2. Haz tus cambios
3. Ejecuta `make check` para verificar dependencias y compilacion
4. Ejecuta `make test` para correr los tests
5. Envia un pull request

## Setup de Desarrollo

```bash
brew install jq bats-core    # dependencias
xcode-select --install       # compilador Swift

make check    # verificar que todo funciona
make test     # correr tests
```

## Lineamientos

- Todo el codigo y mensajes de commit deben estar en ingles
- Usa el estilo de commit convencional: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`
- Los scripts de shell deben pasar `bash -n`
- El codigo Swift debe compilar con `swiftc -O -framework Cocoa` (no se necesita proyecto Xcode)
- Agrega tests para funcionalidad nueva cuando sea posible

## Reportar Problemas

Abre un issue en GitHub con:
- Version de macOS
- Pasos para reproducir
- Comportamiento esperado vs real
- Logs relevantes (`macmon log`)
