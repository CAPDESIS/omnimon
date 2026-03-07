# Contributing to OmniMon

Thanks for your interest in contributing to OmniMon! As an open source project, we rely on the community to improve, stabilize, and expand the tool across all platforms.

## Development Environment

Setting up the cross-platform environment (Rust, Tauri, Svelte) is straightforward with our orchestration scripts.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/chochy2001/omnimon.git
   cd omnimon
   ```

2. **Run the setup script:**
   * macOS/Linux: `./v4/setup-dev.sh`
   * Windows: `.\v4\setup-dev.ps1`

   This script checks and/or installs Node.js, Rust, Cargo, and native OS dependencies like WebView2 (Windows) or libwebkit2gtk (Linux).

3. **Start development mode:**
   ```bash
   cd v4
   make dev
   ```
   This compiles the Rust backend and launches the Tauri interface with Vite/Svelte hot-reloading.

## Cross-Platform Requirements

OmniMon v4 is designed to run natively on **macOS, Windows, and Linux**. Any new feature or module (e.g. browser tab tracking, native OS interactions) **must** be supported on all three platforms, or degrade gracefully if the OS API doesn't support it.

* Before proposing a new feature, ensure the code compiles and passes tests on all three environments.
* Use Rust's `#[cfg(target_os = "...")]` typing for OS-specific implementations.
* **CI/CD will automatically validate** your changes on Ubuntu, macOS, and Windows runners. If your Pull Request breaks the build on any platform, it cannot be merged.

## Workflow and Pull Requests

1. Fork the project and work on a descriptive branch, e.g. `feat/my-new-feature` or `fix/bug-fix`.
2. Implement your changes (avoid mixing frontend logic in native core crates without proper IPC justification).
3. **Critical checkpoint:** Verify your code meets standards:
   ```bash
   cd v4
   make test-all
   ```
   This runs `cargo fmt`, `cargo clippy --workspace -- -D warnings`, and `cargo test`. **Your PR will not be accepted if GitHub CI fails or detects warnings.**
4. Open a Pull Request against the `main` branch clearly describing what problem your code solves and how to test it.

## Commit Convention (Conventional Commits)

We require Conventional Commits to maintain a clean history and generate reliable changelogs.
* `feat:` New features (e.g. `feat(ai): add Claude 3.5 support`).
* `fix:` Bug fixes (e.g. `fix(core): prevent hang when reading nonexistent process`).
* `docs:` Documentation-only changes (README, SECURITY, CONTRIBUTING, `/docs`).
* `chore:` Infrastructure maintenance, dependencies, or release processes.
* `refactor:` Code refactoring without altering observable behavior.
* `test:` Adding or fixing tests.

We're excited to review your contributions!

---

# Contribuir a OmniMon (Español)

¡Gracias por tu interés en contribuir a OmniMon! Como proyecto de código abierto, dependemos de la comunidad para mejorar, estabilizar y expandir la herramienta en todas las plataformas.

## Entorno de Desarrollo

Configurar el entorno multiplataforma (Rust, Tauri, Svelte) es sencillo con nuestros scripts de orquestación.

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/chochy2001/omnimon.git
   cd omnimon
   ```

2. **Ejecutar el script de configuración:**
   * macOS/Linux: `./v4/setup-dev.sh`
   * Windows: `.\v4\setup-dev.ps1`

   Este script verifica y/o instala Node.js, Rust, Cargo y dependencias nativas del sistema operativo como WebView2 (Windows) o libwebkit2gtk (Linux).

3. **Iniciar el modo de desarrollo:**
   ```bash
   cd v4
   make dev
   ```
   Esto compila el backend de Rust y lanza la interfaz de Tauri con recarga en caliente (hot-reloading) de Vite/Svelte.

## Requisitos Multiplataforma

OmniMon v4 está diseñado para ejecutarse de forma nativa en **macOS, Windows y Linux**. Cualquier nueva característica o módulo (ej. seguimiento de pestañas del navegador, interacciones nativas del SO) **debe** ser compatible en las tres plataformas, o degradarse de manera elegante si la API del SO no lo soporta.

* Antes de proponer una nueva característica, asegúrate de que el código compile y pase las pruebas en los tres entornos.
* Usa la directiva de Rust `#[cfg(target_os = "...")]` para implementaciones específicas por SO.
* **El CI/CD validará automáticamente** tus cambios en entornos de Ubuntu, macOS y Windows. Si tu Pull Request rompe la compilación en alguna plataforma, no podrá ser fusionada.

## Flujo de Trabajo y Pull Requests

1. Haz un Fork del proyecto y trabaja en una rama descriptiva, ej. `feat/nueva-funcion` o `fix/correccion-error`.
2. Implementa tus cambios (evita mezclar lógica de frontend en los crates nativos sin una justificación clara de IPC).
3. **Punto de control crítico:** Verifica que tu código cumpla con los estándares:
   ```bash
   cd v4
   make test-all
   ```
   Esto ejecuta `cargo fmt`, `cargo clippy --workspace -- -D warnings` y `cargo test`. **Tu PR no será aceptada si el CI de GitHub falla o detecta advertencias (warnings).**
4. Abre una Pull Request contra la rama `main` describiendo claramente qué problema resuelve tu código y cómo probarlo.

## Convención de Commits (Conventional Commits)

Requerimos Conventional Commits para mantener un historial limpio y generar registros de cambios (changelogs) confiables.
* `feat:` Nuevas funcionalidades (ej. `feat(ai): add Claude 3.5 support`).
* `fix:` Corrección de errores (ej. `fix(core): prevent hang when reading nonexistent process`).
* `docs:` Solo cambios en la documentación (README, SECURITY, CONTRIBUTING, `/docs`).
* `chore:` Mantenimiento de infraestructura, dependencias o procesos de lanzamiento.
* `refactor:` Refactorización de código sin alterar el comportamiento observable.
* `test:` Añadir o arreglar pruebas.

¡Estamos emocionados de revisar tus contribuciones!