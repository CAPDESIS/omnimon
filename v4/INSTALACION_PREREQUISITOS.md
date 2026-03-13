# Guía de Instalación de Prerequisitos para OmniMon

Esta guía te ayudará a instalar todas las herramientas necesarias para ejecutar OmniMon en modo desarrollo en Windows.

---

## 📋 Prerequisitos Necesarios

Para ejecutar OmniMon en modo desarrollo necesitas:

| Herramienta | Versión Mínima | Estado | Propósito |
|-------------|----------------|--------|-----------|
| **Node.js** | v18+ | ✅ Recomendado | Runtime de JavaScript para el frontend |
| **Rust** | Latest stable | ⚠️ **CRÍTICO** | Compilador para el backend Tauri |
| **Visual Studio Build Tools** | 2019+ | ⚠️ **CRÍTICO** | Herramientas de compilación de C++ |
| **Bun** | Latest | 📦 Opcional | Package manager (alternativa a npm) |

---

## 🚀 Instalación Automática (Recomendado)

Hemos creado scripts automáticos para facilitar la instalación:

### Opción 1: Script Todo-en-Uno

```powershell
cd C:\Users\ohcho\Documents\Apps\omnimon\v4
.\instalar-todo.ps1
```

Este script instalará automáticamente:
- ✅ Rust (rustup + cargo)
- ✅ Bun (opcional)
- ✅ Verificará Node.js y npm
- ✅ Verificará Visual Studio Build Tools

### Opción 2: Instalación Individual

**Instalar solo Rust:**
```powershell
.\instalar-rust.ps1
```

**Instalar solo Bun:**
```powershell
.\instalar-bun.ps1
```

---

## 🔧 Instalación Manual

Si prefieres instalar manualmente cada herramienta:

### 1️⃣ Node.js (Probablemente ya instalado)

**Verificar si está instalado:**
```powershell
node --version
npm --version
```

**Instalar si falta:**
1. Descarga desde: https://nodejs.org/
2. Ejecuta el instalador
3. Selecciona "Automatically install necessary tools" durante la instalación
4. Reinicia PowerShell

---

### 2️⃣ Rust (CRÍTICO - Necesario para Tauri)

**Verificar si está instalado:**
```powershell
cargo --version
rustc --version
```

**Instalar:**

#### Método A: Instalador Oficial (Recomendado)
1. Descarga: https://rustup.rs/
2. Ejecuta `rustup-init.exe`
3. Presiona **ENTER** para instalación por defecto
4. Espera a que termine (puede tomar 5-10 minutos)
5. Cierra y reabre PowerShell

#### Método B: PowerShell One-liner
```powershell
Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile "$env:TEMP\rustup-init.exe"
& "$env:TEMP\rustup-init.exe" -y
```

**Agregar al PATH:**
Si Rust se instaló pero PowerShell no lo encuentra:
```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

Para hacerlo permanente:
```powershell
[System.Environment]::SetEnvironmentVariable('Path', $env:Path + ";$env:USERPROFILE\.cargo\bin", 'User')
```

---

### 3️⃣ Visual Studio Build Tools (CRÍTICO para Windows)

**Verificar si está instalado:**
```powershell
if (Test-Path "C:\Program Files\Microsoft Visual Studio") {
    Write-Host "Visual Studio Build Tools: Instalado"
} else {
    Write-Host "Visual Studio Build Tools: NO instalado"
}
```

**Instalar si falta:**

1. Descarga: https://visualstudio.microsoft.com/visual-cpp-build-tools/
2. Ejecuta el instalador
3. Selecciona: **"Desktop development with C++"**
4. Marca estas opciones:
   - ✅ MSVC v143 - VS 2022 C++ x64/x86 build tools
   - ✅ Windows 10 SDK (latest)
   - ✅ C++ CMake tools for Windows
5. Instala (puede tomar 30-60 minutos)
6. Reinicia el sistema

**O instala Visual Studio Community completo:**
- Descarga: https://visualstudio.microsoft.com/downloads/
- Durante instalación, selecciona: "Desktop development with C++"

---

### 4️⃣ Bun (Opcional - Package Manager)

**Verificar si está instalado:**
```powershell
bun --version
```

**Instalar:**
```powershell
powershell -c "irm bun.sh/install.ps1|iex"
```

**O descarga desde:** https://bun.sh/

**Nota:** Bun es el package manager preferido del proyecto según `CLAUDE.md`, pero **npm también funciona perfectamente**.

---

## ✅ Verificación Completa

Después de instalar todo, ejecuta este comando para verificar:

```powershell
# Crear script de verificación temporal
@"
Write-Host '================================' -ForegroundColor Cyan
Write-Host '  Verificación de Prerequisitos' -ForegroundColor Cyan
Write-Host '================================' -ForegroundColor Cyan
Write-Host ''

`$allGood = `$true

