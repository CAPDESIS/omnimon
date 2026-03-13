# Script completo de instalacion de prerequisitos para OmniMon
Write-Host "====================================================" -ForegroundColor Cyan
Write-Host "  Instalador Completo de Prerequisitos - OmniMon" -ForegroundColor Cyan
Write-Host "====================================================" -ForegroundColor Cyan
Write-Host ""

$errorsFound = $false

# ============================================
# Paso 1: Verificar Node.js
# ============================================
Write-Host "[1/4] Verificando Node.js..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version 2>&1
    $npmVersion = npm --version 2>&1
    Write-Host "  Node.js: $nodeVersion" -ForegroundColor Green
    Write-Host "  npm: $npmVersion" -ForegroundColor Green
    Write-Host "  [OK] Node.js ya esta instalado" -ForegroundColor Green
} catch {
    Write-Host "  [X] Node.js NO esta instalado" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Por favor instala Node.js manualmente:" -ForegroundColor Yellow
    Write-Host "    1. Visita: https://nodejs.org/" -ForegroundColor White
    Write-Host "    2. Descarga la version LTS" -ForegroundColor White
    Write-Host "    3. Ejecuta el instalador" -ForegroundColor White
    Write-Host "    4. Reinicia PowerShell" -ForegroundColor White
    Write-Host ""
    $errorsFound = $true
}

Write-Host ""

# ============================================
# Paso 2: Verificar Visual Studio Build Tools
# ============================================
Write-Host "[2/4] Verificando Visual Studio Build Tools..." -ForegroundColor Yellow

if (Test-Path "C:\Program Files\Microsoft Visual Studio") {
    Write-Host "  [OK] Visual Studio Build Tools instalado" -ForegroundColor Green
} else {
    Write-Host "  [X] Visual Studio Build Tools NO instalado" -ForegroundColor Red
    Write-Host ""
    Write-Host "  CRITICO: Se requiere para compilar en Windows" -ForegroundColor Red
    Write-Host ""
    Write-Host "  Por favor instala manualmente:" -ForegroundColor Yellow
    Write-Host "    1. Descarga: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor White
    Write-Host "    2. Ejecuta el instalador" -ForegroundColor White
    Write-Host "    3. Selecciona: 'Desktop development with C++'" -ForegroundColor White
    Write-Host "    4. Marca: MSVC, Windows SDK, CMake tools" -ForegroundColor White
    Write-Host "    5. Instala (30-60 minutos)" -ForegroundColor White
    Write-Host "    6. Reinicia el sistema" -ForegroundColor White
    Write-Host ""
    $errorsFound = $true
}

Write-Host ""

# ============================================
# Paso 3: Instalar Rust
# ============================================
Write-Host "[3/4] Verificando Rust..." -ForegroundColor Yellow

try {
    $cargoVersion = cargo --version 2>&1
    Write-Host "  [OK] Rust ya esta instalado: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "  Rust NO esta instalado - Instalando..." -ForegroundColor Yellow
    Write-Host ""

    try {
        # Descargar rustup
        $rustupUrl = "https://win.rustup.rs/x86_64"
        $rustupPath = "$env:TEMP\rustup-init.exe"

        Write-Host "  Descargando rustup..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath -UseBasicParsing

        Write-Host "  Instalando Rust (puede tomar 10-15 minutos)..." -ForegroundColor Cyan
        Write-Host "  Por favor espera..." -ForegroundColor DarkYellow

        # Ejecutar instalador silencioso
        Start-Process -FilePath $rustupPath -ArgumentList "-y" -Wait -NoNewWindow

        # Agregar al PATH de la sesion actual
        $cargoPath = "$env:USERPROFILE\.cargo\bin"
        $env:PATH = "$cargoPath;$env:PATH"

        Write-Host ""
        Write-Host "  [OK] Rust instalado exitosamente!" -ForegroundColor Green

        # Verificar
        $cargoVersion = & "$cargoPath\cargo.exe" --version 2>&1
        Write-Host "  Version: $cargoVersion" -ForegroundColor Green

    } catch {
        Write-Host ""
        Write-Host "  [X] ERROR: No se pudo instalar Rust automaticamente" -ForegroundColor Red
        Write-Host "  $($_.Exception.Message)" -ForegroundColor Red
        Write-Host ""
        Write-Host "  Instalacion manual:" -ForegroundColor Yellow
        Write-Host "    1. Descarga: https://rustup.rs/" -ForegroundColor White
        Write-Host "    2. Ejecuta rustup-init.exe" -ForegroundColor White
        Write-Host "    3. Presiona ENTER para instalacion por defecto" -ForegroundColor White
        Write-Host ""
        $errorsFound = $true
    }
}

