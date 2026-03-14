# OmniMon v4 - Guía de Inicio Rápido

OmniMon es un monitor de sistema multiplataforma construido con Tauri, Rust y Svelte.

---

## Prerequisitos

### Esenciales (todas las plataformas)
- **Node.js** v18+ — Runtime de JavaScript
- **Rust** 1.75+ (cargo + rustc) — Compilador para Tauri
- **Bun** — Package manager del proyecto

### Específicos por plataforma

| Plataforma | Dependencia adicional |
|------------|----------------------|
| **Windows** | Visual Studio Build Tools 2019+ con "Desktop development with C++" |
| **macOS** | Xcode Command Line Tools (`xcode-select --install`) |
| **Linux** | `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libgtk-3-dev` |

**Guía de instalación completa:** [INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md)

### Verificar que tienes todo instalado

```bash
node --version    # debe ser v18+
cargo --version   # debe estar instalado
bun --version     # debe estar instalado
```

---

## Inicio Rápido

### Windows
```powershell
cd omnimon\v4
.\EJECUTAR_OMNIMON.bat
```

### macOS / Linux
```bash
cd omnimon/v4
./setup-dev.sh   # Solo la primera vez
make dev
```

### Universal (cualquier plataforma)
```bash
cd omnimon/v4/apps/desktop
bun install
bun run tauri dev
```

---

## Documentación

| Documento | Descripción |
|-----------|-------------|
| [INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md) | Instalación de herramientas por SO (Windows, macOS, Linux) |
| [EJECUTAR_DEV.md](./EJECUTAR_DEV.md) | Guía para ejecutar, debug y troubleshooting |
| [CLAUDE.md](../CLAUDE.md) | Instrucciones para contribución con IA |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Guía para contribuidores |

---

## Scripts Disponibles

| Script | Plataforma | Descripción |
|--------|-----------|-------------|
| `EJECUTAR_OMNIMON.bat` | Windows | Ejecuta OmniMon con auto-detección |
| `run-dev-auto.ps1` | Windows | Script PowerShell con auto-detección |
| `run-dev.bat` | Windows | Script de desarrollo alternativo |
| `run-dev.ps1` | Windows | Script PowerShell de desarrollo |
| `setup-dev.sh` | macOS/Linux | Verifica e instala dependencias |
| `setup-dev.ps1` | Windows | Verifica e instala dependencias |
| `instalar-todo.ps1` | Windows | Instala todos los prerequisitos |
| `instalar-rust.ps1` | Windows | Instala solo Rust |
| `instalar-bun.ps1` | Windows | Instala solo Bun |

---

## Estructura del Proyecto

```
v4/
├── apps/
│   └── desktop/           # Aplicación Tauri principal
│       ├── src/           # Frontend (Svelte 5)
│       └── src-tauri/     # Backend (Rust)
├── crates/
│   ├── core/              # Motor de monitoreo nativo
│   ├── cli/               # CLI (17+ comandos)
│   └── tui/               # Terminal UI (ratatui)
├── Cargo.toml             # Workspace (4 crates)
├── Makefile               # Atajos de desarrollo
├── EJECUTAR_OMNIMON.bat   # Script de ejecución (Windows)
├── setup-dev.sh           # Setup para macOS/Linux
└── README.md              # Este archivo
```

---

## Troubleshooting Rápido

| Problema | Solución |
|----------|----------|
| `cargo: command not found` | Instalar Rust o cerrar/reabrir terminal |
| `Port 1420 is already in use` | Matar proceso anterior: `lsof -ti:1420 \| xargs kill -9` |
| `LINK: fatal error` (Windows) | Instalar Visual Studio Build Tools con C++ |
| `libwebkit2gtk not found` (Linux) | `sudo apt install libwebkit2gtk-4.1-dev` |
| La app no abre | Verificar prerequisitos con `node --version && cargo --version` |
| Compilación lenta (primera vez) | Normal (~5-15 min). Las siguientes son incrementales |

Troubleshooting detallado: [EJECUTAR_DEV.md](./EJECUTAR_DEV.md#troubleshooting)

---

## Funcionalidades de OmniMon

- Monitoreo de CPU, RAM, Disco en tiempo real
- Análisis de conexiones de red
- Explorador de procesos con kill inteligente
- Monitoreo de tabs de navegador (Chrome/Edge/Brave)
- Temas claro/oscuro
- Global hotkey (Ctrl+Alt+O)
- System tray con tooltip dinámico
- Sistema de alertas
- Gráficas de métricas en tiempo real
- Optimización con IA (multi-proveedor)

---

## Desarrollo

```bash
cd omnimon/v4

make dev                                    # Modo desarrollo full-stack
bun run test                                # Tests unitarios frontend
cargo test --workspace                      # Tests Rust
cargo check --workspace                     # Type check Rust
bun run tauri build -- --debug --no-bundle  # Validación rápida full-stack
```

**Package Manager:** Bun (no npm/yarn). Lockfile: `bun.lock`

---

## Contribuir

1. Lee [CONTRIBUTING.md](../CONTRIBUTING.md)
2. Verifica que los tests pasen: `bun run test && cargo test --workspace`
3. Verifica que compile: `cargo check --workspace`
4. Sigue la convención de commits (conventional commits)
