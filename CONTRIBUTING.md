# Contributing to OmniMon

Thanks for your interest in contributing to OmniMon! As an open source project, we rely on the community to improve, stabilize, and expand the tool across all platforms.

## Prerequisites

- **Rust** 1.75+ with `cargo`
- **bun** (package manager — not npm/yarn)
- **Tauri CLI** (`cargo install tauri-cli`)
- **Platform deps:**
  - **Windows:** Visual Studio Build Tools 2019+ with "Desktop development with C++", WebView2
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux (Debian/Ubuntu):** `sudo apt install -y build-essential libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libjavascriptcoregtk-4.1-dev libsoup-3.0-dev`

See [v4/INSTALACION_PREREQUISITOS.md](v4/INSTALACION_PREREQUISITOS.md) for detailed installation instructions per OS.

## Development Environment

1. **Clone the repository:**
   ```bash
   git clone https://github.com/chochy2001/omnimon.git
   cd omnimon
   ```

2. **Run the setup script:**
   - macOS/Linux: `./v4/setup-dev.sh`
   - Windows: `.\v4\setup-dev.ps1`

   This checks and installs Rust, bun, Tauri CLI, and native OS dependencies.

3. **Start development mode:**
   ```bash
   cd v4
   make dev
   ```
   Compiles the Rust backend and launches the Tauri app with Vite/Svelte hot-reloading.

## Development Commands

```bash
cd v4

# Development
make dev                                    # Full-stack dev mode

# Testing
bun run test                                # Frontend unit tests (Vitest)
bun run test:e2e                            # E2E tests (Playwright)
cargo test --workspace                      # Rust tests
make test-all                               # All: fmt + clippy + cargo test

# Linting
cargo fmt --check                           # Rust formatting
cargo clippy -- -D warnings                 # Rust linting (zero warnings)

# Build
bun run build                               # Frontend build
cargo check --workspace                     # Rust type check
bun run tauri build -- --debug --no-bundle  # Quick full-stack validation
```

## Cross-Platform Requirements

OmniMon runs natively on **macOS, Windows, and Linux**. All features must be supported on all three platforms or degrade gracefully.

- Use `#[cfg(target_os = "...")]` for OS-specific Rust code.
- CI/CD validates on Ubuntu, macOS, and Windows runners. PRs that break any platform cannot be merged.

## Workflow and Pull Requests

1. Fork the project and work on a descriptive branch: `feat/my-feature` or `fix/my-fix`.
2. Keep changes focused — avoid mixing frontend logic in native core crates without IPC justification.
3. **Before submitting:** verify your code meets standards:
   ```bash
   cd v4
   make test-all
   ```
4. Open a Pull Request against `main` describing what problem your code solves and how to test it.

## Code Style

### Rust
- `cargo fmt` — no exceptions
- `cargo clippy -- -D warnings` — zero warnings
- Edition 2021
- Use `#[cfg(target_os)]` for platform-specific code

### TypeScript / Svelte
- Strict TypeScript (`strict: true`)
- Svelte 5 with runes (`$state`, `$derived`, `$effect`)
- bun as package manager (never npm/yarn)

## Commit Convention (Conventional Commits)

We require Conventional Commits for clean history and reliable changelogs:

- `feat:` — New features (e.g. `feat(ai): add Claude 3.5 support`)
- `fix:` — Bug fixes (e.g. `fix(core): prevent hang on nonexistent process`)
- `docs:` — Documentation changes
- `chore:` — Infrastructure, dependencies, release processes
- `refactor:` — Code restructuring without behavior changes
- `test:` — Adding or fixing tests
- `perf:` — Performance improvements

## Project Structure

```
v4/
├── Cargo.toml               # Workspace (4 crates)
├── Makefile                  # Dev shortcuts
├── setup-dev.sh              # Dev environment setup
├── crates/
│   ├── core/                 # Native monitoring engine
│   ├── cli/                  # CLI binary (17+ commands)
│   └── tui/                  # Terminal UI (ratatui)
└── apps/desktop/             # Tauri desktop app
    ├── src/                  # Svelte 5 frontend
    │   ├── components/       # 39+ UI components
    │   ├── stores/           # Reactive state management
    │   └── lib/              # Utilities, types, IPC wrappers
    └── src-tauri/            # Rust Tauri backend
```

We're excited to review your contributions!

---

# Contribuir a OmniMon (Español)

¡Gracias por tu interés en contribuir a OmniMon!

## Prerrequisitos

- **Rust** 1.75+ con `cargo`
- **bun** (gestor de paquetes — no npm/yarn)
- **Tauri CLI** (`cargo install tauri-cli`)
- **Deps de plataforma:**
  - **Windows:** Visual Studio Build Tools 2019+ con "Desktop development with C++", WebView2
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux (Debian/Ubuntu):** `sudo apt install -y build-essential libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libjavascriptcoregtk-4.1-dev libsoup-3.0-dev`

Guía detallada: [v4/INSTALACION_PREREQUISITOS.md](v4/INSTALACION_PREREQUISITOS.md)

## Entorno de Desarrollo

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/chochy2001/omnimon.git
   cd omnimon
   ```

2. **Ejecutar el script de configuración:**
   - macOS/Linux: `./v4/setup-dev.sh`
   - Windows: `.\v4\setup-dev.ps1`

3. **Iniciar modo desarrollo:**
   ```bash
   cd v4
   make dev
   ```

## Comandos de Desarrollo

```bash
cd v4

make dev                                    # Modo desarrollo full-stack
bun run test                                # Tests unitarios frontend
cargo test --workspace                      # Tests Rust
make test-all                               # Todo: fmt + clippy + tests
cargo fmt --check                           # Formato Rust
cargo clippy -- -D warnings                 # Lint Rust (cero warnings)
```

## Requisitos Multiplataforma

OmniMon se ejecuta nativamente en **macOS, Windows y Linux**. Toda nueva funcionalidad debe ser compatible en las tres plataformas o degradarse de forma elegante.

- Usa `#[cfg(target_os = "...")]` para código específico de plataforma.
- El CI/CD valida en runners de Ubuntu, macOS y Windows.

## Flujo de Trabajo

1. Haz fork y trabaja en una rama descriptiva: `feat/nueva-funcion` o `fix/correccion`.
2. Mantén los cambios enfocados.
3. Verifica antes de enviar: `cd v4 && make test-all`
4. Abre un Pull Request contra `main` describiendo qué problema resuelve.

## Convención de Commits

- `feat:` — Nuevas funcionalidades
- `fix:` — Corrección de errores
- `docs:` — Documentación
- `chore:` — Infraestructura y dependencias
- `refactor:` — Reestructuración sin cambio de comportamiento
- `test:` — Tests
- `perf:` — Mejoras de rendimiento

¡Esperamos tus contribuciones!
