# Script para ejecutar OmniMon en modo desarrollo
# Asegura que Rust/Cargo estén en el PATH

Write-Host "==================================" -ForegroundColor Cyan
Write-Host "  OmniMon - Modo Desarrollo" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan
Write-Host ""

# Cambiar al directorio de la app
$appDir = "C:\Users\ohcho\Documents\Apps\omnimon\v4\apps\desktop"
Set-Location $appDir

Write-Host "[1/3] Verificando dependencias..." -ForegroundColor Yellow

# Verificar que cargo esté disponible
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Cargo no está en el PATH" -ForegroundColor Red
    Write-Host "Por favor ejecuta este script desde 'Developer PowerShell for VS'" -ForegroundColor Red
    Write-Host "O agrega Cargo al PATH:" -ForegroundColor Yellow
    Write-Host '  $env:PATH += ";$env:USERPROFILE\.cargo\bin"' -ForegroundColor Gray
    pause
    exit 1
}

# Verificar que node esté disponible
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Node.js no está instalado o no está en el PATH" -ForegroundColor Red
    pause
    exit 1
}

Write-Host "  [OK] Cargo: $(cargo --version)" -ForegroundColor Green
Write-Host "  [OK] Node: $(node --version)" -ForegroundColor Green
Write-Host ""

# Instalar dependencias si no existen
if (-not (Test-Path "node_modules")) {
    Write-Host "[2/3] Instalando dependencias de npm..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Fallo la instalación de dependencias" -ForegroundColor Red
        pause
        exit 1
    }
} else {
    Write-Host "[2/3] Dependencias ya instaladas" -ForegroundColor Green
}

Write-Host ""
Write-Host "[3/3] Iniciando OmniMon..." -ForegroundColor Yellow
Write-Host ""
Write-Host "Presiona Ctrl+C para detener la aplicación" -ForegroundColor Gray
Write-Host ""
Write-Host "----------------------------------------" -ForegroundColor DarkGray
Write-Host "Verificaciones que puedes hacer:" -ForegroundColor Cyan
Write-Host "  1. Ctrl+Alt+O - Toggle ventana" -ForegroundColor White
Write-Host "  2. Tooltip dinámico en tray icon" -ForegroundColor White
Write-Host "  3. Cerrar ventana X - App termina completamente" -ForegroundColor White
Write-Host "  4. Conexiones de red visibles (requiere Admin)" -ForegroundColor White
Write-Host "----------------------------------------" -ForegroundColor DarkGray
Write-Host ""

# Ejecutar tauri dev
npm run tauri dev
