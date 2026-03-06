# Arquitectura de OmniMon

*Lea esto en otros idiomas: [English](ARCHITECTURE.md)*

Este documento describe la arquitectura de alto nivel de OmniMon, centrándose en la comunicación y la seguridad del sistema.

## Puente de IPC Seguro

OmniMon utiliza un robusto puente de Comunicación entre Procesos (IPC) para comunicarse de manera segura entre el frontend (interfaz de usuario) y el backend nativo del sistema.

### Mitigación de RCE en AppleScript
Para ejecutar AppleScript de manera segura en tareas como la introspección de pestañas del navegador, sin el riesgo de Ejecución Remota de Código (RCE) mediante la inyección de argumentos, OmniMon evita la interpolación de cadenas de datos proporcionados por el usuario dentro de los scripts.
En su lugar, los scripts de Apple utilizan el manejador `on run argv`. Los argumentos se pasan estrictamente como parámetros posicionales utilizando la bandera `-e` con `osascript`:

```rust
let mut cmd = Command::new("osascript");
cmd.arg("-e");
cmd.arg(script); // El script estático que contiene 'on run argv'
cmd.arg(user_provided_arg1); // Pasado de forma segura como argumentos posicionales
cmd.arg(user_provided_arg2);
```

### Validación de WebSocket para CDP
Para el Protocolo de Herramientas de Desarrollo de Chrome (CDP), el sistema garantiza que los endpoints de WebSocket no sean susceptibles a salto de directorios (path traversal). Cualquier `tab_id` enviado desde el frontend se valida estrictamente antes de ser utilizado para construir las URLs de conexión. El sistema rechaza activamente caracteres como `/`, `\`, `?` y `#`, garantizando que las conexiones solo se realicen a endpoints de depuración válidos y autorizados.
