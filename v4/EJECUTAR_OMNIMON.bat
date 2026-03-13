@echo off
REM ====================================================================
REM   EJECUTAR OMNIMON - Aplicación de Escritorio (NO navegador)
REM ====================================================================
REM
REM Este script ejecuta OmniMon como aplicación nativa de Windows
REM con icono en system tray, permisos completos y todas las funciones.
REM
REM IMPORTANTE: Si cargo no está en el PATH, el script te dará
REM instrucciones exactas de cómo ejecutarlo manualmente.
REM ====================================================================

echo.
echo ============================================
echo   OmniMon - Aplicacion de Escritorio
echo ============================================
echo.
echo IMPORTANTE: Esto abre una APP NATIVA (no navegador web)
echo            con icono en system tray y permisos completos
echo.
echo ============================================
echo.

REM Cambiar al directorio del script
cd /d "%~dp0"

REM Ejecutar el script de PowerShell mejorado
powershell.exe -ExecutionPolicy Bypass -File "%~dp0run-dev-auto.ps1"

if errorlevel 1 (
    echo.
    echo ERROR: La aplicacion no pudo iniciarse
    echo.
    echo Lee las instrucciones arriba para ejecutarla manualmente.
    echo.
    pause
    exit /b 1
)

echo.
echo Aplicacion cerrada correctamente.
echo.
pause
