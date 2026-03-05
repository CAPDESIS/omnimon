# Contributing to OmniMon

¡Gracias por tu interés en contribuir a OmniMon! Como proyecto open source, dependemos de la comunidad para mejorar, estabilizar y expandir la herramienta a través de todas sus plataformas.

## Entorno de Desarrollo (DX)

Levantar el entorno multiplataforma (Rust, Tauri, Svelte) es sumamente fácil gracias a nuestros scripts de orquestación.

1. **Clona el repositorio:**
   ```bash
   git clone https://github.com/chochy2001/macmon.git
   cd macmon
   ```

2. **Ejecuta el script de Setup:**
   * En macOS/Linux: `./v4/setup-dev.sh`
   * En Windows: `.\v4\setup-dev.ps1`
   
   Este script verificará y/o instalará Node.js, Rust, Cargo, y dependencias nativas requeridas por el SO como WebView2 (Windows) o libwebkit2gtk (Linux).

3. **Inicia el Modo Desarrollo:**
   ```bash
   cd v4
   make dev
   ```
   Esto compilará el backend en Rust y levantará la interfaz de Tauri conectada al hot-reloading de Vite/Svelte.

## Flujo de Trabajo y Pull Requests

1. Crea un fork del proyecto y trabaja en una rama descriptiva, por ejemplo: `feat/mi-nueva-funcionalidad` o `fix/solucion-bug`.
2. Implementa tus cambios (recuerda no mezclar lógica de frontend en los crates del core nativo sin una buena justificación de IPC).
3. **Punto de Control Crítico:** Verifica que tu código cumpla los estándares:
   ```bash
   cd v4
   make test-all
   ```
   Esto correrá `cargo fmt`, `cargo clippy --workspace -- -D warnings`, y `cargo test`. **Tu PR no será aceptado si el CI de GitHub falla en estos pasos o detecta warnings.**
4. Abre un Pull Request contra la rama `main` describiendo claramente qué problema resuelve tu código y cómo probarlo.

## Convención de Commits (Conventional Commits)

Exigimos el uso de Conventional Commits para mantener un historial limpio y generar changelogs automáticos confiables.
* `feat:` Para nuevas funcionalidades (ej. `feat(ai): agregar soporte para Claude 3.5`).
* `fix:` Para reparación de bugs (ej. `fix(core): prevenir cuelgue al leer proceso inexistente`).
* `docs:` Cambios exclusivos a README, SECURITY, CONTRIBUTING o carpeta `/docs`.
* `chore:` Mantenimiento de infraestructura, dependencias, o procesos de release.
* `refactor:` Refactorización de código existente sin alterar funcionalidad observable.
* `test:` Adición o corrección de tests de la suite.

¡Estamos emocionados de revisar tus contribuciones!