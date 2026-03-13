# Script para instalar Rust en Windows
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Instalador de Rust para OmniMon" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Descargando rustup (instalador de Rust)..." -ForegroundColor Yellow
Write-Host ""

# Descargar rustup-init.exe
$rustupUrl = "https://win.rustup.rs/x86_64"
$rustupPath = "$env:TEMP\rustup-init.exe"

try {
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath -UseBasicParsing
    Write-Host "Descarga completada." -ForegroundColor Green
    Write-Host ""

    Write-Host "Ejecutando instalador de Rust..." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "IMPORTANTE: Durante la instalacion:" -ForegroundColor Magenta
    Write-Host "  1. Presiona ENTER para aceptar la instalacion por defecto" -ForegroundColor White
    Write-Host "  2. Espera a que termine (puede tomar varios minutos)" -ForegroundColor White
    Write-Host "  3. NO cierres esta ventana hasta que diga 'Rust is installed now'" -ForegroundColor White
    Write-Host ""

    # Ejecutar rustup-init
    Start-Process -FilePath $rustupPath -ArgumentList "-y" -Wait -NoNewWindow

    Write-Host ""
    Write-Host "Rust instalado correctamente!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Agregando Rust al PATH..." -ForegroundColor Yellow

    # Agregar Rust al PATH de la sesion actual
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    $env:PATH = "$cargoPath;$env:PATH"

    Write-Host "PATH actualizado." -ForegroundColor Green
    Write-Host ""
    Write-Host "Verificando instalacion..." -ForegroundColor Yellow

    # Verificar cargo
    & "$cargoPath\cargo.exe" --version

    Write-Host ""
    Write-Host "============================================" -ForegroundColor Green
    Write-Host "  RUST INSTALADO EXITOSAMENTE" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "SIGUIENTE PASO:" -ForegroundColor Cyan
    Write-Host "  Cierra esta ventana y ejecuta:" -ForegroundColor White
    Write-Host "  .\EJECUTAR_OMNIMON.bat" -ForegroundColor Yellow
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "ERROR: No se pudo descargar o instalar Rust" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    Write-Host "INSTALACION MANUAL:" -ForegroundColor Yellow
    Write-Host "  1. Visita: https://rustup.rs/" -ForegroundColor White
    Write-Host "  2. Descarga rustup-init.exe" -ForegroundColor White
    Write-Host "  3. Ejecutalo y sigue las instrucciones" -ForegroundColor White
    Write-Host ""
}

pause
