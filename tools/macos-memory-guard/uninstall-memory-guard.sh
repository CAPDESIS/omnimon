#!/usr/bin/env bash
# Unload LaunchAgent and remove installed memory-guard files (keeps logs/config by default).
set -euo pipefail

HOME_DIR="${HOME:?}"
LIB_DIR="${MEMORY_GUARD_LIB_DIR:-$HOME_DIR/.local/lib/omnimon-memory-guard}"
BIN_DIR="${MEMORY_GUARD_BIN_DIR:-$HOME_DIR/.local/bin}"
LAUNCH_DIR="$HOME_DIR/Library/LaunchAgents"
LABEL="com.omnimon.memory-guard"
PLIST="$LAUNCH_DIR/${LABEL}.plist"
UID_NUM="$(id -u)"
PURGE_STATE=0

for a in "$@"; do
  case "$a" in
    --purge-state) PURGE_STATE=1 ;;
  esac
done

if launchctl print "gui/${UID_NUM}/${LABEL}" >/dev/null 2>&1; then
  launchctl bootout "gui/${UID_NUM}/${LABEL}" 2>/dev/null || true
fi
rm -f "$PLIST"
rm -f "$BIN_DIR/omnimon-memory-guard"
rm -rf "$LIB_DIR"

if [[ $PURGE_STATE -eq 1 ]]; then
  rm -rf "${XDG_STATE_HOME:-$HOME_DIR/.local/state}/omnimon-memory-guard"
  rm -f "${XDG_CONFIG_HOME:-$HOME_DIR/.config}/omnimon/memory-guard.conf"
fi

echo "uninstalled $LABEL"
