# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v4.0.x  | :white_check_mark: |
| v3.x.x  | :x:                |

## Reporting a Vulnerability

Por favor, **NO** abras un Issue público para reportar vulnerabilidades de seguridad.
Envía un correo electrónico directamente a los mantenedores o utiliza la función "Security Advisories" privada de GitHub. Responderemos en un plazo máximo de 48 horas.

## Medidas de Seguridad de OmniMon

### 1. Blocklists Inmutables y Seguras
OmniMon incluye listas de bloqueo estrictas y divididas por plataforma (`#[cfg(target_os)]`) incrustadas directamente en el core nativo de Rust (`v4/crates/core/src/killer.rs`).
*   **macOS:** Protege `kernel_task`, `launchd`, `WindowServer`, `coreaudiod`, etc.
*   **Windows:** Protege `smss.exe`, `csrss.exe`, `svchost.exe`, `lsass.exe`, etc.
*   **Linux:** Protege `systemd`, `init`, `dbus-daemon`, `xorg`, etc.

Incluso si el usuario o la IA intenta terminar estos procesos, el comando será denegado nativamente, previniendo "kernel panics" o cuelgues del SO (Blue Screens).

### 2. Almacenamiento de Credenciales (Keychain Nativo)
Las claves de API de los proveedores de IA (OpenAI, Anthropic, OpenRouter) **nunca** se almacenan en texto plano en el disco.
Utilizamos el crate multiplataforma `keyring` para abstraer y delegar el almacenamiento seguro al manejador de credenciales criptográficas del Sistema Operativo:
*   macOS: **Keychain Access**
*   Windows: **Credential Manager**
*   Linux: **Secret Service API (GNOME Keyring/KWallet)**

### 3. Mitigación de Vectores MITRE ATT&CK
*   **T1059 (Command and Scripting Interpreter):** La v4 eliminó la dependencia general de Bash y AppleScript inyectado (presentes en v3). Toda la introspección y finalización de procesos se hace a nivel de FFI/OS API con Rust de forma nativa, mitigando ataques de inyección de comandos.
*   **T1552 (Unsecured Credentials):** Mitigado a través de nuestra implementación obligatoria de Keychain/Credential Manager nativo.
*   **T1548.002 (Bypass User Access Control):** OmniMon se ejecuta en modo usuario (User-space) y no solicita escalada de privilegios (`sudo`/`root`) para operaciones regulares. Su capacidad de terminar procesos se limita estrictamente a la sesión de usuario actual (UID match).
