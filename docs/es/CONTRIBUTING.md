# Politica de Contribucion

Este repositorio se mantiene por el owner.

1. La identidad de autor en commits debe ser `chochy2001 <54371626+chochy2001@users.noreply.github.com>`.
2. La propiedad de codigo se aplica con `.github/CODEOWNERS` y `@chochy2001`.
3. No se aceptan metadatos de commit generados por asistentes externos.
4. Antes de hacer push ejecuta `make check`, `make test`, `make audit` y `make verify-authors`.

## Verificacion de Autor

Usa:

```bash
make verify-authors
```

CI tambien valida la identidad del autor en cada push y pull request.
