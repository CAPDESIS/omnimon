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

# --- PID File (symlink-safe) ---
PID_FILE="${MACMON_TMPDIR}/macmond.pid"

write_pid() {
    # Prevent symlink attacks: remove any existing file/symlink first,
    # then create with restrictive umask
    rm -f "$PID_FILE"
    (umask 077; echo $$ > "$PID_FILE")
    # Verify it's a regular file, not a symlink
    if [[ -L "$PID_FILE" ]]; then
        macmon_log "SECURITY: PID file is a symlink, refusing to continue"
        rm -f "$PID_FILE"
        exit 1
    fi
}

remove_pid() {
    rm -f "$PID_FILE"
}

# --- Cooldown State ---
LAST_FLUTTER_ALERT=0
LAST_RAM_ALERT=0
LAST_IDLE_ALERT=0
LAST_ORPHAN_ALERT=0

cooldown_elapsed() {
    local last_time="$1"
    local cooldown
    cooldown=$(macmon_cfg "INTERVALS_COOLDOWN" "300")
    [[ "$cooldown" =~ ^[0-9]+$ ]] || cooldown=300
    local now
    now=$(date +%s)
    (( now - last_time >= cooldown ))
}

# --- Signal Handlers ---
RUNNING=true

cleanup() {
    RUNNING=false
    macmon_log "Daemon shutting down (PID $$)"
    macmon_notify "macmon" "Daemon stopped" 2>/dev/null || true
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
                    macmon_notify "macmon" "Freed memory from selected processes"
                    macmon_log "Killed user-selected processes"
                fi
            fi
        else
            macmon_notify "macmon" "Warning: RAM at ${free_pct}% free. Check macmon status for details."
            macmon_log "User declined picker, sent passive notification"
        fi
    fi
}

do_check_orphans() {
    cooldown_elapsed "$LAST_ORPHAN_ALERT" || return 0

    local orphans
    if orphans=$(check_orphan_daemons); then
        LAST_ORPHAN_ALERT=$(date +%s)
        local orphan_summary=""
        local orphan_count=0
        while IFS=: read -r name count reason; do
            orphan_summary="${orphan_summary}\n- ${name}: ${count} (${reason})"
            (( orphan_count += count )) || true
        done <<< "$orphans"

        macmon_log "Orphan daemons detected: $orphan_count total"

        if macmon_ask_yes_no "macmon - Orphan Processes" "Found ${orphan_count} orphan build daemon(s):${orphan_summary}\n\nClean them up?"; then
            while IFS=: read -r name count reason; do
                case "$name" in
                    SourceKitService) kill_orphan_by_pattern "SourceKitService" ;;
                    GradleDaemon)     kill_orphan_by_pattern "GradleDaemon" ;;
                    xcodebuild)       kill_orphan_by_pattern "xcodebuild" ;;
                    qemu-system)      kill_orphan_by_pattern "qemu-system" ;;
                esac
            done <<< "$orphans"
            macmon_notify "macmon" "Cleaned up ${orphan_count} orphan daemon(s)"
            macmon_log "User approved orphan cleanup: $orphan_count processes"
        else
            macmon_log "User declined orphan cleanup"
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
                macmon_notify "macmon" "Cleaned up idle processes"
                macmon_log "Killed idle processes selected by user"
            fi
        fi
    fi
}

# --- Main Loop ---

macmon_log "Daemon started (PID $$, version ${MACMON_VERSION})"
write_pid
macmon_notify "macmon" "Daemon started (v${MACMON_VERSION})"

check_interval=$(macmon_cfg "INTERVALS_CHECK" "60")
idle_interval=$(macmon_cfg "INTERVALS_IDLE_CHECK" "600")
[[ "$check_interval" =~ ^[0-9]+$ ]] || check_interval=60
[[ "$idle_interval" =~ ^[0-9]+$ ]] || idle_interval=600
last_idle_check=0

now=0
i=0

while $RUNNING; do
    rotate_log

    # Run checks
    do_check_flutter
    do_check_orphans
    do_check_ram

    # Idle check at its own interval
    now=$(date +%s)
    if (( now - last_idle_check >= idle_interval )); then
        do_check_idle
        last_idle_check=$now
    fi

    # Invalidate memory pressure cache for next cycle
    _cached_mem_pressure=""
    _cached_mem_pressure_time=0

    # Sleep in small increments so signals are handled promptly
    for (( i = 0; i < check_interval; i++ )); do
        $RUNNING || break
        sleep 1
    done
done
