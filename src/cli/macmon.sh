#!/usr/bin/env bash
# macmon - macOS system monitor CLI
# Usage: macmon [command] [options]

set -euo pipefail

MACMON_HOME="${MACMON_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
source "${MACMON_HOME}/lib/macmon-core.sh"
macmon_load_config "${MACMON_CONFIG:-$HOME/.config/macmon/macmon.yaml}"

PLIST_LABEL="com.macmon.daemon"
PLIST_PATH="$HOME/Library/LaunchAgents/${PLIST_LABEL}.plist"
MACMON_LOG_DIR=$(macmon_cfg "LOG_DIR" "$HOME/.local/log/macmon")
MACMON_LOG_FILE="${MACMON_LOG_DIR}/macmond.log"
export MACMON_LOG_DIR MACMON_LOG_FILE

# --- Subcommands ---

cmd_picker() {
    local selected
    if selected=$(show_process_picker); then
        if [[ -n "$selected" ]]; then
            local count
            count=$(echo "$selected" | wc -l | tr -d ' ')
            local kill_file
            kill_file=$(mktemp "${MACMON_TMPDIR}/macmon-kill.XXXXXX.json")
            echo "$selected" | jq -R -s '
                split("\n") | map(select(length > 0)) |
                map({pid: (. | tonumber), name: "selected"})
            ' > "$kill_file"
            kill_processes "$kill_file"
            rm -f "$kill_file"
            echo "Closed $count process(es)"
        fi
    fi
}

cmd_status() {
    echo "macmon v${MACMON_VERSION} - System Status"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Memory
    local free_pct
    free_pct=$(get_free_ram_percent)
    local phys_mem
    phys_mem=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
    local phys_gb
    phys_gb=$(awk "BEGIN {printf \"%.0f\", ${phys_mem}/1073741824}")

    local mem_color="\033[32m"  # green
    local threshold
    threshold=$(macmon_cfg "THRESHOLDS_RAM_FREE_PERCENT" "25")
    if (( free_pct < threshold )); then
        mem_color="\033[31m"  # red
    elif (( free_pct < 50 )); then
        mem_color="\033[33m"  # yellow
    fi
    printf "  RAM:     ${mem_color}%s%% free\033[0m of %sGB\n" "$free_pct" "$phys_gb"

    # Swap
    local swap_info
    swap_info=$(sysctl -n vm.swapusage 2>/dev/null || echo "")
    local swap_used="0"
    local swap_total="0"
    if [[ "$swap_info" =~ total[[:space:]]*=[[:space:]]*([0-9]+(\.[0-9]+)?)M ]]; then
        swap_total="${BASH_REMATCH[1]}"
    fi
    if [[ "$swap_info" =~ used[[:space:]]*=[[:space:]]*([0-9]+(\.[0-9]+)?)M ]]; then
        swap_used="${BASH_REMATCH[1]}"
    fi
    local swap_color="\033[32m"
    local swap_threshold
    swap_threshold=$(macmon_cfg "THRESHOLDS_SWAP_USED_MB" "2048")
    if awk "BEGIN {exit !(${swap_used} > ${swap_threshold})}"; then
        swap_color="\033[31m"
    elif awk "BEGIN {exit !(${swap_used} > 512)}"; then
        swap_color="\033[33m"
    fi
    printf "  Swap:    ${swap_color}%.0fMB\033[0m used of %.0fMB\n" "$swap_used" "$swap_total"

    # Processes
    local total_procs
    total_procs=$(ps -Ae -o pid= 2>/dev/null | wc -l | tr -d ' ')
    printf "  Procs:   %s total\n" "$total_procs"

    # Flutter testers
    local flutter_count
    flutter_count=$(pgrep -x flutter_tester 2>/dev/null | wc -l | tr -d ' ')
    if (( flutter_count > 0 )); then
        local flutter_threshold
        flutter_threshold=$(macmon_cfg "THRESHOLDS_FLUTTER_PROCESS_COUNT" "10")
        local fc_color="\033[32m"
        if (( flutter_count > flutter_threshold )); then
            fc_color="\033[31m"
        fi
        printf "  Flutter: ${fc_color}%s\033[0m flutter_tester processes\n" "$flutter_count"
    fi

    # Orphan build daemons
    local sk_count gradle_count xb_count qemu_count
    sk_count=$(pgrep -x SourceKitService 2>/dev/null | wc -l | tr -d ' ')
    gradle_count=$(pgrep -f GradleDaemon 2>/dev/null | wc -l | tr -d ' ')
    xb_count=$(pgrep -x xcodebuild 2>/dev/null | wc -l | tr -d ' ')
    qemu_count=$(pgrep -f qemu-system 2>/dev/null | wc -l | tr -d ' ')
    local orphan_total=$(( sk_count + gradle_count + xb_count + qemu_count ))
    if (( orphan_total > 0 )); then
        printf "  Orphans: "
        local parts=()
        (( sk_count > 0 )) && parts+=("SourceKit:$sk_count")
        (( gradle_count > 0 )) && parts+=("Gradle:$gradle_count")
        (( xb_count > 0 )) && parts+=("xcodebuild:$xb_count")
        (( qemu_count > 0 )) && parts+=("qemu:$qemu_count")
        local IFS=', '
        printf "\033[33m%s\033[0m\n" "${parts[*]}"
    fi

    # Daemon status
    echo ""
    if launchctl list "$PLIST_LABEL" &>/dev/null; then
        printf "  Daemon:  \033[32mrunning\033[0m\n"
    else
        printf "  Daemon:  \033[31mstopped\033[0m\n"
    fi

    echo ""
}

