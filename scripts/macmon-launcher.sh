#!/usr/bin/env bash
# macmon-launcher.sh — .app bundle entry point
# Lives at macmon.app/Contents/MacOS/macmon-launcher
# Sets up MACMON_HOME, collects process data, and launches ProcessPicker --standalone

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTENTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SHARED_SUPPORT="${CONTENTS_DIR}/SharedSupport"

export MACMON_HOME="$SHARED_SUPPORT"
export MACMON_CONFIG="${HOME}/.config/macmon/macmon.yaml"

# Ensure config directory exists
CONFIG_DIR="${HOME}/.config/macmon"
if [[ ! -d "$CONFIG_DIR" ]]; then
    mkdir -p "$CONFIG_DIR"
    chmod 700 "$CONFIG_DIR"
fi

# Create default config if missing
if [[ ! -f "$MACMON_CONFIG" ]]; then
    if [[ -f "$SHARED_SUPPORT/config/macmon.default.yaml" ]]; then
        cp "$SHARED_SUPPORT/config/macmon.default.yaml" "$MACMON_CONFIG"
        chmod 600 "$MACMON_CONFIG"
    fi
fi

# Ensure log directory exists
LOG_DIR="${HOME}/.local/log/macmon"
mkdir -p "$LOG_DIR" 2>/dev/null || true

# Source macmon-core for process collection
source "${SHARED_SUPPORT}/lib/macmon-core.sh"
macmon_load_config "$MACMON_CONFIG"

# Collect process data to a temp file
TMPDIR="${TMPDIR:-/tmp}"
MACMON_TMPDIR="${TMPDIR}/macmon-$$"
mkdir -p "$MACMON_TMPDIR"

cleanup() {
    rm -rf "$MACMON_TMPDIR"
}
trap cleanup EXIT

json_file="${MACMON_TMPDIR}/macmon-procs.json"
collect_processes_json \
    "$(macmon_cfg "THRESHOLDS_PROCESS_RAM_MIN_KB" "102400")" \
    "$(macmon_cfg "THRESHOLDS_IDLE_CPU_PERCENT" "1.0")" \
    > "$json_file"

proc_count=$(jq '.processes | length' "$json_file" 2>/dev/null || echo 0)
if (( proc_count == 0 )); then
    osascript -e 'display dialog "No qualifying processes found." buttons {"OK"} default button 1 with title "macmon"' 2>/dev/null || true
    exit 0
fi

# Launch ProcessPicker from Helpers
PICKER="${CONTENTS_DIR}/Helpers/ProcessPicker"
if [[ ! -x "$PICKER" ]]; then
    osascript -e 'display dialog "ProcessPicker binary not found." buttons {"OK"} default button 1 with title "macmon"' 2>/dev/null || true
    exit 1
fi

exec "$PICKER" --file "$json_file" --standalone
