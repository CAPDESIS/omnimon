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

# --- Message Catalog (CLI i18n-ready) ---
# Future language packs can override these variables before sourcing commands.
MSG_CLOSED_PROCESSES="Closed"
MSG_STATUS_TITLE="macmon"
MSG_DAEMON_ALREADY_RUNNING="Daemon is already running"
MSG_DAEMON_NOT_RUNNING="Daemon is not running"
MSG_DAEMON_STARTED="Daemon started"
MSG_DAEMON_STOPPED="Daemon stopped"
MSG_LAUNCHAGENT_MISSING="Error: LaunchAgent plist not found at"
MSG_RUN_INSTALL_FIRST="Run 'macmon install' or the install.sh script first"
MSG_CREATED_CONFIG="Created config at"
MSG_RESET_CONFIG="Config reset to defaults at"
MSG_NO_CONFIG="\# No user config found. Using defaults:"
MSG_CREATE_CONFIG_HINT="\# (Create with: macmon config edit)"
MSG_NO_LOG="No log file found at"
MSG_COLLECTING="Collecting process data..."
MSG_EXPORTED="Exported to"
MSG_NO_PEAKS="No peak data found. Start the daemon to collect peaks."
MSG_PEAK_FILES="Peak data files:"
MSG_LATEST_PEAKS="Latest peaks:"
MSG_EXPORT_USAGE="Usage: macmon export [json|csv|--peaks]"
MSG_VERSION="macmon"
MSG_UNKNOWN_COMMAND="Unknown command:"
MSG_RUN_HELP="Run 'macmon help' for usage"
MSG_PROFILE_CURRENT="Active profile:"
MSG_PROFILE_SWITCHED="Switched profile:"
MSG_PROFILE_MISSING="Profile not found:"
MSG_UPDATE_CHECKING="Checking for updates..."
MSG_UPDATE_UP_TO_DATE="Already up to date"
MSG_UPDATE_AVAILABLE="Update available:"
MSG_UPDATE_DOWNLOADING="Downloading update..."
MSG_UPDATE_INSTALLING="Installing update..."
MSG_UPDATE_DONE="Updated successfully to"
MSG_UPDATE_FAILED="Update failed"

# --- Subcommands ---

cmd_picker() {
    local selected
    if selected=$(show_process_picker); then
        if [[ -n "$selected" ]]; then
            local count
            count=$(echo "$selected" | wc -l | tr -d ' ')
            local kill_file
            kill_file=$(mktemp "${MACMON_TMPDIR}/macmon-kill.XXXXXX.json")
            build_kill_payload_json "$selected" "$kill_file"
            count=$(jq 'length' "$kill_file" 2>/dev/null || echo 0)
            kill_processes "$kill_file"
            rm -f "$kill_file"
            echo "$MSG_CLOSED_PROCESSES $count process(es)"
        fi
    fi
}

cmd_status() {
    echo "$MSG_STATUS_TITLE v${MACMON_VERSION} - System Status"
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
    local active_profile
    active_profile=$(macmon_get_active_profile 2>/dev/null || echo "default")
    printf "  Profile: %s\n" "$active_profile"

    # Custom process thresholds
    local custom_lines
    if custom_lines=$(macmon_get_custom_processes 2>/dev/null); then
        while IFS=: read -r proc_name max_inst max_ram max_cpu; do
            [[ -n "$proc_name" ]] || continue
            local proc_count
            proc_count=$(pgrep -x "$proc_name" 2>/dev/null | wc -l | tr -d ' ')
            (( proc_count > 0 )) || continue
            local color="\033[32m"
            if (( max_inst > 0 && proc_count > max_inst )); then
                color="\033[31m"
            fi
            printf "  Custom:  ${color}%s\033[0m %s" "$proc_count" "$proc_name"
            local limits=()
            (( max_inst > 0 )) && limits+=("max_instances=${max_inst}")
            (( max_ram > 0 )) && limits+=("max_ram_mb=${max_ram}")
            (( max_cpu > 0 )) && limits+=("max_cpu_percent=${max_cpu}")
            if (( ${#limits[@]} > 0 )); then
                local IFS=', '
                printf " [%s]" "${limits[*]}"
            fi
            printf "\n"
        done <<< "$custom_lines"
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

signal_daemon_reload_if_running() {
    local pid_file="${TMPDIR:-/tmp}/macmond.pid"
    [[ -f "$pid_file" ]] || return 0
    local daemon_pid
    daemon_pid=$(tr -d '[:space:]' < "$pid_file" 2>/dev/null || true)
    [[ "$daemon_pid" =~ ^[0-9]+$ ]] || return 0
    kill -USR1 "$daemon_pid" 2>/dev/null || true
}

cmd_profile() {
    case "${1:-}" in
        list)
            macmon_list_profiles
            ;;
        current|"")
            local current
            current=$(macmon_get_active_profile 2>/dev/null || echo "default")
            echo "$MSG_PROFILE_CURRENT $current"
            ;;
        use)
            local profile_name="${2:-}"
            if [[ -z "$profile_name" ]]; then
                echo "Usage: macmon profile use <name>"
                return 1
            fi
            if ! macmon_set_active_profile "$profile_name"; then
                echo "$MSG_PROFILE_MISSING $profile_name"
                return 1
            fi
            macmon_load_config ""
            signal_daemon_reload_if_running
            echo "$MSG_PROFILE_SWITCHED $profile_name"
            ;;
        *)
            echo "Usage: macmon profile [list|current|use <name>]"
            return 1
            ;;
    esac
}

