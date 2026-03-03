#!/usr/bin/env bash
# uninstall.sh - Remove macmon from the current user's system
set -euo pipefail

INSTALL_DIR="${HOME}/.local/libexec/macmon"
BIN_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/macmon"
LOG_DIR="${HOME}/.local/log/macmon"
PLIST_DIR="${HOME}/Library/LaunchAgents"
PLIST_LABEL="com.macmon.daemon"

echo "macmon uninstaller"
echo "=================="
echo ""

# Stop daemon
if launchctl list "$PLIST_LABEL" &>/dev/null; then
    echo "Stopping daemon..."
    launchctl unload "${PLIST_DIR}/${PLIST_LABEL}.plist" 2>/dev/null || true
fi

# Remove plist
rm -f "${PLIST_DIR}/${PLIST_LABEL}.plist"
echo "Removed LaunchAgent"

# Remove CLI symlink
rm -f "${BIN_DIR}/macmon"
echo "Removed macmon symlink"

# Remove install directory
rm -rf "$INSTALL_DIR"
echo "Removed $INSTALL_DIR"

# Ask about config
if [[ -d "$CONFIG_DIR" ]]; then
    read -rp "Remove configuration ($CONFIG_DIR)? [y/N] " answer
    if [[ "$answer" =~ ^[Yy] ]]; then
        rm -rf "$CONFIG_DIR"
        echo "Removed configuration"
    else
        echo "Kept configuration at $CONFIG_DIR"
    fi
fi

# Ask about logs
if [[ -d "$LOG_DIR" ]]; then
    read -rp "Remove logs ($LOG_DIR)? [y/N] " answer
    if [[ "$answer" =~ ^[Yy] ]]; then
        rm -rf "$LOG_DIR"
        echo "Removed logs"
    else
        echo "Kept logs at $LOG_DIR"
    fi
fi

echo ""
echo "macmon has been uninstalled."
