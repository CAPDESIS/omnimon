#!/usr/bin/env bash
set -e

echo "🔍 Verificando dependencias de desarrollo para macmon v4..."

# 1. Verificar Rust / Cargo
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: 'cargo' no está instalado. Instala Rust (https://rustup.rs/)."
    exit 1
fi
echo "✅ cargo detectado: $(cargo --version)"

# 2. Verificar Node.js / NPM
if ! command -v npm &> /dev/null; then
    echo "❌ Error: 'npm' no está instalado. Instala Node.js (https://nodejs.org/)."
    exit 1
fi
echo "✅ npm detectado: $(npm --version)"

# 3. Dependencias específicas de Linux (Debian/Ubuntu)
if [ "$(uname)" == "Linux" ]; then
    echo "🐧 Verificando dependencias específicas de Linux (Debian/Ubuntu)..."
    if ! dpkg -s libwebkit2gtk-4.1-dev &> /dev/null; then
        echo "⚠️ Advertencia: libwebkit2gtk-4.1-dev parece no estar instalado."
        echo "💡 Ejecuta: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf"
    else
        echo "✅ Dependencias de Linux detectadas."
    fi
fi

# 4. Dependencias específicas de macOS
if [ "$(uname)" == "Darwin" ]; then
    echo "🍎 Entorno macOS detectado. Asegúrate de tener las Xcode Command Line Tools instaladas."
    if ! xcode-select -p &> /dev/null; then
        echo "⚠️ Advertencia: Xcode Command Line Tools no detectadas."
        echo "💡 Ejecuta: xcode-select --install"
    else
        echo "✅ Xcode Command Line Tools detectadas."
    fi
fi

echo ""
echo "📦 Instalando dependencias de NPM (si aplica)..."
if [ -d "apps/desktop" ]; then
    cd apps/desktop
    if [ -f "package.json" ]; then
        npm install
    else
         echo "⚠️ No se encontró package.json en apps/desktop, omitiendo npm install."
    fi
    cd ../..
fi

echo "🚀 ¡Entorno configurado correctamente! Usa 'make dev' para iniciar."