Write-Host ""

# ============================================
# Paso 4: Instalar Bun (Opcional)
# ============================================
Write-Host "[4/4] Verificando Bun (opcional)..." -ForegroundColor Yellow

try {
    $bunVersion = bun --version 2>&1
    Write-Host "  [OK] Bun ya esta instalado: $bunVersion" -ForegroundColor Green
} catch {
    Write-Host "  Bun NO esta instalado" -ForegroundColor DarkYellow
    Write-Host ""

    $installBun = Read-Host "  Deseas instalar Bun? (s/N)"

    if ($installBun -eq "s" -or $installBun -eq "S") {
        try {
            Write-Host "  Instalando Bun..." -ForegroundColor Cyan
            powershell -c "irm bun.sh/install.ps1|iex"
            Write-Host "  [OK] Bun instalado exitosamente!" -ForegroundColor Green
        } catch {
            Write-Host "  [!] No se pudo instalar Bun automaticamente" -ForegroundColor DarkYellow
            Write-Host "  No es critico - puedes usar npm en su lugar" -ForegroundColor DarkYellow
        }
    } else {
        Write-Host "  [!] Saltando instalacion de Bun (npm sera usado)" -ForegroundColor DarkYellow
    }
}

Write-Host ""
Write-Host "====================================================" -ForegroundColor Cyan
Write-Host "  Resumen de Instalacion" -ForegroundColor Cyan
Write-Host "====================================================" -ForegroundColor Cyan
Write-Host ""

# Verificacion final
$allOk = $true

Write-Host "Estado final de prerequisitos:" -ForegroundColor Yellow
Write-Host ""

# Node.js
Write-Host "  Node.js: " -NoNewline
try {
    $null = node --version 2>&1
    Write-Host "OK" -ForegroundColor Green
} catch {
    Write-Host "FALTA" -ForegroundColor Red
    $allOk = $false
}

# Rust
Write-Host "  Rust:    " -NoNewline
try {
    $null = cargo --version 2>&1
    Write-Host "OK" -ForegroundColor Green
} catch {
    Write-Host "FALTA" -ForegroundColor Red
    $allOk = $false
}

# VS Build Tools
Write-Host "  VS Build Tools: " -NoNewline
if (Test-Path "C:\Program Files\Microsoft Visual Studio") {
    Write-Host "OK" -ForegroundColor Green
} else {
    Write-Host "FALTA" -ForegroundColor Red
    $allOk = $false
}

# Bun (opcional)
Write-Host "  Bun (opcional): " -NoNewline
try {
    $null = bun --version 2>&1
    Write-Host "OK" -ForegroundColor Green
} catch {
    Write-Host "No instalado (npm funciona)" -ForegroundColor DarkYellow
}

Write-Host ""

if ($allOk) {
    Write-Host "====================================================" -ForegroundColor Green
    Write-Host "  TODOS LOS PREREQUISITOS INSTALADOS" -ForegroundColor Green
    Write-Host "====================================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "SIGUIENTE PASO:" -ForegroundColor Cyan
    Write-Host "  1. Cierra esta ventana" -ForegroundColor White
    Write-Host "  2. Abre PowerShell nuevamente" -ForegroundColor White
    Write-Host "  3. Ejecuta: .\EJECUTAR_OMNIMON.bat" -ForegroundColor Yellow
    Write-Host ""
} else {
    Write-Host "====================================================" -ForegroundColor Red
    Write-Host "  FALTAN ALGUNOS PREREQUISITOS" -ForegroundColor Red
    Write-Host "====================================================" -ForegroundColor Red
    Write-Host ""
    Write-Host "Por favor instala las herramientas marcadas como FALTA" -ForegroundColor Yellow
    Write-Host "Consulta: INSTALACION_PREREQUISITOS.md para mas detalles" -ForegroundColor Yellow
    Write-Host ""
}

pause
