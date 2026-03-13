# Script mejorado para ejecutar OmniMon en modo desarrollo
# Auto-detecta y agrega Cargo al PATH si es necesario

Write-Host "===================================" -ForegroundColor Cyan
Write-Host "  OmniMon - Modo Desarrollo Auto" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

# Cambiar al directorio de la app
$appDir = "C:\Users\ohcho\Documents\Apps\omnimon\v4\apps\desktop"
Set-Location $appDir

Write-Host "[1/5] Auto-detectando herramientas..." -ForegroundColor Yellow

# Buscar y agregar Bun al PATH
$bunPath = "$env:USERPROFILE\.bun\bin"
if (Test-Path "$bunPath\bun.exe") {
    $env:PATH = "$bunPath;$env:PATH"
    Write-Host "  [OK] Bun agregado al PATH" -ForegroundColor Green
}

# Buscar cargo en ubicaciones comunes
$cargoLocations = @(
    "$env:USERPROFILE\.cargo\bin\cargo.exe",
    "C:\Users\$env:USERNAME\.cargo\bin\cargo.exe",
    "$env:CARGO_HOME\bin\cargo.exe"
)

$cargoPath = $null
foreach ($loc in $cargoLocations) {
    if (Test-Path $loc) {
        $cargoPath = Split-Path $loc -Parent
        Write-Host "  [OK] Cargo encontrado en: $cargoPath" -ForegroundColor Green
        $env:PATH = "$cargoPath;$env:PATH"
        break
    }
}

# Buscar en Program Files para Visual Studio
if (-not $cargoPath) {
    $vsLocations = Get-ChildItem "C:\Program Files\Microsoft Visual Studio" -Recurse -Filter "cargo.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($vsLocations) {
        $cargoPath = Split-Path $vsLocations.FullName -Parent
        Write-Host "  [OK] Cargo encontrado en VS: $cargoPath" -ForegroundColor Green
        $env:PATH = "$cargoPath;$env:PATH"
    }
}

# Verificar que cargo este disponible
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "ERROR: No se pudo encontrar Cargo" -ForegroundColor Red
    Write-Host ""
    Write-Host "Por favor ejecuta CUALQUIERA de estas opciones:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  OPCION A) Abre 'Developer PowerShell for VS' y ejecuta:" -ForegroundColor Cyan
    Write-Host "    cd C:\Users\ohcho\Documents\Apps\omnimon\v4" -ForegroundColor White
    Write-Host "    .\run-dev.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "  OPCION B) Abre PowerShell normal y ejecuta:" -ForegroundColor Cyan
    Write-Host "    cd C:\Users\ohcho\Documents\Apps\omnimon\v4\apps\desktop\src-tauri" -ForegroundColor White
    Write-Host "    cargo run" -ForegroundColor White
    Write-Host ""
    Write-Host "  OPCION C) Busca donde esta instalado Rust y agregalo al PATH" -ForegroundColor Cyan
    Write-Host ""
    pause
    exit 1
}

# Verificar que node este disponible
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Node.js no esta instalado o no esta en el PATH" -ForegroundColor Red
    pause
    exit 1
}

Write-Host "  [OK] Cargo: $(cargo --version)" -ForegroundColor Green
Write-Host "  [OK] Node: $(node --version)" -ForegroundColor Green
Write-Host ""

# Instalar dependencias si no existen
if (-not (Test-Path "node_modules")) {
    Write-Host "[2/5] Instalando dependencias de npm..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Fallo la instalacion de dependencias" -ForegroundColor Red
        pause
        exit 1
    }
} else {
    Write-Host "[2/5] Dependencias ya instaladas" -ForegroundColor Green
}

Write-Host ""
Write-Host "[3/5] Verificando Tauri CLI..." -ForegroundColor Yellow

# Verificar que @tauri-apps/cli este instalado
if (-not (Test-Path "node_modules\@tauri-apps\cli")) {
    Write-Host "  Instalando @tauri-apps/cli..." -ForegroundColor Yellow
    npm install --save-dev @tauri-apps/cli@2
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Fallo la instalacion de Tauri CLI" -ForegroundColor Red
        pause
        exit 1
    }
}

Write-Host "  [OK] Tauri CLI instalado" -ForegroundColor Green
Write-Host ""

Write-Host "[4/5] Verificando Bun..." -ForegroundColor Yellow

# Verificar que bun este disponible ahora
try {
    $bunVersion = bun --version 2>&1
    Write-Host "  [OK] Bun: $bunVersion" -ForegroundColor Green
} catch {
    Write-Host "  [!] Bun no encontrado - usando npm como fallback" -ForegroundColor DarkYellow
}

Write-Host ""

Write-Host "[5/5] Iniciando OmniMon como aplicacion de escritorio..." -ForegroundColor Yellow
Write-Host ""
Write-Host "IMPORTANTE: Esto abrira una VENTANA DE APLICACION (no navegador)" -ForegroundColor Magenta
Write-Host "           Con icono en system tray y permisos completos" -ForegroundColor Magenta
Write-Host ""
Write-Host "Presiona Ctrl+C para detener la aplicacion" -ForegroundColor Gray
Write-Host ""
Write-Host "----------------------------------------" -ForegroundColor DarkGray
Write-Host "Verificaciones que puedes hacer:" -ForegroundColor Cyan
Write-Host "  1. Ctrl+Alt+O - Toggle ventana" -ForegroundColor White
Write-Host "  2. Tooltip dinamico en tray icon" -ForegroundColor White
Write-Host "  3. Cerrar ventana X - App termina completamente" -ForegroundColor White
Write-Host "  4. Conexiones de red visibles (requiere Admin)" -ForegroundColor White
Write-Host "  5. Sin ventanas de PowerShell extras" -ForegroundColor White
Write-Host "----------------------------------------" -ForegroundColor DarkGray
Write-Host ""

# Ejecutar tauri dev - esto abre la APP DE ESCRITORIO completa
npx tauri dev

# Si llegamos aqui, la app se cerro
Write-Host ""
Write-Host "Aplicacion cerrada." -ForegroundColor Yellow
