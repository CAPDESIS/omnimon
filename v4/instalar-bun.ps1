# Script para instalar Bun en Windows
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Instalador de Bun para OmniMon" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Instalando Bun..." -ForegroundColor Yellow
Write-Host ""

try {
    # Instalar Bun usando el instalador oficial de Windows
    powershell -c "irm bun.sh/install.ps1|iex"

    Write-Host ""
    Write-Host "============================================" -ForegroundColor Green
    Write-Host "  BUN INSTALADO EXITOSAMENTE" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "NOTA: Bun es el package manager preferido del proyecto" -ForegroundColor Cyan
    Write-Host "      pero npm tambien funciona." -ForegroundColor Cyan
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "ERROR: No se pudo instalar Bun" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    Write-Host "INSTALACION MANUAL:" -ForegroundColor Yellow
    Write-Host "  1. Visita: https://bun.sh/" -ForegroundColor White
    Write-Host "  2. Sigue las instrucciones de instalacion para Windows" -ForegroundColor White
    Write-Host ""
    Write-Host "O puedes usar npm (ya instalado) en vez de bun." -ForegroundColor Cyan
    Write-Host ""
}

pause
