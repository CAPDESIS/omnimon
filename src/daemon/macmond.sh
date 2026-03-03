#!/usr/bin/env bash
# macmond.sh - macmon background daemon
# Monitors RAM pressure, swap, flutter_tester accumulation, and idle processes

set -euo pipefail

MACMON_HOME="${MACMON_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

# Source shared libraries
source "${MACMON_HOME}/lib/macmon-core.sh"

# --- Configuration ---
macmon_load_config "${MACMON_CONFIG:-$HOME/.config/macmon/macmon.yaml}"

MACMON_LOG_DIR=$(macmon_cfg "LOG_DIR" "$HOME/.local/log/macmon")
MACMON_LOG_FILE="${MACMON_LOG_DIR}/macmond.log"
export MACMON_LOG_DIR MACMON_LOG_FILE

mkdir -p "$MACMON_LOG_DIR"

# --- PID File ---
PID_FILE="${MACMON_TMPDIR}/macmond.pid"

write_pid() {
    echo $$ > "$PID_FILE"
}

remove_pid() {
    rm -f "$PID_FILE"
}

# --- Cooldown State ---
LAST_FLUTTER_ALERT=0
LAST_RAM_ALERT=0
LAST_IDLE_ALERT=0

cooldown_elapsed() {
    local last_time="$1"
    local cooldown
    cooldown=$(macmon_cfg "INTERVALS_COOLDOWN" "300")
    local now
    now=$(date +%s)
    (( now - last_time >= cooldown ))
}

# --- Signal Handlers ---
RUNNING=true

cleanup() {
    RUNNING=false
    macmon_log "Daemon shutting down (PID $$)"
    remove_pid
    # Clean temp files
    rm -f "${MACMON_TMPDIR}"/macmon-*.json 2>/dev/null || true
    exit 0
}

reload_config() {
    macmon_log "Reloading configuration (SIGUSR1)"
    macmon_load_config "${MACMON_CONFIG:-$HOME/.config/macmon/macmon.yaml}"
    macmon_log "Configuration reloaded"
}

trap cleanup SIGTERM SIGINT
trap reload_config SIGUSR1

# --- Monitoring Checks ---

do_check_flutter() {
    cooldown_elapsed "$LAST_FLUTTER_ALERT" || return 0

    if check_flutter_tester; then
        local count
        count=$(pgrep -x flutter_tester 2>/dev/null | wc -l | tr -d ' ')
        LAST_FLUTTER_ALERT=$(date +%s)

        if macmon_ask_yes_no "macmon" "Detected $count flutter_tester processes. Kill them all?"; then
            kill_flutter_testers
            macmon_notify "macmon" "Killed $count flutter_tester processes"
            macmon_log "User approved: killed $count flutter_tester processes"
        else
            macmon_log "User declined flutter_tester cleanup ($count processes)"
        fi
    fi
}

do_check_ram() {
    cooldown_elapsed "$LAST_RAM_ALERT" || return 0

    local free_pct
    free_pct=$(get_free_ram_percent)
    local threshold
    threshold=$(macmon_cfg "THRESHOLDS_RAM_FREE_PERCENT" "25")

    if (( free_pct > 0 && free_pct < threshold )); then
        macmon_log "RAM pressure alert: ${free_pct}% free (threshold: ${threshold}%)"
        LAST_RAM_ALERT=$(date +%s)

        # Get swap info
        local swap_info swap_used_mb
        swap_info=$(sysctl -n vm.swapusage 2>/dev/null || echo "")
        swap_used_mb=0
        if [[ "$swap_info" =~ used[[:space:]]*=[[:space:]]*([0-9]+(\.[0-9]+)?)M ]]; then
            swap_used_mb="${BASH_REMATCH[1]%.*}"
        fi

        local swap_threshold
        swap_threshold=$(macmon_cfg "THRESHOLDS_SWAP_USED_MB" "2048")
        local msg="RAM is low (${free_pct}% free)"
        if (( swap_used_mb > swap_threshold )); then
            msg="${msg}, swap is high (${swap_used_mb}MB used)"
        fi

        if macmon_ask_yes_no "macmon - Memory Pressure" "${msg}. Open process picker to free memory?"; then
            macmon_log "User requested process picker for RAM pressure"
            local selected
            if selected=$(show_process_picker); then
                if [[ -n "$selected" ]]; then
                    local kill_file
                    kill_file=$(mktemp "${MACMON_TMPDIR}/macmon-kill.XXXXXX.json")
                    # Convert PID list to JSON
                    echo "$selected" | jq -R -s '
                        split("\n") | map(select(length > 0)) |
                        map({pid: (. | tonumber), name: "selected"})
                    ' > "$kill_file"
                    kill_processes "$kill_file"
                    rm -f "$kill_file"
                    macmon_log "Killed user-selected processes"
                fi
            fi
        fi
    fi
}

do_check_idle() {
    cooldown_elapsed "$LAST_IDLE_ALERT" || return 0

    local free_pct
    free_pct=$(get_free_ram_percent)
    local idle_trigger
    idle_trigger=$(macmon_cfg "THRESHOLDS_IDLE_RAM_TRIGGER_PERCENT" "40")

    # Only proactively check idle processes when memory is under pressure
    (( free_pct > 0 && free_pct < idle_trigger )) || return 0

    macmon_log "Proactive idle check: ${free_pct}% free RAM (trigger: ${idle_trigger}%)"
    LAST_IDLE_ALERT=$(date +%s)

    if macmon_ask_yes_no "macmon - Idle Processes" "RAM is at ${free_pct}% free. Review idle processes to free memory?"; then
        local selected
        if selected=$(show_process_picker); then
            if [[ -n "$selected" ]]; then
                local kill_file
                kill_file=$(mktemp "${MACMON_TMPDIR}/macmon-kill.XXXXXX.json")
                echo "$selected" | jq -R -s '
                    split("\n") | map(select(length > 0)) |
                    map({pid: (. | tonumber), name: "selected"})
                ' > "$kill_file"
                kill_processes "$kill_file"
                rm -f "$kill_file"
                macmon_log "Killed idle processes selected by user"
            fi
        fi
    fi
}

# --- Main Loop ---

macmon_log "Daemon started (PID $$, version ${MACMON_VERSION})"
write_pid

check_interval=$(macmon_cfg "INTERVALS_CHECK" "60")
idle_interval=$(macmon_cfg "INTERVALS_IDLE_CHECK" "600")
last_idle_check=0

while $RUNNING; do
    rotate_log

    # Run checks
    do_check_flutter
    do_check_ram

    # Idle check at its own interval
    local now
    now=$(date +%s)
    if (( now - last_idle_check >= idle_interval )); then
        do_check_idle
        last_idle_check=$now
    fi

    # Invalidate memory pressure cache for next cycle
    _cached_mem_pressure=""
    _cached_mem_pressure_time=0

    # Sleep in small increments so signals are handled promptly
    local i
    for (( i = 0; i < check_interval && RUNNING; i++ )); do
        sleep 1
    done
done
