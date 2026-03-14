# Guía para Ejecutar OmniMon en Modo Desarrollo

## Prerequisitos

**ANTES de ejecutar OmniMon**, asegúrate de tener instalado:

| Herramienta | Windows | macOS | Linux |
|-------------|---------|-------|-------|
| Node.js v18+ | Requerido | Requerido | Requerido |
| Rust (cargo + rustc) | **CRÍTICO** | **CRÍTICO** | **CRÍTICO** |
| Bun | Recomendado | Recomendado | Recomendado |
| Visual Studio Build Tools | **CRÍTICO** | — | — |
| Xcode Command Line Tools | — | **CRÍTICO** | — |
| libwebkit2gtk-4.1-dev + deps | — | — | **CRÍTICO** |

Si falta algo, consulta **[INSTALACION_PREREQUISITOS.md](./INSTALACION_PREREQUISITOS.md)**.

---

## Ejecutar OmniMon

### Windows

**Opción 1 — Script automático (recomendado):**
```powershell
cd omnimon\v4
.\EJECUTAR_OMNIMON.bat
```
Este script auto-detecta Bun, Cargo y Node, instala dependencias si es necesario, y lanza la app.

**Opción 2 — PowerShell:**
```powershell
cd omnimon\v4
.\run-dev-auto.ps1
```

**Opción 3 — Manual:**
```powershell
cd omnimon\v4\apps\desktop
bun install
bun run tauri dev
```

### macOS

**Opción 1 — Make (recomendado):**
```bash
cd omnimon/v4
./setup-dev.sh   # Solo la primera vez
make dev
```

**Opción 2 — Manual:**
```bash
cd omnimon/v4/apps/desktop
bun install      # o npm install
bun run tauri dev
```

### Linux

**Opción 1 — Make (recomendado):**
```bash
cd omnimon/v4
./setup-dev.sh   # Solo la primera vez
make dev
```

**Opción 2 — Manual:**
```bash
cd omnimon/v4/apps/desktop
bun install      # o npm install
bun run tauri dev
```

**Nota:** Para captura de red en Linux, ejecuta con `sudo` o configura capabilities:
```bash
sudo setcap cap_net_raw+ep target/debug/omnimon-desktop
```

---

## Qué Esperar al Ejecutar

Al iniciar, verás algo como:

```
     Running BeforeDevCommand (`bun run dev`)
     Running DevCommand (`cargo run --no-default-features`)
$ vite

  VITE v6.x.x  ready in XXXms

  ➜  Local:   http://localhost:1420/

    Compiling omnimon-desktop v0.1.0
    Finished `dev` profile [unoptimized + debuginfo]
```

Luego se abrirá la **ventana nativa de OmniMon** (no un navegador).

---

## Verificar Funcionalidades

### 1. Global Hotkey — Ctrl+Alt+O
- Con OmniMon corriendo, presiona `Ctrl+Alt+O`
- La ventana debe aparecer/desaparecer
- Funciona en Windows y Linux (macOS no soporta global hotkeys de esta forma)

### 2. Tray Tooltip Dinámico
- Pasa el mouse sobre el icono del system tray
- Debería mostrar: `OmniMon - CPU: X.X% | RAM: X.XGB (XX%)`
- Se actualiza cada 5 segundos

### 3. Cierre Completo
- Presiona el botón X de la ventana
- Verifica que el proceso realmente termine (Task Manager / `ps aux`)
- En Windows: la app sale completamente (no se queda en background)

### 4. Conexiones de Red
- Abre la sección de Network
- Deberías ver conexiones TCP/UDP
- Si no aparecen: ejecuta como Administrador (Windows) o con `sudo` (Linux)

### 5. Browser Tabs
La detección de pestañas funciona **automáticamente** en Windows y macOS:
- **Windows**: Usa Windows UI Automation API — detecta tabs por título sin configuración
- **macOS**: Usa AppleScript — detecta tabs con título y URL sin configuración
- **Linux**: Requiere Chrome DevTools Protocol (CDP):
  - Lanza Chrome con `google-chrome --remote-debugging-port=9222`

