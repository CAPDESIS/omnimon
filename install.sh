#!/usr/bin/env bash
# install.sh - Install macmon for the current user
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${HOME}/.local/libexec/macmon"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/macmon"
LOG_DIR="${HOME}/.local/log/macmon"
PLIST_DIR="${HOME}/Library/LaunchAgents"
PLIST_LABEL="com.macmon.daemon"

echo "macmon installer"
echo "================"
echo ""

# Security: refuse to install if source directory is not owned by current user
src_owner=$(stat -f%u "$SCRIPT_DIR" 2>/dev/null)
if [[ "$src_owner" != "$(id -u)" ]]; then
    echo "ERROR: Source directory not owned by current user. Aborting." >&2
    echo "  Source: $SCRIPT_DIR (owner uid: $src_owner, you: $(id -u))" >&2
    exit 1
fi

echo "Install directory: $INSTALL_DIR"
echo "Config directory:  $CONFIG_DIR"
echo "Log directory:     $LOG_DIR"
echo ""

# Stop existing daemon if running
if launchctl list "$PLIST_LABEL" &>/dev/null; then
    echo "Stopping existing daemon..."
    launchctl unload "${PLIST_DIR}/${PLIST_LABEL}.plist" 2>/dev/null || true
fi

# Create directories with restrictive permissions (MITRE TA0004 - Privilege Escalation)
# User-only for sensitive directories to prevent other users from planting malicious scripts
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$PLIST_DIR"
mkdir -p "$CONFIG_DIR" "$LOG_DIR"
mkdir -p "$CONFIG_DIR/profiles"
chmod 700 "$CONFIG_DIR" "$LOG_DIR"
chmod 700 "$CONFIG_DIR/profiles"
chmod 755 "$INSTALL_DIR" "$BIN_DIR"

# Copy project files
echo "Copying files..."
cp -R "$SCRIPT_DIR"/lib "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/src "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/scripts "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/config "$INSTALL_DIR/"