cmd_start() {
    if launchctl list "$PLIST_LABEL" &>/dev/null; then
        echo "$MSG_DAEMON_ALREADY_RUNNING"
        return 0
    fi
    if [[ ! -f "$PLIST_PATH" ]]; then
        echo "$MSG_LAUNCHAGENT_MISSING $PLIST_PATH"
        echo "$MSG_RUN_INSTALL_FIRST"
        return 1
    fi
    launchctl load -w "$PLIST_PATH"
    echo "$MSG_DAEMON_STARTED"
}

cmd_stop() {
    if ! launchctl list "$PLIST_LABEL" &>/dev/null; then
        echo "$MSG_DAEMON_NOT_RUNNING"
        return 0
    fi
    launchctl unload "$PLIST_PATH"
    echo "$MSG_DAEMON_STOPPED"
}

cmd_restart() {
    cmd_stop 2>/dev/null || true
    sleep 1
    cmd_start
}

cmd_config() {
    local config_file="${MACMON_CONFIG:-$HOME/.config/macmon/macmon.yaml}"
    local safe_config=""
    safe_config=$(_validated_config_path "$config_file" || true)
    if [[ "${1:-}" == "edit" ]]; then
        if [[ ! -f "$config_file" ]]; then
            mkdir -p "$(dirname "$config_file")"
            cp "${MACMON_HOME}/config/macmon.default.yaml" "$config_file"
            echo "$MSG_CREATED_CONFIG $config_file"
        fi
        ${EDITOR:-nano} "$config_file"
    elif [[ "${1:-}" == "path" ]]; then
        echo "$config_file"
    elif [[ "${1:-}" == "reset" ]]; then
        mkdir -p "$(dirname "$config_file")"
        cp "${MACMON_HOME}/config/macmon.default.yaml" "$config_file"
        echo "$MSG_RESET_CONFIG $config_file"
    else
        if [[ -n "$safe_config" && -f "$safe_config" ]]; then
            echo "# Active config: $config_file"
            cat "$safe_config"
        else
            echo "$MSG_NO_CONFIG"
            echo "$MSG_CREATE_CONFIG_HINT"
            cat "${MACMON_HOME}/config/macmon.default.yaml"
        fi
    fi
}

cmd_log() {
    local log_file="$MACMON_LOG_FILE"
    if [[ ! -f "$log_file" ]]; then
        echo "$MSG_NO_LOG $log_file"
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

    echo "$MSG_COLLECTING"
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
            echo "$MSG_EXPORTED $outfile"
            ;;
        csv)
            local outfile="${output_dir}/macmon-${timestamp}.csv"
            printf '%s' "$json" | jq -r '
                ["PID","Name","RAM_MB","CPU_Pct","Uptime","UptimeSec","Idle","Group","State","CWD","Detail"],
                (.processes[] | [.pid,.name,.ramMB,.cpuPct,.uptime,.uptimeSeconds,.idle,.group,.state,.cwd,.detail])
                | @csv
            ' > "$outfile"
            echo "$MSG_EXPORTED $outfile"
            ;;
        --peaks)
            local peak_dir="${MACMON_LOG_DIR}"
            local peak_files
            peak_files=$(ls -1 "$peak_dir"/peaks-*.json 2>/dev/null || true)
            if [[ -z "$peak_files" ]]; then
                echo "$MSG_NO_PEAKS"
                return 1
            fi
            echo "$MSG_PEAK_FILES"
            echo "$peak_files"
            echo ""
            echo "$MSG_LATEST_PEAKS"
            local latest
            latest=$(echo "$peak_files" | tail -1)
            jq '.' "$latest"
            ;;
        *)
            echo "$MSG_EXPORT_USAGE"
            return 1
            ;;
    esac
}

