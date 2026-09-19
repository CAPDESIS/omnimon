#!/usr/bin/env bash
# Install memory-guard outside ~/Documents (macOS 26 TCC-safe) and load LaunchAgent.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
HOME_DIR="${HOME:?}"
LIB_DIR="${MEMORY_GUARD_LIB_DIR:-$HOME_DIR/.local/lib/omnimon-memory-guard}"
BIN_DIR="${MEMORY_GUARD_BIN_DIR:-$HOME_DIR/.local/bin}"
STATE_DIR="${MEMORY_GUARD_STATE_DIR:-${XDG_STATE_HOME:-$HOME_DIR/.local/state}/omnimon-memory-guard}"
CFG_DIR="${XDG_CONFIG_HOME:-$HOME_DIR/.config}/omnimon"
LAUNCH_DIR="$HOME_DIR/Library/LaunchAgents"
LABEL="com.omnimon.memory-guard"
PLIST="$LAUNCH_DIR/${LABEL}.plist"
UID_NUM="$(id -u)"

mkdir -p "$LIB_DIR" "$BIN_DIR" "$STATE_DIR" "$CFG_DIR" "$LAUNCH_DIR"

install -m 0755 "$ROOT/memory-guard.sh" "$LIB_DIR/memory-guard.sh"
ln -sfn "$LIB_DIR/memory-guard.sh" "$BIN_DIR/omnimon-memory-guard"

if [[ ! -f "$CFG_DIR/memory-guard.conf" ]]; then
  cat >"$CFG_DIR/memory-guard.conf" <<'EOF'
# OmniMon memory-guard config (sourced by bash)
# MG_INTERVAL_OK=15
# MG_INTERVAL_STRESS=5
# MG_FREE_WARN=25
# MG_FREE_CRIT=15
# MG_FREE_EMERGENCY=8
# MG_SWAP_WARN_MB=1024
# MG_SWAP_CRIT_MB=4096
# MG_SWAP_EMERGENCY_MB=8192
# MG_PROC_KILL_MB=6144
# MG_CHROME_TOTAL_CRIT_MB=12288
# MG_CHROME_TOTAL_EMERGENCY_MB=20480
# MG_AUTO_KILL=1 # applies only at emergency
# MG_NOTIFY=1
# MG_DRY_RUN=0
EOF
fi

# Plist paths must stay under $HOME/.local (never ~/Documents — TCC on macOS 26).
cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/bash</string>
    <string>${LIB_DIR}/memory-guard.sh</string>
    <string>run</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>${BIN_DIR}:/usr/bin:/bin:/usr/sbin:/sbin</string>
    <key>HOME</key>
    <string>${HOME_DIR}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ProcessType</key>
  <string>Background</string>
  <key>Nice</key>
  <integer>5</integer>
  <key>StandardOutPath</key>
  <string>${STATE_DIR}/launchd.out.log</string>
  <key>StandardErrorPath</key>
  <string>${STATE_DIR}/launchd.err.log</string>
  <key>ThrottleInterval</key>
  <integer>10</integer>
</dict>
</plist>
EOF

# Validate offline first
/bin/bash "$LIB_DIR/memory-guard.sh" selftest

# Reload agent (gui domain)
if launchctl print "gui/${UID_NUM}/${LABEL}" >/dev/null 2>&1; then
  launchctl bootout "gui/${UID_NUM}/${LABEL}" 2>/dev/null || true
fi
launchctl bootstrap "gui/${UID_NUM}" "$PLIST"
launchctl enable "gui/${UID_NUM}/${LABEL}" 2>/dev/null || true
launchctl kickstart -k "gui/${UID_NUM}/${LABEL}" 2>/dev/null || true

# Wait until the single run instance is alive (KeepAlive).
ok=0
for _ in 1 2 3 4 5 6; do
  sleep 1
  if /bin/bash "$LIB_DIR/memory-guard.sh" status 2>/dev/null | /usr/bin/grep -q 'daemon=running'; then
    ok=1
    break
  fi
done
if [[ "$ok" != "1" ]]; then
  echo "warning: daemon not yet running; launchd will retry KeepAlive" >&2
fi

echo "installed: $LIB_DIR/memory-guard.sh"
echo "cli:       $BIN_DIR/omnimon-memory-guard"
echo "plist:     $PLIST"
echo "state:     $STATE_DIR"
launchctl print "gui/${UID_NUM}/${LABEL}" 2>/dev/null | /usr/bin/head -n 20 || true
