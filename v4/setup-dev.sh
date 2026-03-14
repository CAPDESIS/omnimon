#!/usr/bin/env bash
set -e

echo "==================================="
echo "  OmniMon v4 - Setup de Desarrollo"
echo "==================================="
echo ""

ERRORS=0

# 1. Check Rust / Cargo
echo -n "[1/4] Rust/Cargo: "
if ! command -v cargo &> /dev/null; then
    echo "NO INSTALADO"
    echo "  -> Instala Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    ERRORS=$((ERRORS + 1))
else
    echo "$(cargo --version)"
fi

# 2. Check Node.js
echo -n "[2/4] Node.js: "
if ! command -v node &> /dev/null; then
    echo "NO INSTALADO"
    echo "  -> Instala Node.js: https://nodejs.org/"
    ERRORS=$((ERRORS + 1))
else
    echo "$(node --version)"
fi

# 3. Check Bun
echo -n "[3/4] Bun: "
if command -v bun &> /dev/null; then
    echo "$(bun --version)"
    PKG_MGR="bun"
else
    echo "No instalado (usando npm como alternativa)"
    if command -v npm &> /dev/null; then
        PKG_MGR="npm"
    else
        echo "  -> Error: ni bun ni npm están disponibles"
        ERRORS=$((ERRORS + 1))
    fi
fi

# 4. Platform-specific dependencies
echo -n "[4/4] Dependencias de plataforma: "
if [ "$(uname)" == "Linux" ]; then
    if dpkg -s libwebkit2gtk-4.1-dev &> /dev/null 2>&1; then
        echo "Linux deps OK"
    else
        echo "FALTAN"
        echo "  -> Ejecuta: sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libssl-dev libgtk-3-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev"
        ERRORS=$((ERRORS + 1))
    fi
elif [ "$(uname)" == "Darwin" ]; then
    if xcode-select -p &> /dev/null; then
        echo "macOS (Xcode CLI Tools OK)"
    else
        echo "FALTAN"
        echo "  -> Ejecuta: xcode-select --install"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo "Plataforma: $(uname)"
fi

echo ""

if [ $ERRORS -gt 0 ]; then
    echo "Se encontraron $ERRORS problemas. Instala las dependencias faltantes y vuelve a ejecutar."
    exit 1
fi

# Install dependencies
echo "Instalando dependencias del frontend..."
if [ -d "apps/desktop" ]; then
    cd apps/desktop
    if [ -f "package.json" ]; then
        $PKG_MGR install
    else
        echo "Advertencia: package.json no encontrado en apps/desktop"
    fi
    cd ../..
fi

echo ""
echo "==================================="
echo "  Setup completado exitosamente!"
echo "  Ejecuta 'make dev' para iniciar."
echo "==================================="
