#!/usr/bin/env bash
# install-web.sh - Smart Multiplatform Web Installer for OmniMon v5
# Usage: curl -fsSL https://raw.githubusercontent.com/chochy2001/omnimon/main/scripts/install-web.sh | bash
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
info "Fetching latest v5 release info from GitHub API..."

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
    warn "Legacy OmniMon v3 detected. The new v5 is a standalone App/DMG."
    warn "You may want to run '~/.local/libexec/macmon/uninstall.sh' to clean up v3 daemons later."
    echo ""
fi

get_asset_url() {
    local ext="$1"
    printf '%s' "$release_json" | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*'"${ext}"'"' | head -1 | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//;s/"//'
}

# Downloads SHA256SUMS.txt from the release and verifies the given artifact
# against it. Fails closed: any missing piece (checksum file, entry, or
# checksum tool) aborts the install instead of running unverified binaries.
verify_artifact() {
    local file_path="$1"
    local asset_name="${file_path##*/}"
    local sums_url sums_file line count

    sums_url=$(get_asset_url "SHA256SUMS\.txt")
    if [[ -z "$sums_url" ]]; then
        error "SHA256SUMS.txt not found in latest release. Refusing to install an unverified artifact."
    fi

    sums_file="${TMPDIR_INSTALL}/SHA256SUMS.txt"
    info "Downloading SHA256SUMS.txt to verify ${asset_name}..."
    curl -fsSL -o "$sums_file" "$sums_url" || error "Failed to download SHA256SUMS.txt. Refusing to install an unverified artifact."

    line=$(awk -v n="$asset_name" '{f=$NF; sub(/^\*/, "", f); if (f == n) print}' "$sums_file")
    count=$(printf '%s\n' "$line" | grep -c .)
    if [[ "$count" -ne 1 ]]; then
        error "Expected exactly one SHA256 entry for ${asset_name} in SHA256SUMS.txt (found ${count}). Refusing to install."
    fi

    info "Verifying SHA256 checksum of ${asset_name}..."
    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$TMPDIR_INSTALL" && printf '%s\n' "$line" | sha256sum -c -) \
            || error "SHA256 verification failed for ${asset_name}. Aborting install."
    elif command -v shasum >/dev/null 2>&1; then
        (cd "$TMPDIR_INSTALL" && printf '%s\n' "$line" | shasum -a 256 -c -) \
            || error "SHA256 verification failed for ${asset_name}. Aborting install."
    else
        error "Neither sha256sum nor shasum is available. Refusing to install an unverified artifact."
    fi
    info "Checksum OK: ${asset_name}"
}

TMPDIR_INSTALL=$(mktemp -d "${TMPDIR:-/tmp}/omnimon-install.XXXXXXXXXX")
trap 'rm -rf "$TMPDIR_INSTALL"' EXIT

if [[ "$OS" == "Darwin" ]]; then
    info "macOS Environment. Looking for .dmg artifact..."
    asset_url=$(get_asset_url "\.dmg")
    
    if [[ -z "$asset_url" ]]; then
        error "Could not find .dmg in latest release. Visit: https://github.com/$REPO/releases"
    fi
    
    dmg_path="${TMPDIR_INSTALL}/${asset_url##*/}"
    info "Downloading $asset_url ..."
    curl -fSL -o "$dmg_path" "$asset_url" || error "Failed to download DMG"
    verify_artifact "$dmg_path"
    
    info "Mounting DMG. Please drag OmniMon to your Applications folder."
    hdiutil attach "$dmg_path"

elif [[ "$OS" == "Linux" ]]; then
    info "Linux Environment. Looking for .deb or .rpm artifact..."
    asset_url=$(get_asset_url "\.deb")
    
    if [[ -z "$asset_url" ]]; then
        asset_url=$(get_asset_url "\.rpm")
        if [[ -z "$asset_url" ]]; then
            error "Could not find .deb or .rpm in latest release. Visit: https://github.com/$REPO/releases"
        fi
    fi

    if [[ "$asset_url" == *.rpm ]]; then
        rpm_path="${TMPDIR_INSTALL}/${asset_url##*/}"
        info "Downloading $asset_url ..."
        curl -fSL -o "$rpm_path" "$asset_url" || error "Failed to download RPM"
        verify_artifact "$rpm_path"
        info "Installing rpm package (requires sudo)..."
        sudo rpm -Uvh "$rpm_path"
        info "OmniMon installed successfully."
        exit 0
    fi

    deb_path="${TMPDIR_INSTALL}/${asset_url##*/}"
    info "Downloading $asset_url ..."
    curl -fSL -o "$deb_path" "$asset_url" || error "Failed to download DEB"
    verify_artifact "$deb_path"
    
    info "Installing deb package (requires sudo)..."
    sudo dpkg -i "$deb_path" || sudo apt-get install -f -y
    info "OmniMon installed successfully."

elif [[ "$OS" == *"MINGW"* ]] || [[ "$OS" == *"CYGWIN"* ]] || [[ "$OS" == *"MSYS"* ]]; then
    info "Windows Environment. Looking for .msi artifact..."
    asset_url=$(get_asset_url "\.msi")
    
    if [[ -z "$asset_url" ]]; then
        error "Could not find .msi in latest release. Visit: https://github.com/$REPO/releases"
    fi
    
    exe_path="${TMPDIR_INSTALL}/${asset_url##*/}"
    info "Downloading $asset_url ..."
    curl -fSL -o "$exe_path" "$asset_url" || error "Failed to download EXE"
    verify_artifact "$exe_path"
    
    info "Launching installer..."
    start msiexec /i "$exe_path"

else
    error "Operating System '$OS' is not automatically supported by this script. Download binaries from: https://github.com/$REPO/releases"
fi

echo ""
info "Transition to OmniMon v5 Complete!"
