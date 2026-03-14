# Guía de Instalación de Prerequisitos para OmniMon

Guía completa para instalar todas las herramientas necesarias en **Windows**, **macOS** y **Linux**.

---

## Prerequisitos Necesarios

| Herramienta | Versión Mínima | Importancia | Propósito |
|-------------|----------------|-------------|-----------|
| **Node.js** | v18+ | Recomendado | Runtime de JavaScript para el frontend |
| **Rust** | 1.75+ stable | **CRÍTICO** | Compilador para el backend Tauri |
| **Bun** | Latest | Recomendado | Package manager del proyecto |
| **Visual Studio Build Tools** | 2019+ | **CRÍTICO** (solo Windows) | Herramientas de compilación C++ |
| **Xcode Command Line Tools** | Latest | **CRÍTICO** (solo macOS) | Herramientas de compilación |
| **libwebkit2gtk + deps** | 4.1+ | **CRÍTICO** (solo Linux) | WebView para Tauri |

---

## Instalación por Sistema Operativo

### Windows

#### 1. Node.js

**Verificar:**
```powershell
node --version
npm --version
```

**Instalar si falta:**
1. Descarga desde https://nodejs.org/ (versión LTS)
2. Ejecuta el instalador
3. Selecciona "Automatically install necessary tools"
4. Reinicia PowerShell

#### 2. Rust (CRÍTICO)

**Verificar:**
```powershell
cargo --version
rustc --version
```

**Instalar:**

**Opción A — Instalador oficial (recomendado):**
1. Descarga https://rustup.rs/ → ejecuta `rustup-init.exe`
2. Presiona **ENTER** para instalación por defecto
3. Espera a que termine (~10 minutos)
4. Cierra y reabre PowerShell

**Opción B — PowerShell one-liner:**
```powershell
Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile "$env:TEMP\rustup-init.exe"
& "$env:TEMP\rustup-init.exe" -y
```

**Si cargo no se encuentra después de instalar:**
```powershell
# Temporal (sesión actual)
$env:PATH += ";$env:USERPROFILE\.cargo\bin"

# Permanente
[System.Environment]::SetEnvironmentVariable('Path', $env:Path + ";$env:USERPROFILE\.cargo\bin", 'User')
```

#### 3. Visual Studio Build Tools (CRÍTICO)

**Verificar:**
```powershell
if (Test-Path "C:\Program Files\Microsoft Visual Studio") { "Instalado" } else { "NO instalado" }
```

**Instalar:**
1. Descarga https://visualstudio.microsoft.com/visual-cpp-build-tools/
2. Ejecuta el instalador
3. Selecciona **"Desktop development with C++"**
4. Marca:
   - MSVC v143 - VS 2022 C++ x64/x86 build tools
   - Windows 10/11 SDK (latest)
   - C++ CMake tools for Windows
5. Instala y reinicia el sistema

**Alternativa:** Instala Visual Studio Community completo desde https://visualstudio.microsoft.com/downloads/ seleccionando "Desktop development with C++".

#### 4. Bun

**Verificar:**
```powershell
bun --version
```

**Instalar:**
```powershell
powershell -c "irm bun.sh/install.ps1|iex"
```

#### 5. WebView2 Runtime

Windows 10/11 generalmente ya lo tiene. Si no:
- Descarga desde https://developer.microsoft.com/en-us/microsoft-edge/webview2/

#### Instalación automática (Windows)

Puedes usar el script todo-en-uno:
```powershell
cd omnimon/v4
.\instalar-todo.ps1
```

---

### macOS

#### 1. Xcode Command Line Tools (CRÍTICO)

**Verificar:**
```bash
xcode-select -p
```

**Instalar:**
```bash
xcode-select --install
```

#### 2. Homebrew (recomendado)

Si no tienes Homebrew:
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

#### 3. Node.js

**Verificar:**
```bash
node --version
npm --version
```

**Instalar:**
```bash
brew install node
```

O descarga desde https://nodejs.org/

#### 4. Rust (CRÍTICO)

**Verificar:**
```bash
cargo --version
rustc --version
```

**Instalar:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

#### 5. Bun

**Verificar:**
```bash
bun --version
```

**Instalar:**
```bash
curl -fsSL https://bun.sh/install | bash
```

O con Homebrew:
```bash
brew install oven-sh/bun/bun
```

#### Resumen macOS
```bash
# Todo en un solo bloque
xcode-select --install
brew install node
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
curl -fsSL https://bun.sh/install | bash
```

---

### Linux (Debian/Ubuntu)

#### 1. Dependencias del sistema (CRÍTICO)

Tauri requiere varias bibliotecas del sistema:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev
```

**Para Fedora/RHEL:**
```bash
sudo dnf install -y \
  gcc gcc-c++ make \
  openssl-devel \
  gtk3-devel \
  webkit2gtk4.1-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  patchelf \
  javascriptcoregtk4.1-devel \
  libsoup3-devel
```

**Para Arch Linux:**
```bash
sudo pacman -S --needed \
  base-devel \
  curl wget file \
  openssl \
  gtk3 \
  webkit2gtk-4.1 \
  libappindicator-gtk3 \
  librsvg \
  patchelf \
  libsoup3
```

#### 2. Node.js

**Verificar:**
```bash
node --version
npm --version
```

**Instalar (vía NodeSource):**
```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
```

O usa [nvm](https://github.com/nvm-sh/nvm):
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
source ~/.bashrc
nvm install --lts
```

