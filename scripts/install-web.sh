#!/usr/bin/env bash
# install-web.sh - Smart Multiplatform Web Installer for OmniMon v4
# Usage: curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/install-web.sh | bash
set -euo pipefail

REPO="chochy2001/omnimon"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

# --- Colors ---
G='\033[0;32m'
C='\033[0;36m'
Y='\033[1;33m'
RED='\033[0;31m'
B='\033[1m'
D='\033[2m'
R='\033[0m'

info()  { printf '%b\n' "${C}${B}==> ${R}${B}$1${R}"; }
error() { printf '%b\n' "${RED}${B}Error:${R} $1" >&2; exit 1; }
warn()  { printf '%b\n' "${Y}${B}Warning:${R} $1"; }

info "OmniMon Smart Web Installer (Multiplatform)"
echo ""

OS="$(uname -s)"
ARCH="$(uname -m)"

info "Detected OS: $OS ($ARCH)"
info "Fetching latest v4 release info from GitHub API..."

release_json=$(curl -fsSL "$API_URL" 2>/dev/null) || error "Failed to fetch release info from GitHub API"

# Extract version tag
tag_name=$(printf '%s' "$release_json" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"//;s/"//')
if [[ -z "$tag_name" ]]; then
    error "Could not parse release tag from GitHub API"
fi

version="${tag_name#v}"
printf '%b\n' "  ${D}Latest version: ${R}${B}${version}${R}"
echo ""

# Fallback migration check
if [[ -d "$HOME/.local/libexec/macmon" ]] && [[ "$OS" == "Darwin" ]]; then
    warn "Legacy macmon v3 detected. The new v4 is a standalone App/DMG."
    warn "You may want to run '~/.local/libexec/macmon/uninstall.sh' to clean up v3 daemons later."
    echo ""
fi

get_asset_url() {
    local ext="$1"
    printf '%s' "$release_json" | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*'"${ext}"'"' | head -1 | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//;s/"//'
}

TMPDIR_INSTALL=$(mktemp -d "${TMPDIR:-/tmp}/omnimon-install.XXXXXXXXXX")
trap 'rm -rf "$TMPDIR_INSTALL"' EXIT

if [[ "$OS" == "Darwin" ]]; then
    info "macOS Environment. Looking for .dmg artifact..."
    asset_url=$(get_asset_url "\.dmg")
    
    if [[ -z "$asset_url" ]]; then
        error "Could not find .dmg in latest release. Visit: https://github.com/$REPO/releases"
    fi
    
    dmg_path="${TMPDIR_INSTALL}/OmniMon.dmg"
    info "Downloading $asset_url ..."
    curl -fSL -o "$dmg_path" "$asset_url" || error "Failed to download DMG"
    
    info "Mounting DMG. Please drag OmniMon to your Applications folder."
    hdiutil attach "$dmg_path"

elif [[ "$OS" == "Linux" ]]; then
    info "Linux Environment. Looking for .deb artifact..."
    asset_url=$(get_asset_url "\.deb")
    
    if [[ -z "$asset_url" ]]; then
        error "Could not find .deb in latest release. Visit: https://github.com/$REPO/releases"
    fi
    
    deb_path="${TMPDIR_INSTALL}/omnimon.deb"
    info "Downloading $asset_url ..."
    curl -fSL -o "$deb_path" "$asset_url" || error "Failed to download DEB"
    
    info "Installing deb package (requires sudo)..."
    sudo dpkg -i "$deb_path" || sudo apt-get install -f -y
    info "OmniMon installed successfully."

elif [[ "$OS" == *"MINGW"* ]] || [[ "$OS" == *"CYGWIN"* ]] || [[ "$OS" == *"MSYS"* ]]; then
    info "Windows Environment. Looking for .exe artifact..."
    asset_url=$(get_asset_url "\.exe")
    
    if [[ -z "$asset_url" ]]; then
        error "Could not find .exe in latest release. Visit: https://github.com/$REPO/releases"
    fi
    
    exe_path="${TMPDIR_INSTALL}/omnimon-setup.exe"
    info "Downloading $asset_url ..."
    curl -fSL -o "$exe_path" "$asset_url" || error "Failed to download EXE"
    
    info "Launching installer..."
    start "$exe_path"

else
    error "Operating System '$OS' is not automatically supported by this script. Download binaries from: https://github.com/$REPO/releases"
fi

echo ""
info "Transition to OmniMon v4 Complete!"