# Fix permissions: scripts executable by owner only, no group/other write
chmod 755 "$INSTALL_DIR"/lib/*.sh
chmod 755 "$INSTALL_DIR"/src/daemon/macmond.sh
chmod 755 "$INSTALL_DIR"/src/cli/macmon.sh
chmod 755 "$INSTALL_DIR"/scripts/*.sh
# Non-executable files: read-only for group/other
find "$INSTALL_DIR"/src/gui -name '*.swift' -exec chmod 644 {} +
find "$INSTALL_DIR"/config -type f -exec chmod 644 {} +

# Helper: compile universal binary (arm64 + x86_64, merged with lipo)
compile_universal() {
    local output="$1"
    shift
    # $@ = remaining swiftc flags and sources
    swiftc -O -target arm64-apple-macos13 "$@" -o "${output}-arm64"
    swiftc -O -target x86_64-apple-macos13 "$@" -o "${output}-x86_64"
    lipo -create -output "$output" "${output}-arm64" "${output}-x86_64"
    rm -f "${output}-arm64" "${output}-x86_64"
}

# Use pre-compiled binaries if present (web installer flow), otherwise compile
if [[ -x "$SCRIPT_DIR/ProcessPicker" && -x "$SCRIPT_DIR/DiskIOHelper" && -x "$SCRIPT_DIR/MacmonStatusBar" ]]; then
    echo "Using pre-compiled binaries..."
    cp "$SCRIPT_DIR/ProcessPicker" "$INSTALL_DIR/ProcessPicker"
    cp "$SCRIPT_DIR/DiskIOHelper" "$INSTALL_DIR/DiskIOHelper"
    cp "$SCRIPT_DIR/MacmonStatusBar" "$INSTALL_DIR/MacmonStatusBar"
    echo "Binaries copied successfully"
else
    # Compile Swift picker
    echo "Compiling ProcessPicker..."
    compile_universal "$INSTALL_DIR/ProcessPicker" \
        -framework Cocoa \
        "$INSTALL_DIR/src/gui/ProcessPickerModel.swift" \
        "$INSTALL_DIR/src/gui/Localization.swift" \
        "$INSTALL_DIR/src/gui/AIService.swift" \
        "$INSTALL_DIR/src/gui/ProcessPicker.swift"
    echo "ProcessPicker compiled successfully"

    # Compile DiskIOHelper
    echo "Compiling DiskIOHelper..."
    compile_universal "$INSTALL_DIR/DiskIOHelper" \
        "$INSTALL_DIR/src/gui/DiskIOHelper.swift"
    echo "DiskIOHelper compiled successfully"

    # Compile MacmonStatusBar
    echo "Compiling MacmonStatusBar..."
    compile_universal "$INSTALL_DIR/MacmonStatusBar" \
        -framework Cocoa \
        "$INSTALL_DIR/src/gui/Localization.swift" \
        "$INSTALL_DIR/src/gui/AIService.swift" \
        "$INSTALL_DIR/src/gui/PreferencesWindow.swift" \
        "$INSTALL_DIR/src/gui/MacmonStatusBar.swift"
    echo "MacmonStatusBar compiled successfully"
fi

# Harden binary permissions: executable by owner, readable by group (755)
chmod 755 "$INSTALL_DIR/ProcessPicker" "$INSTALL_DIR/DiskIOHelper" "$INSTALL_DIR/MacmonStatusBar"

# Create config if absent, with restrictive permissions
if [[ ! -f "$CONFIG_DIR/macmon.yaml" ]]; then
    cp "$INSTALL_DIR/config/macmon.default.yaml" "$CONFIG_DIR/macmon.yaml"
    chmod 600 "$CONFIG_DIR/macmon.yaml"
    echo "Created default config at $CONFIG_DIR/macmon.yaml"
fi

# Install default profiles if missing
for profile in "$INSTALL_DIR"/config/profiles/*.yaml; do
    [[ -f "$profile" ]] || continue
    base=$(basename "$profile")
    if [[ ! -f "$CONFIG_DIR/profiles/$base" ]]; then
        cp "$profile" "$CONFIG_DIR/profiles/$base"
        chmod 600 "$CONFIG_DIR/profiles/$base"
    fi
done

# Generate plist from template
echo "Installing LaunchAgent..."
sed \
    -e "s|@@INSTALL_DIR@@|${HOME}/.local|g" \
    -e "s|@@LOG_DIR@@|${LOG_DIR}|g" \
    -e "s|@@CONFIG_DIR@@|${CONFIG_DIR}|g" \
    "$SCRIPT_DIR/templates/com.macmon.daemon.plist.in" \
    > "${PLIST_DIR}/${PLIST_LABEL}.plist"
# Plist must be readable by launchd (644)
chmod 644 "${PLIST_DIR}/${PLIST_LABEL}.plist"

# Create CLI wrapper with restrictive permissions
echo "Creating macmon symlink..."
(umask 022; cat > "${BIN_DIR}/macmon" <<WRAPPER
#!/usr/bin/env bash
export MACMON_HOME="$INSTALL_DIR"
export MACMON_CONFIG="$CONFIG_DIR/macmon.yaml"
exec "\${MACMON_HOME}/src/cli/macmon.sh" "\$@"
WRAPPER
)
chmod 755 "${BIN_DIR}/macmon"

# Start daemon
echo "Starting daemon..."
launchctl load -w "${PLIST_DIR}/${PLIST_LABEL}.plist"

# Read installed version
INSTALLED_VERSION="unknown"
if [[ -f "$INSTALL_DIR/lib/macmon-core.sh" ]]; then
    INSTALLED_VERSION=$(grep -o 'MACMON_VERSION="[^"]*"' "$INSTALL_DIR/lib/macmon-core.sh" | cut -d'"' -f2)
fi

# --- Colored onboarding message ---
G='\033[0;32m'   # green
C='\033[0;36m'   # cyan
Y='\033[1;33m'   # yellow
B='\033[1m'      # bold
D='\033[2m'      # dim
R='\033[0m'      # reset

echo ""
printf '%b' "${G}${B}"
cat <<'BANNER'
               ___  ___  ___   ___ ___  ___  ___
  _ __  __ _ _|  _||   \/   | / _ \   \| __|/ __|
 | '  \/ _` / _|  | |) | |) | (_) | |) | _|\__ \
 |_|_|_\__,_\__|  |___/|___/ \___/|___/|___|___/
BANNER
printf '%b' "${R}"
echo ""
printf '%b' "  ${B}v${INSTALLED_VERSION}${R}${D} — installed successfully${R}\n"
echo ""
printf '%b' "  ${G}${B}Daemon is running${R} and will auto-start on login.\n"
printf '%b' "  ${D}Monitoring RAM, swap, orphan daemons, and idle processes.${R}\n"
echo ""
printf '%b' "  ${C}${B}Quick Start${R}\n"
printf '%b' "  ${Y}macmon${R}              Open the native process picker\n"
printf '%b' "  ${Y}macmon status${R}       System health summary in terminal\n"
printf '%b' "  ${Y}macmon config edit${R}  Customize thresholds and intervals\n"
echo ""
printf '%b' "  ${C}${B}Menu Bar${R}\n"
printf '%b' "  ${D}Run once to start the status bar icon:${R}\n"
printf '%b' "  ${Y}MACMON_HOME=${INSTALL_DIR} ${INSTALL_DIR}/MacmonStatusBar &${R}\n"
printf '%b' "  ${D}Add to Login Items for auto-start.${R}\n"
echo ""
printf '%b' "  ${D}Config:${R}  ${CONFIG_DIR}/macmon.yaml\n"
printf '%b' "  ${D}Logs:${R}    ${LOG_DIR}/macmond.log\n"
printf '%b' "  ${D}Docs:${R}    macmon help\n"
echo ""

# PATH reminder
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
    printf '%b' "  ${Y}${B}Note:${R} Add ${BIN_DIR} to your PATH:\n"
    printf '%b' "  ${D}echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc${R}\n"
    echo ""
fi