# Node.js
Write-Host '[1] Node.js: ' -NoNewline -ForegroundColor Yellow
try {
    `$nodeVersion = node --version 2>&1
    Write-Host "✅ `$nodeVersion" -ForegroundColor Green
} catch {
    Write-Host '❌ NO INSTALADO' -ForegroundColor Red
    `$allGood = `$false
}

# npm
Write-Host '[2] npm: ' -NoNewline -ForegroundColor Yellow
try {
    `$npmVersion = npm --version 2>&1
    Write-Host "✅ `$npmVersion" -ForegroundColor Green
} catch {
    Write-Host '❌ NO INSTALADO' -ForegroundColor Red
    `$allGood = `$false
}

# Rust/Cargo
Write-Host '[3] Rust/Cargo: ' -NoNewline -ForegroundColor Yellow
try {
    `$cargoVersion = cargo --version 2>&1
    Write-Host "✅ `$cargoVersion" -ForegroundColor Green
} catch {
    Write-Host '❌ NO INSTALADO (CRÍTICO)' -ForegroundColor Red
    `$allGood = `$false
}

# Visual Studio
Write-Host '[4] Visual Studio Build Tools: ' -NoNewline -ForegroundColor Yellow
if (Test-Path 'C:\Program Files\Microsoft Visual Studio') {
    Write-Host '✅ Instalado' -ForegroundColor Green
} else {
    Write-Host '❌ NO INSTALADO (CRÍTICO)' -ForegroundColor Red
    `$allGood = `$false
}

# Bun (opcional)
Write-Host '[5] Bun (opcional): ' -NoNewline -ForegroundColor Yellow
try {
    `$bunVersion = bun --version 2>&1
    Write-Host "✅ `$bunVersion" -ForegroundColor Green
} catch {
    Write-Host '⚠️  No instalado (npm funciona como alternativa)' -ForegroundColor DarkYellow
}

Write-Host ''
Write-Host '================================' -ForegroundColor Cyan

if (`$allGood) {
    Write-Host '  ✅ TODOS LOS PREREQUISITOS OK' -ForegroundColor Green
    Write-Host '  Puedes ejecutar OmniMon ahora' -ForegroundColor Green
} else {
    Write-Host '  ❌ FALTAN PREREQUISITOS' -ForegroundColor Red
    Write-Host '  Instala las herramientas marcadas con ❌' -ForegroundColor Red
}

Write-Host '================================' -ForegroundColor Cyan
"@ | Out-File -FilePath "$env:TEMP\verificar.ps1" -Encoding UTF8

powershell -ExecutionPolicy Bypass -File "$env:TEMP\verificar.ps1"
```

---

## 🎯 Después de Instalar Todo

Una vez que todos los prerequisitos estén instalados:

1. **Cierra y reabre PowerShell** (para que el PATH se actualice)

2. **Verifica nuevamente:**
   ```powershell
   cargo --version
   node --version
   npm --version
   ```

3. **Ejecuta OmniMon:**
   ```powershell
   cd C:\Users\ohcho\Documents\Apps\omnimon\v4
   .\EJECUTAR_OMNIMON.bat
   ```

---

## 🐛 Troubleshooting

### "cargo: command not found" después de instalar Rust

**Solución 1:** Cierra y reabre PowerShell

**Solución 2:** Agrega manualmente al PATH:
```powershell
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
```

**Solución 3:** Verifica que se instaló correctamente:
```powershell
Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe"
```

Si devuelve `True`, el problema es solo del PATH.

---

### "LINK: fatal error LNK1181" durante compilación

Esto significa que **Visual Studio Build Tools no está instalado** o le falta el componente C++.

**Solución:**
1. Instala Visual Studio Build Tools
2. Asegúrate de seleccionar "Desktop development with C++"
3. Reinicia el sistema después de la instalación

---

### "error: linker `link.exe` not found"

Mismo problema que el anterior - necesitas Visual Studio Build Tools.

**Solución rápida:**
Ejecuta desde **"Developer PowerShell for VS"** en vez de PowerShell normal.

---

### La instalación de Rust tarda mucho

Es normal - Rust descarga varios componentes (rustc, cargo, clippy, rustfmt, etc.) que pueden sumar varios GB.

**Tiempo típico:**
- Descarga: 5-10 minutos (depende de tu conexión)
- Instalación: 2-5 minutos
- **Total: ~10-15 minutos**

---

## 📚 Recursos Adicionales

- **Tauri Prerequisites:** https://tauri.app/start/prerequisites/
- **Rust Installation:** https://www.rust-lang.org/tools/install
- **Node.js Downloads:** https://nodejs.org/
- **Visual Studio Build Tools:** https://visualstudio.microsoft.com/downloads/

---

## 🎉 Todo Listo

Si todos los prerequisitos están instalados correctamente, dirígete a:

📄 **[EJECUTAR_DEV.md](./EJECUTAR_DEV.md)** - Guía para ejecutar OmniMon en modo desarrollo

O ejecuta directamente:
```powershell
.\EJECUTAR_OMNIMON.bat
```
