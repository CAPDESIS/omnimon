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
echo "Install directory: $INSTALL_DIR"
echo "Config directory:  $CONFIG_DIR"
echo "Log directory:     $LOG_DIR"
echo ""

# Stop existing daemon if running
if launchctl list "$PLIST_LABEL" &>/dev/null; then
    echo "Stopping existing daemon..."
    launchctl unload "${PLIST_DIR}/${PLIST_LABEL}.plist" 2>/dev/null || true
fi

# Create directories
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$CONFIG_DIR" "$LOG_DIR" "$PLIST_DIR"

# Copy project files
echo "Copying files..."
cp -R "$SCRIPT_DIR"/lib "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/src "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/scripts "$INSTALL_DIR/"
cp -R "$SCRIPT_DIR"/config "$INSTALL_DIR/"

# Make scripts executable
chmod +x "$INSTALL_DIR"/lib/*.sh
chmod +x "$INSTALL_DIR"/src/daemon/macmond.sh
chmod +x "$INSTALL_DIR"/src/cli/macmon.sh
chmod +x "$INSTALL_DIR"/scripts/*.sh

# Compile Swift picker
echo "Compiling ProcessPicker..."
swiftc -O -framework Cocoa \
    -o "$INSTALL_DIR/ProcessPicker" \
    "$INSTALL_DIR/src/gui/ProcessPicker.swift"
echo "ProcessPicker compiled successfully"

# Compile DiskIOHelper
echo "Compiling DiskIOHelper..."
swiftc -O \
    -o "$INSTALL_DIR/DiskIOHelper" \
    "$INSTALL_DIR/src/gui/DiskIOHelper.swift"
echo "DiskIOHelper compiled successfully"

# Create config if absent
if [[ ! -f "$CONFIG_DIR/macmon.yaml" ]]; then
    cp "$INSTALL_DIR/config/macmon.default.yaml" "$CONFIG_DIR/macmon.yaml"
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

# Create CLI symlink
echo "Creating macmon symlink..."
cat > "${BIN_DIR}/macmon" <<WRAPPER
#!/usr/bin/env bash
export MACMON_HOME="$INSTALL_DIR"
export MACMON_CONFIG="$CONFIG_DIR/macmon.yaml"
exec "\${MACMON_HOME}/src/cli/macmon.sh" "\$@"
WRAPPER
chmod +x "${BIN_DIR}/macmon"

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
echo "The daemon is running and will auto-start on login."
echo "Config: $CONFIG_DIR/macmon.yaml"
echo "Logs:   $LOG_DIR/macmond.log"