cmd_update() {
    local api_url="https://api.github.com/repos/chochy2001/macmon/releases/latest"

    echo "$MSG_UPDATE_CHECKING"

    if ! command -v jq >/dev/null 2>&1; then
        echo "$MSG_UPDATE_FAILED: jq is required for secure update parsing"
        return 1
    fi

    local release_json
    release_json=$(curl -fsSL "$api_url" 2>/dev/null) || {
        echo "$MSG_UPDATE_FAILED: could not reach GitHub API"
        return 1
    }

    local remote_tag
    remote_tag=$(printf '%s' "$release_json" | jq -r '.tag_name // empty')
    if [[ -z "$remote_tag" ]]; then
        echo "$MSG_UPDATE_FAILED: could not parse release tag"
        return 1
    fi
    local remote_version="${remote_tag#v}"

    # Compare versions using sort -V
    local newest
    newest=$(printf '%s\n%s\n' "$MACMON_VERSION" "$remote_version" | sort -V | tail -1)
    if [[ "$newest" == "$MACMON_VERSION" && "$MACMON_VERSION" == "$remote_version" ]] || [[ "$newest" == "$MACMON_VERSION" && "$newest" != "$remote_version" ]]; then
        echo "$MSG_UPDATE_UP_TO_DATE (v${MACMON_VERSION})"
        return 0
    fi

    echo "$MSG_UPDATE_AVAILABLE v${MACMON_VERSION} -> v${remote_version}"

    # Parse download URL for universal tarball
    local archive_name="macmon-${remote_version}-macos-universal.tar.gz"
    local asset_url
    asset_url=$(printf '%s' "$release_json" | jq -r --arg archive_name "$archive_name" '
        [.assets[]? | select(.name == $archive_name) | .browser_download_url][0] // empty
    ')
    if [[ -z "$asset_url" ]]; then
        echo "$MSG_UPDATE_FAILED: could not find ${archive_name} in release assets"
        return 1
    fi

    local tmpdir="${TMPDIR:-/tmp}/macmon-update-$$"
    mkdir -p "$tmpdir"
    trap 'rm -rf "$tmpdir"' RETURN

    echo "$MSG_UPDATE_DOWNLOADING"
    curl -fSL -o "${tmpdir}/${archive_name}" "$asset_url" || {
        echo "$MSG_UPDATE_FAILED: download error"
        return 1
    }

    echo "$MSG_UPDATE_INSTALLING"
    tar xzf "${tmpdir}/${archive_name}" -C "$tmpdir" || {
        echo "$MSG_UPDATE_FAILED: extraction error"
        return 1
    }

    local install_src="${tmpdir}/macmon"
    if [[ ! -d "$install_src" ]]; then
        install_src="$tmpdir"
    fi

    if [[ ! -f "${install_src}/install.sh" ]]; then
        echo "$MSG_UPDATE_FAILED: install.sh not found in archive"
        return 1
    fi

    cd "$install_src"
    bash install.sh || {
        echo "$MSG_UPDATE_FAILED: install.sh returned an error"
        return 1
    }

    echo "$MSG_UPDATE_DONE v${remote_version}"
}

cmd_version() {
    echo "$MSG_VERSION v${MACMON_VERSION}"
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
  profile         Show active profile
  profile list    List available profiles
  profile use X   Switch active profile
  log             Show last 50 lines of daemon log
  log -f          Follow daemon log (tail -f)
  update          Check for updates and install if available
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
  macmon update           # Check and install updates

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
    profile)        shift; cmd_profile "$@" ;;
    log)            shift; cmd_log "$@" ;;
    update)         cmd_update ;;
    version|--version|-v)  cmd_version ;;
    help|--help|-h) cmd_help ;;
    "")             cmd_picker ;;
    *)
        echo "$MSG_UNKNOWN_COMMAND $1"
        echo "$MSG_RUN_HELP"
        exit 1
        ;;
esac