#### Limitaciones por plataforma
| Plataforma | Títulos | URLs | Cerrar tabs | Requiere flags |
|------------|---------|------|-------------|----------------|
| macOS | ✅ | ✅ | ✅ | No |
| Windows | ✅ | ❌ (solo con CDP) | ❌ (solo con CDP) | No |
| Linux | ✅ (CDP) | ✅ (CDP) | ✅ (CDP) | Sí |

> **Nota Windows:** Para funcionalidad completa (URLs + cerrar tabs), puedes opcionalmente lanzar Chrome con CDP:
> `"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222 --user-data-dir="%LOCALAPPDATA%\Google\Chrome\User Data Debug"`

---

## Debug y Logging

Para ver logs detallados:

```bash
# Todas las plataformas
RUST_LOG=debug bun run tauri dev

# Solo errores
RUST_LOG=error bun run tauri dev

# Solo módulo de red
RUST_LOG=network=debug bun run tauri dev
```

**En PowerShell:**
```powershell
$env:RUST_LOG="debug"
bun run tauri dev
```

**Logs esperados:**
```
[network] Windows native API: got 45 connections
[network] Found 42 TCP connections, 3 UDP connections
Global hotkey Ctrl+Alt+O registered successfully
```

---

## Troubleshooting

### "Port 1420 is already in use"

Un proceso anterior de Vite sigue corriendo.

**Linux/macOS:**
```bash
lsof -ti:1420 | xargs kill -9
```

**Windows (PowerShell):**
```powershell
Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue |
  Select-Object -ExpandProperty OwningProcess |
  ForEach-Object { Stop-Process -Id $_ -Force }
```

**Windows (CMD):**
```cmd
for /f "tokens=5" %a in ('netstat -ano ^| findstr :1420') do taskkill /F /PID %a
```

### "cargo: command not found"
- Cierra y reabre la terminal
- O ejecuta: `source "$HOME/.cargo/env"` (macOS/Linux)
- Windows: `$env:PATH += ";$env:USERPROFILE\.cargo\bin"`

### "tauri: command not found"
Las dependencias de npm/bun no están instaladas:
```bash
cd apps/desktop
bun install   # o npm install
```

### "LINK: fatal error" o "linker not found" (Windows)
Visual Studio Build Tools falta o le falta el componente C++. Instala con "Desktop development with C++".

### "Package libwebkit2gtk not found" (Linux)
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
```

### Compilación muy lenta (primera vez)
Es normal. La primera compilación de Rust descarga y compila ~200+ crates. Las siguientes compilaciones son incrementales y mucho más rápidas.

### La app se abre pero la pantalla está en blanco
- Verifica que Vite esté corriendo (debe mostrar `http://localhost:1420/`)
- Si el puerto está mal, revisa `apps/desktop/src-tauri/tauri.conf.json`

---

## Build para Producción

```bash
cd omnimon/v4/apps/desktop

# Build de debug (más rápido, sin bundle)
bun run tauri build -- --debug --no-bundle

# Build de release completo
bun run tauri build
```

Los binarios se generan en `v4/target/release/` o `v4/target/debug/`.

---

## Comandos de Desarrollo Útiles

```bash
cd omnimon/v4

# Desarrollo
make dev                                    # Full-stack dev mode
bun run tauri dev                           # Alternativa sin Make

# Testing
bun run test                                # Frontend unit tests
cargo test --workspace                      # Rust tests
make test-all                               # Todo: fmt + clippy + tests

# Linting
cargo fmt --check                           # Formato Rust
cargo clippy -- -D warnings                 # Lint Rust

# Verificación rápida
cargo check --workspace                     # Type check Rust
bun run build                               # Build frontend
```
