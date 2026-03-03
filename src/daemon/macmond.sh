#!/usr/bin/env bash
# macmond.sh - macmon background daemon
# Monitors RAM pressure, swap, custom process thresholds, and idle processes

set -euo pipefail

MACMON_HOME="${MACMON_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

# Source shared libraries
source "${MACMON_HOME}/lib/macmon-core.sh"

# --- Configuration ---
macmon_load_config ""

MACMON_LOG_DIR=$(macmon_cfg "LOG_DIR" "$HOME/.local/log/macmon")
MACMON_LOG_FILE="${MACMON_LOG_DIR}/macmond.log"
export MACMON_LOG_DIR MACMON_LOG_FILE

mkdir -p "$MACMON_LOG_DIR"

# --- PID File (symlink-safe) ---
PID_FILE="${MACMON_TMPDIR}/macmond.pid"
PID_LOCK_DIR="${MACMON_TMPDIR}/macmond.pid.lock"
WATCHED_CONFIG_FILE=""
LAST_CONFIG_MTIME=0

write_pid() {
    local lock_wait=0
    while ! mkdir "$PID_LOCK_DIR" 2>/dev/null; do
        (( lock_wait++ )) || true
        if (( lock_wait > 50 )); then
            macmon_log "SECURITY: could not acquire PID lock"
            exit 1
        fi
        sleep 0.1
    done

    local tmp_pid
    tmp_pid=$(mktemp "${MACMON_TMPDIR}/macmond.pid.XXXXXX")
    (umask 077; printf '%s\n' "$$" > "$tmp_pid")
    mv -f "$tmp_pid" "$PID_FILE"

    if [[ -L "$PID_FILE" || ! -f "$PID_FILE" ]]; then
        macmon_log "SECURITY: invalid PID file type, refusing to continue"
        rm -f "$PID_FILE"
        rmdir "$PID_LOCK_DIR" 2>/dev/null || true
        exit 1
    fi
    rmdir "$PID_LOCK_DIR" 2>/dev/null || true
}

remove_pid() {
    while ! mkdir "$PID_LOCK_DIR" 2>/dev/null; do
        sleep 0.1
    done
    rm -f "$PID_FILE"
    rmdir "$PID_LOCK_DIR" 2>/dev/null || true
}

# --- Cooldown State ---
LAST_CUSTOM_ALERT=0
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
SLEEP_PID=""

cleanup() {
    RUNNING=false
    # Kill any sleeping child so the loop exits immediately
    [[ -n "$SLEEP_PID" ]] && kill "$SLEEP_PID" 2>/dev/null || true
    # Reap all background children (prevents zombie accumulation)
    wait 2>/dev/null || true
    macmon_log "Daemon shutting down (PID $$)"
    macmon_notify "macmon" "Daemon stopped" 2>/dev/null || true
    remove_pid
    # Clean temp files
    rm -f "${MACMON_TMPDIR}"/macmon-*.json 2>/dev/null || true
    exit 0
}

reload_config() {
    macmon_log "Reloading configuration (SIGUSR1)"
    macmon_load_config ""
    macmon_invalidate_custom_processes_cache
    # Re-read intervals in case they changed
    check_interval=$(macmon_cfg "INTERVALS_CHECK" "60")
    idle_interval=$(macmon_cfg "INTERVALS_IDLE_CHECK" "600")
    [[ "$check_interval" =~ ^[0-9]+$ ]] || check_interval=60
    [[ "$idle_interval" =~ ^[0-9]+$ ]] || idle_interval=600
    WATCHED_CONFIG_FILE=$(macmon_get_loaded_config_path 2>/dev/null || true)
    if [[ -z "$WATCHED_CONFIG_FILE" ]]; then
        WATCHED_CONFIG_FILE=$(macmon_resolve_config_file "" || true)
    fi
    if [[ -n "$WATCHED_CONFIG_FILE" && -f "$WATCHED_CONFIG_FILE" ]]; then
        LAST_CONFIG_MTIME=$(stat -f %m "$WATCHED_CONFIG_FILE" 2>/dev/null || echo 0)
    fi
    if [[ -n "${MACMON_CFG_CONFIG_ERROR:-}" ]]; then
        macmon_log "CONFIG WARNING: ${MACMON_CFG_CONFIG_ERROR}; defaults remain active"
        macmon_notify "macmon - Config Warning" "Invalid configuration detected (${MACMON_CFG_CONFIG_ERROR}). Using safe defaults."
    fi
    macmon_log "Configuration reloaded (check=${check_interval}s, idle=${idle_interval}s)"
}

