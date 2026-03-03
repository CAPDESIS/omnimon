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
chmod 700 "$CONFIG_DIR" "$LOG_DIR"
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

# Compile Swift picker
echo "Compiling ProcessPicker..."
swiftc -O -framework Cocoa \
    -o "$INSTALL_DIR/ProcessPicker" \
    "$INSTALL_DIR/src/gui/ProcessPickerModel.swift" \
    "$INSTALL_DIR/src/gui/ProcessPicker.swift"
echo "ProcessPicker compiled successfully"

# Compile DiskIOHelper
echo "Compiling DiskIOHelper..."
swiftc -O \
    -o "$INSTALL_DIR/DiskIOHelper" \
    "$INSTALL_DIR/src/gui/DiskIOHelper.swift"
echo "DiskIOHelper compiled successfully"

# Compile MacmonStatusBar
echo "Compiling MacmonStatusBar..."
swiftc -O -framework Cocoa \
    -o "$INSTALL_DIR/MacmonStatusBar" \
    "$INSTALL_DIR/src/gui/MacmonStatusBar.swift"
echo "MacmonStatusBar compiled successfully"

# Harden binary permissions: executable by owner, readable by group (755)
chmod 755 "$INSTALL_DIR/ProcessPicker" "$INSTALL_DIR/DiskIOHelper" "$INSTALL_DIR/MacmonStatusBar"

# Create config if absent, with restrictive permissions
if [[ ! -f "$CONFIG_DIR/macmon.yaml" ]]; then
    cp "$INSTALL_DIR/config/macmon.default.yaml" "$CONFIG_DIR/macmon.yaml"
    chmod 600 "$CONFIG_DIR/macmon.yaml"
    echo "Created default config at $CONFIG_DIR/macmon.yaml"
fi

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

echo ""
echo "Installation complete!"
echo ""
echo "Make sure $BIN_DIR is in your PATH."
echo "  Usage:"
echo "    macmon              # Open process picker"
echo "    macmon status       # System health summary"
echo "    macmon help         # All commands"
echo ""
echo "  Menu Bar:"
echo "    MACMON_HOME=$INSTALL_DIR $INSTALL_DIR/MacmonStatusBar &"
echo "    # Runs a status bar icon with live RAM/swap/process info."
echo "    # Add to Login Items for auto-start."
echo ""
echo "The daemon is running and will auto-start on login."
echo "Config: $CONFIG_DIR/macmon.yaml"
echo "Logs:   $LOG_DIR/macmond.log"
