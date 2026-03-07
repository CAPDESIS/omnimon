# OmniMon Architecture

*Read this in other languages: [Español](ARCHITECTURE_ES.md)*


This document describes the high-level architecture of OmniMon, focusing on the communication and security of the system.

## Secure IPC Bridge

OmniMon uses a robust Inter-Process Communication (IPC) bridge to safely communicate between the frontend and the system's native backend.

### AppleScript RCE Mitigation
To securely execute AppleScript for tasks like browser tab introspection without the risk of Remote Code Execution (RCE) via argument injection, OmniMon avoids string interpolation of user-provided data into the scripts.
Instead, AppleScripts utilize the `on run argv` handler. Arguments are passed strictly as positional parameters using the `-e` flag with `osascript`:
```rust
let mut cmd = Command::new("osascript");
cmd.arg("-e");
cmd.arg(script); // The static script containing 'on run argv'
cmd.arg(user_provided_arg1); // Passed securely as positional args
cmd.arg(user_provided_arg2);
```

### CDP WebSocket Validation
For the Chrome Debugging Protocol (CDP), the system ensures that WebSocket endpoints are not susceptible to path traversal. Any `tab_id` sent from the frontend is strictly validated before being used to construct connection URLs. The system actively rejects characters like `/`, `\`, `?`, and `#`, guaranteeing that connections are only made to valid, authorized debugging endpoints.


---


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
