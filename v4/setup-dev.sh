#!/usr/bin/env bash
set -e

echo "Checking development dependencies for OmniMon v4..."

# 1. Check Rust / Cargo
if ! command -v cargo &> /dev/null; then
    echo "Error: 'cargo' is not installed. Install Rust: https://rustup.rs/"
    exit 1
fi
echo "cargo detected: $(cargo --version)"

# 2. Check Node.js / NPM
if ! command -v npm &> /dev/null; then
    echo "Error: 'npm' is not installed. Install Node.js: https://nodejs.org/"
    exit 1
fi
echo "npm detected: $(npm --version)"

# 3. Linux-specific dependencies (Debian/Ubuntu)
if [ "$(uname)" == "Linux" ]; then
    echo "Checking Linux-specific dependencies (Debian/Ubuntu)..."
    if ! dpkg -s libwebkit2gtk-4.1-dev &> /dev/null; then
        echo "Warning: libwebkit2gtk-4.1-dev does not appear to be installed."
        echo "Run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf"
    else
        echo "Linux dependencies detected."
    fi
fi

# 4. macOS-specific dependencies
if [ "$(uname)" == "Darwin" ]; then
    echo "macOS environment detected. Ensure Xcode Command Line Tools are installed."
    if ! xcode-select -p &> /dev/null; then
        echo "Warning: Xcode Command Line Tools not detected."
        echo "Run: xcode-select --install"
    else
        echo "Xcode Command Line Tools detected."
    fi
fi

echo ""
echo "Installing NPM dependencies..."
if [ -d "apps/desktop" ]; then
    cd apps/desktop
    if [ -f "package.json" ]; then
        npm install
    else
        echo "Warning: package.json not found in apps/desktop, skipping npm install."
    fi
    cd ../..
fi

echo "Environment configured successfully! Run 'make dev' to start."
