@echo off
REM Script batch para ejecutar OmniMon en modo desarrollo
REM Este script lanza PowerShell con el script run-dev.ps1

echo ============================================
echo   OmniMon - Iniciando modo desarrollo
echo ============================================
echo.

REM Cambiar al directorio del script
cd /d "%~dp0"

REM Ejecutar el script de PowerShell
powershell.exe -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1"

if errorlevel 1 (
    echo.
    echo ERROR: La aplicacion no pudo iniciarse
    pause
    exit /b 1
)