#### 3. Rust (CRÍTICO)

**Verificar:**
```bash
cargo --version
rustc --version
```

**Instalar:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

#### 4. Bun

**Verificar:**
```bash
bun --version
```

**Instalar:**
```bash
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc
```

#### Resumen Linux (Debian/Ubuntu)
```bash
# Dependencias del sistema
sudo apt update && sudo apt install -y build-essential curl wget file \
  libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf libjavascriptcoregtk-4.1-dev libsoup-3.0-dev

# Node.js
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Bun
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc
```

---

## Verificación Completa

Ejecuta estos comandos en cualquier plataforma para verificar que todo está instalado:

```bash
echo "=== Verificación de Prerequisitos ==="
echo ""

echo -n "Node.js: "
node --version 2>/dev/null || echo "NO INSTALADO"

echo -n "npm: "
npm --version 2>/dev/null || echo "NO INSTALADO"

echo -n "Rust/Cargo: "
cargo --version 2>/dev/null || echo "NO INSTALADO (CRÍTICO)"

echo -n "rustc: "
rustc --version 2>/dev/null || echo "NO INSTALADO (CRÍTICO)"

echo -n "Bun: "
bun --version 2>/dev/null || echo "No instalado (recomendado)"

echo ""
echo "=== Fin de verificación ==="
```

**En PowerShell (Windows):**
```powershell
Write-Host "=== Verificacion de Prerequisitos ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Node.js: " -NoNewline; try { node --version } catch { Write-Host "NO INSTALADO" -ForegroundColor Red }
Write-Host "npm: " -NoNewline; try { npm --version } catch { Write-Host "NO INSTALADO" -ForegroundColor Red }
Write-Host "Cargo: " -NoNewline; try { cargo --version } catch { Write-Host "NO INSTALADO (CRITICO)" -ForegroundColor Red }
Write-Host "rustc: " -NoNewline; try { rustc --version } catch { Write-Host "NO INSTALADO (CRITICO)" -ForegroundColor Red }
Write-Host "Bun: " -NoNewline; try { bun --version } catch { Write-Host "No instalado (recomendado)" -ForegroundColor Yellow }
Write-Host ""
Write-Host "=== Fin ===" -ForegroundColor Cyan
```

---

## Ejecutar OmniMon

Una vez que todos los prerequisitos están instalados:

### Windows
```powershell
cd omnimon/v4
.\EJECUTAR_OMNIMON.bat
```

### macOS / Linux
```bash
cd omnimon/v4
./setup-dev.sh   # Solo la primera vez (instala deps de npm)
make dev
```

**Alternativa universal:**
```bash
cd omnimon/v4/apps/desktop
bun install          # o npm install
bun run tauri dev    # o npx tauri dev
```

---

## Troubleshooting

### Todas las plataformas

#### "cargo: command not found"
- Rust no está instalado o no está en el PATH
- **Fix:** Cierra y reabre la terminal, o ejecuta `source "$HOME/.cargo/env"`

#### "Port 1420 is already in use"
- Hay un proceso previo de Vite ocupando el puerto
- **Fix:** Mata procesos anteriores:
  ```bash
  # Linux/macOS
  lsof -ti:1420 | xargs kill -9

  # Windows (PowerShell)
  Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty OwningProcess |
    ForEach-Object { Stop-Process -Id $_ -Force }
  ```

#### Compilación Rust tarda mucho
La primera compilación descarga y compila todas las dependencias. Es normal que tarde 5-15 minutos. Las siguientes compilaciones serán mucho más rápidas (compilación incremental).

### Windows

#### "LINK: fatal error LNK1181" o "linker `link.exe` not found"
- Visual Studio Build Tools no está instalado o falta el componente C++
- **Fix:** Instala Build Tools con "Desktop development with C++"
- **Alt:** Ejecuta desde "Developer PowerShell for VS"

#### "cargo no se reconoce" después de instalar Rust
```powershell
# Verifica que existe
Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"

# Si devuelve True, agrega al PATH
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

### macOS

#### "xcrun: error: invalid active developer path"
```bash
xcode-select --install
```

#### Problemas con permisos de Homebrew
```bash
sudo chown -R $(whoami) /usr/local/share/zsh /usr/local/share/zsh/site-functions
```

### Linux

#### "Package libwebkit2gtk-4.1-dev not found"
En distribuciones más antiguas puede llamarse `libwebkit2gtk-4.0-dev`:
```bash
sudo apt install libwebkit2gtk-4.0-dev
```

#### "No suitable WebView2 loader found"
Linux no usa WebView2 — esto indica un problema de configuración de Tauri. Asegúrate de tener `libwebkit2gtk` instalado.

#### Errores de OpenSSL
```bash
sudo apt install libssl-dev pkg-config
```

---

## Recursos Adicionales

- **Tauri Prerequisites (oficial):** https://tauri.app/start/prerequisites/
- **Rust Installation:** https://www.rust-lang.org/tools/install
- **Node.js Downloads:** https://nodejs.org/
- **Bun:** https://bun.sh/
- **Visual Studio Build Tools:** https://visualstudio.microsoft.com/downloads/

---

## Siguiente Paso

Una vez instalado todo, consulta **[EJECUTAR_DEV.md](./EJECUTAR_DEV.md)** para la guía de ejecución.
