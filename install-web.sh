#!/usr/bin/env bash
# install-web.sh - One-command web installer for macmon
# Usage: curl -fsSL https://raw.githubusercontent.com/chochy2001/macmon/main/install-web.sh | bash
set -euo pipefail

REPO="chochy2001/macmon"
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

# --- Pre-flight checks ---

# macOS only
if [[ "$(uname -s)" != "Darwin" ]]; then
    error "macmon is only supported on macOS"
fi

# Required tools (both ship with macOS)
command -v curl >/dev/null 2>&1 || error "curl is required but not found"
command -v tar  >/dev/null 2>&1 || error "tar is required but not found"

info "macmon web installer"
echo ""

# --- Fetch latest release tag ---
info "Fetching latest release..."
release_json=$(curl -fsSL "$API_URL" 2>/dev/null) || error "Failed to fetch release info from GitHub API"

# Parse tag_name (e.g. "v1.2.0") — no jq needed
tag_name=$(printf '%s' "$release_json" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"//;s/"//')
if [[ -z "$tag_name" ]]; then
    error "Could not parse release tag from GitHub API"
fi

version="${tag_name#v}"
printf '%b\n' "  ${D}Latest version: ${R}${B}${version}${R}"
echo ""

# --- Download release tarball ---
archive_name="macmon-${version}-macos-universal.tar.gz"

# Parse browser_download_url for the universal tarball
asset_url=$(printf '%s' "$release_json" | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*'"${archive_name}"'"' | head -1 | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//;s/"//')
if [[ -z "$asset_url" ]]; then
    error "Could not find ${archive_name} in release assets. Is the release properly built?"
fi

TMPDIR_INSTALL=$(mktemp -d "${TMPDIR:-/tmp}/macmon-install.XXXXXXXXXX")
trap 'rm -rf "$TMPDIR_INSTALL"' EXIT

info "Downloading ${archive_name}..."
curl -fSL -o "${TMPDIR_INSTALL}/${archive_name}" "$asset_url" || error "Failed to download release archive"

# --- Checksum verification ---
checksum_name="checksums-sha256.txt"
checksum_url=$(printf '%s' "$release_json" | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*'"${checksum_name}"'"' | head -1 | sed 's/.*"browser_download_url"[[:space:]]*:[[:space:]]*"//;s/"//')
if [[ -z "$checksum_url" ]]; then
    error "Could not find ${checksum_name} in release assets. Cannot verify integrity."
fi

info "Verifying checksum..."
curl -fsSL -o "${TMPDIR_INSTALL}/${checksum_name}" "$checksum_url" || error "Failed to download checksum file"

# Verify archive integrity (filter to the specific file we downloaded)
(cd "$TMPDIR_INSTALL" && grep "$archive_name" "$checksum_name" | shasum -a 256 -c -) || error "Checksum verification failed. Archive may be corrupted or tampered with."

# --- Extract and install ---
info "Extracting..."
tar xzf "${TMPDIR_INSTALL}/${archive_name}" -C "$TMPDIR_INSTALL" || error "Failed to extract archive"

# The archive extracts to a macmon/ directory
install_src="${TMPDIR_INSTALL}/macmon"
if [[ ! -d "$install_src" ]]; then
    # Fallback: check if files are at the root of the temp dir
    install_src="$TMPDIR_INSTALL"
fi

if [[ ! -f "${install_src}/install.sh" ]]; then
    error "install.sh not found in release archive"
fi

info "Installing macmon v${version}..."
echo ""
cd "$install_src"
bash install.sh

echo ""
info "Installation complete!"
echo ""
