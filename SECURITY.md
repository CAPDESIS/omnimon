# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| v5.0.x  | :white_check_mark: |
| v4.0.x  | :x:                |
| v3.x.x  | :x:                |

## Reporting a Vulnerability

Please **DO NOT** open a public Issue to report security vulnerabilities.
Send an email directly to the maintainers or use GitHub's private "Security Advisories" feature. We will respond within a maximum of 48 hours.

## OmniMon Security Measures

### 1. Immutable & Secure Blocklists
OmniMon includes strict, OS-specific blocklists (`#[cfg(target_os)]`) embedded directly into the native Rust core (`v4/crates/core/src/killer.rs`).
*   **macOS:** Protects `kernel_task`, `launchd`, `WindowServer`, `coreaudiod`, etc.
*   **Windows:** Protects `smss.exe`, `csrss.exe`, `svchost.exe`, `lsass.exe`, etc.
*   **Linux:** Protects `systemd`, `init`, `dbus-daemon`, `xorg`, etc.

Even if the user or AI attempts to terminate these processes, the command will be natively denied, preventing kernel panics or OS crashes (Blue Screens).

### 2. Credential Storage (Native Keychain)
API keys for AI providers (OpenAI, Anthropic, OpenRouter) and CrabNebula are **never** stored in plain text on disk.
We use the cross-platform `keyring` crate to abstract and delegate secure storage to the OS's cryptographic credential manager:
*   macOS: **Keychain Access**
*   Windows: **Credential Manager**
*   Linux: **Secret Service API (GNOME Keyring/KWallet)**

### 3. Commitment to MITRE ATT&CK
We continuously model our threats based on the MITRE ATT&CK framework to defend the system:
*   **T1059 (Command and Scripting Interpreter):** v4 removed the general dependency on Bash and injected AppleScript (present in v3). All introspection and process termination are done natively at the FFI/OS API level with Rust, mitigating command injection attacks.
*   **T1552 (Unsecured Credentials):** Mitigated through our mandatory implementation of native Keychain/Credential Manager.
*   **T1548.002 (Bypass User Access Control):** OmniMon runs in user-space and does not request privilege escalation (`sudo`/`root`) for regular operations. Its ability to terminate processes is strictly limited to the current user session (UID match).

### 4. Ed25519 Release Signatures and SHA-256 Checksums
Starting with v5.0, every release binary is signed using Ed25519 cryptographic keys. Each artifact is accompanied by a SHA-256 checksum file. Users and automated systems can verify both the signature authenticity and file integrity before installation, ensuring that distributed binaries have not been tampered with.

### 5. Automated CVE Scanning
Our CI/CD lifecycle includes static analysis tools like `cargo-audit`, `npm audit`, `Grype`, and the Dependabot ecosystem for continuous detection of Common Vulnerabilities and Exposures (CVEs). Both Rust (`cargo audit`) and JavaScript (`npm audit`) dependency trees are scanned on every pull request before merging, blocking compromised transitive dependencies.

### 6. Input Sanitization for System Calls (Frontend -> Backend IPC)
All interactions between the frontend and native backend pass through a secure IPC bridge. This prevents malicious data injected from the frontend from compromising the system:
*   **AppleScript:** User-provided identifiers are never directly concatenated into AppleScript strings. They are passed as positional arguments, eliminating Remote Code Execution (RCE) vectors.
*   **WebSockets (CDP):** Debugging session IDs are rigorously validated to prevent Path Traversal. Characters like `/`, `\`, `?`, and `#` are rejected.

---

# Política de Seguridad (Español)

## Versiones Soportadas

| Versión | Soportada          |
| ------- | ------------------ |
| v5.0.x  | :white_check_mark: |
| v4.0.x  | :x:                |
| v3.x.x  | :x:                |

## Reportar una Vulnerabilidad

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
Las claves de API de los proveedores de IA (OpenAI, Anthropic, OpenRouter) y CrabNebula **nunca** se almacenan en texto plano en el disco.
Utilizamos el crate multiplataforma `keyring` para abstraer y delegar el almacenamiento seguro al manejador de credenciales criptográficas del Sistema Operativo:
*   macOS: **Keychain Access**
*   Windows: **Credential Manager**
*   Linux: **Secret Service API (GNOME Keyring/KWallet)**

### 3. Compromiso con MITRE ATT&CK
Modelamos constantemente nuestras amenazas basándonos en el framework MITRE ATT&CK para defender el sistema:
*   **T1059 (Command and Scripting Interpreter):** La v4 eliminó la dependencia general de Bash y AppleScript inyectado (presentes en v3). Toda la introspección y finalización de procesos se hace a nivel de FFI/OS API con Rust de forma nativa, mitigando ataques de inyección de comandos.
*   **T1552 (Unsecured Credentials):** Mitigado a través de nuestra implementación obligatoria de Keychain/Credential Manager nativo.
*   **T1548.002 (Bypass User Access Control):** OmniMon se ejecuta en modo usuario (User-space) y no solicita escalada de privilegios (`sudo`/`root`) para operaciones regulares. Su capacidad de terminar procesos se limita estrictamente a la sesión de usuario actual (UID match).

### 4. Firmas Ed25519 de Release y Checksums SHA-256
A partir de la v5.0, cada binario de release se firma con claves criptográficas Ed25519. Cada artefacto va acompañado de un archivo de checksum SHA-256. Los usuarios y sistemas automatizados pueden verificar tanto la autenticidad de la firma como la integridad del archivo antes de la instalación, asegurando que los binarios distribuidos no han sido manipulados.

### 5. Escaneos de CVEs Automatizados
Nuestro ciclo de CI/CD incluye herramientas de análisis estático como `cargo-audit`, `npm audit`, `Grype` y el ecosistema de Dependabot para la detección continua de Vulnerabilidades y Exposiciones Comunes (CVE). Tanto el árbol de dependencias de Rust (`cargo audit`) como el de JavaScript (`npm audit`) se escanean en cada pull request antes de ser fusionado, bloqueando dependencias transitivas comprometidas.

### 6. Sanitización de Entradas para Llamadas al Sistema (Frontend -> Backend IPC)
Todas las interacciones entre el frontend y el backend nativo pasan a través de un puente de IPC Seguro. Esto previene que datos maliciosos inyectados desde el frontend comprometan el sistema:
*   **AppleScript:** Los identificadores proporcionados por el usuario (como IDs de pestañas o URLs) nunca se concatenan directamente en cadenas de AppleScript. En su lugar, se pasan como argumentos posicionales (vía `osascript -e` y `on run argv`), eliminando vectores de Remote Code Execution (RCE).
*   **WebSockets (CDP):** Los IDs de sesión de depuración están rigurosamente validados para evitar el *Path Traversal*. Se rechazan los caracteres como `/`, `\`, `?` y `#`, lo que garantiza que las conexiones solo puedan abrirse contra los *endpoints* permitidos del Chrome Debugging Protocol.