cmd_start() {
    if launchctl list "$PLIST_LABEL" &>/dev/null; then
        echo "Daemon is already running"
        return 0
    fi
    if [[ ! -f "$PLIST_PATH" ]]; then
        echo "Error: LaunchAgent plist not found at $PLIST_PATH"
        echo "Run 'macmon install' or the install.sh script first"
        return 1
    fi
    launchctl load -w "$PLIST_PATH"
    echo "Daemon started"
}

cmd_stop() {
    if ! launchctl list "$PLIST_LABEL" &>/dev/null; then
        echo "Daemon is not running"
        return 0
    fi
    launchctl unload "$PLIST_PATH"
    echo "Daemon stopped"
}

cmd_restart() {
    cmd_stop 2>/dev/null || true
    sleep 1
    cmd_start
}

cmd_config() {
    local config_file="${MACMON_CONFIG:-$HOME/.config/macmon/macmon.yaml}"
    if [[ "${1:-}" == "edit" ]]; then
        if [[ ! -f "$config_file" ]]; then
            mkdir -p "$(dirname "$config_file")"
            cp "${MACMON_HOME}/config/macmon.default.yaml" "$config_file"
            echo "Created config at $config_file"
        fi
        ${EDITOR:-nano} "$config_file"
    elif [[ "${1:-}" == "path" ]]; then
        echo "$config_file"
    elif [[ "${1:-}" == "reset" ]]; then
        mkdir -p "$(dirname "$config_file")"
        cp "${MACMON_HOME}/config/macmon.default.yaml" "$config_file"
        echo "Config reset to defaults at $config_file"
    else
        if [[ -f "$config_file" ]]; then
            echo "# Active config: $config_file"
            cat "$config_file"
        else
            echo "# No user config found. Using defaults:"
            echo "# (Create with: macmon config edit)"
            cat "${MACMON_HOME}/config/macmon.default.yaml"
        fi
    fi
}

cmd_log() {
    local log_file="$MACMON_LOG_FILE"
    if [[ ! -f "$log_file" ]]; then
        echo "No log file found at $log_file"
        return 1
    fi
    if [[ "${1:-}" == "--follow" || "${1:-}" == "-f" ]]; then
        tail -f "$log_file"
    else
        tail -50 "$log_file"
    fi
}

cmd_export() {
    local format="${1:-json}"
    local output_dir="$HOME/.local/share/macmon/exports"
    local timestamp
    timestamp=$(date '+%Y%m%d-%H%M%S')

    mkdir -p "$output_dir"

    echo "Collecting process data..."
    local json
    json=$(collect_processes_json \
        "$(macmon_cfg "THRESHOLDS_PROCESS_RAM_MIN_KB" "102400")" \
        "$(macmon_cfg "THRESHOLDS_IDLE_CPU_PERCENT" "1.0")")

    case "$format" in
        json)
            local outfile="${output_dir}/macmon-${timestamp}.json"
            printf '%s' "$json" | jq --arg ts "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" '{
                timestamp: $ts,
                snapshot: .
            }' > "$outfile"
            echo "Exported to $outfile"
            ;;
        csv)
            local outfile="${output_dir}/macmon-${timestamp}.csv"
            printf '%s' "$json" | jq -r '
                ["PID","Name","RAM_MB","CPU_Pct","Uptime","UptimeSec","Idle","Group","State","CWD","Detail"],
                (.processes[] | [.pid,.name,.ramMB,.cpuPct,.uptime,.uptimeSeconds,.idle,.group,.state,.cwd,.detail])
                | @csv
            ' > "$outfile"
            echo "Exported to $outfile"
            ;;
        --peaks)
            local peak_dir="${MACMON_LOG_DIR}"
            local peak_files
            peak_files=$(ls -1 "$peak_dir"/peaks-*.json 2>/dev/null || true)
            if [[ -z "$peak_files" ]]; then
                echo "No peak data found. Start the daemon to collect peaks."
                return 1
            fi
            echo "Peak data files:"
            echo "$peak_files"
            echo ""
            echo "Latest peaks:"
            local latest
            latest=$(echo "$peak_files" | tail -1)
            jq '.' "$latest"
            ;;
        *)
            echo "Usage: macmon export [json|csv|--peaks]"
            return 1
            ;;
    esac
}

cmd_version() {
    echo "macmon v${MACMON_VERSION}"
}

cmd_help() {
    cat <<EOF
macmon v${MACMON_VERSION} - macOS System Monitor

Usage: macmon [command] [options]

Commands:
  (default)       Open the process picker UI
  status          Show system health summary
  start           Start the background daemon
  stop            Stop the background daemon
  restart         Restart the background daemon
  config          Show current configuration
  config edit     Edit configuration in \$EDITOR
  config reset    Reset configuration to defaults
  export [json]   Export current snapshot as JSON
  export csv      Export current snapshot as CSV
  export --peaks  Show daily peak consumption data
  log             Show last 50 lines of daemon log
  log -f          Follow daemon log (tail -f)
  version         Show version
  help            Show this help message

Options:
  --min-ram MB    Minimum RAM (MB) for process picker (default: 100)

Examples:
  macmon                  # Open process picker
  macmon status           # Check system health
  macmon start            # Start monitoring daemon
  macmon config edit      # Customize thresholds
  macmon log -f           # Watch daemon activity

EOF
}

# --- Main ---

case "${1:-}" in
    status)         cmd_status ;;
    start)          cmd_start ;;
    stop)           cmd_stop ;;
    restart)        cmd_restart ;;
    config)         shift; cmd_config "$@" ;;
    export)         shift; cmd_export "$@" ;;
    log)            shift; cmd_log "$@" ;;
    version|--version|-v)  cmd_version ;;
    help|--help|-h) cmd_help ;;
    "")             cmd_picker ;;
    *)
        echo "Unknown command: $1"
        echo "Run 'macmon help' for usage"
        exit 1
        ;;
esac