watch_config_changes() {
    [[ -n "$WATCHED_CONFIG_FILE" && -f "$WATCHED_CONFIG_FILE" ]] || return 0
    local current_mtime
    current_mtime=$(stat -f %m "$WATCHED_CONFIG_FILE" 2>/dev/null || echo 0)
    [[ "$current_mtime" =~ ^[0-9]+$ ]] || return 0
    (( current_mtime > LAST_CONFIG_MTIME )) || return 0
    LAST_CONFIG_MTIME="$current_mtime"
    macmon_log "Config file changed on disk: $WATCHED_CONFIG_FILE"
    reload_config
}

trap cleanup SIGTERM SIGINT
trap reload_config SIGUSR1

# --- Monitoring Checks ---

do_check_custom_processes() {
    cooldown_elapsed "$LAST_CUSTOM_ALERT" || return 0

    local violations
    if violations=$(check_custom_processes); then
        LAST_CUSTOM_ALERT=$(date +%s)

        # Build a human-readable summary
        local summary="" violation_count=0
        while IFS=: read -r vname vtype vcurrent vthreshold; do
            [[ -z "$vname" ]] && continue
            (( violation_count++ )) || true
            case "$vtype" in
                instances) summary+=$'\n'; summary+="- ${vname}: ${vcurrent} instances (max ${vthreshold})" ;;
                ram)       summary+=$'\n'; summary+="- ${vname}: ${vcurrent}MB RAM (max ${vthreshold}MB)" ;;
                cpu)       summary+=$'\n'; summary+="- ${vname}: ${vcurrent}% CPU (max ${vthreshold}%)" ;;
            esac
        done <<< "$violations"

        local prompt
        prompt="Detected ${violation_count} process threshold violation(s):${summary}"
        prompt+=$'\n\nKill offending processes?'
        if macmon_ask_yes_no "macmon - Process Alert" "$prompt"; then
            local killed_names=":"
            while IFS=: read -r vname vtype vcurrent vthreshold; do
                [[ -z "$vname" ]] && continue
                if [[ "$killed_names" == *":${vname}:"* ]]; then
                    continue
                fi
                killed_names+="${vname}:"
                kill_process_by_name "$vname"
            done <<< "$violations"
            macmon_notify "macmon" "Cleaned up ${violation_count} process violation(s)"
            macmon_log "User approved: cleaned up $violation_count custom process violations"
        else
            macmon_log "User declined custom process cleanup ($violation_count violations)"
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
            orphan_summary+=$'\n'
            orphan_summary+="- ${name}: ${count} (${reason})"
            (( orphan_count += count )) || true
        done <<< "$orphans"

        macmon_log "Orphan daemons detected: $orphan_count total"

        local orphan_prompt
        orphan_prompt="Found ${orphan_count} orphan build daemon(s):${orphan_summary}"
        orphan_prompt+=$'\n\nClean them up?'
        if macmon_ask_yes_no "macmon - Orphan Processes" "$orphan_prompt"; then
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

# Close stdin (daemon runs headless, no interactive input)
exec 0</dev/null

macmon_log "Daemon started (PID $$, version ${MACMON_VERSION})"
write_pid
macmon_notify "macmon" "Daemon started (v${MACMON_VERSION})"

check_interval=$(macmon_cfg "INTERVALS_CHECK" "60")
idle_interval=$(macmon_cfg "INTERVALS_IDLE_CHECK" "600")
[[ "$check_interval" =~ ^[0-9]+$ ]] || check_interval=60
[[ "$idle_interval" =~ ^[0-9]+$ ]] || idle_interval=600
last_idle_check=0

now=0

while $RUNNING; do
    rotate_log
    watch_config_changes

    # Run checks
    do_check_custom_processes
    do_check_orphans
    do_check_ram

    # Idle check at its own interval
    now=$(date +%s)
    if (( now - last_idle_check >= idle_interval )); then
        do_check_idle
        last_idle_check=$now
    fi

    # Reap any background children from graceful-quit.sh invocations
    # (prevents zombie process accumulation over days of uptime)
    wait 2>/dev/null || true

    # Invalidate memory pressure cache for next cycle
    _cached_mem_pressure=""
    _cached_mem_pressure_time=0

    # Energy-efficient sleep: single sleep call instead of 1-second ticks.
    # Signals (SIGTERM/SIGINT/SIGUSR1) interrupt the sleep immediately
    # because we trap them and kill the sleep child.
    $RUNNING || break
    sleep "$check_interval" &
    SLEEP_PID=$!
    wait "$SLEEP_PID" 2>/dev/null || true
    SLEEP_PID=""
done
