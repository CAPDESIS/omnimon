#!/usr/bin/env bash
# macmon-core.sh - Shared functions for macmon
# Sources: macmon-config.sh for configuration values

set -euo pipefail

export MACMON_VERSION="4.0.5"
MACMON_HOME="${MACMON_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

# --- Validate MACMON_HOME (MITRE T1574 - Hijack Execution Flow) ---
# Prevent path injection and privilege escalation via tampered MACMON_HOME
_validate_macmon_home() {
    local path="$1"
    # Must be an absolute path
    if [[ "$path" != /* ]]; then
        echo "macmon: ERROR: MACMON_HOME must be an absolute path" >&2
        return 1
    fi
    # Reject shell metacharacters
    if [[ "$path" =~ [\'\"\\\`\$\!\;\|\&\(\)] ]]; then
        echo "macmon: ERROR: MACMON_HOME contains forbidden characters" >&2
        return 1
    fi
    # Must be a directory
    if [[ ! -d "$path" ]]; then
        echo "macmon: ERROR: MACMON_HOME does not exist: $path" >&2
        return 1
    fi
    # Must be owned by current user (no other user can plant malicious scripts)
    local dir_owner
    dir_owner=$(stat -f%u "$path" 2>/dev/null) || return 1
    if [[ "$dir_owner" != "$(id -u)" ]]; then
        echo "macmon: ERROR: MACMON_HOME not owned by current user" >&2
        return 1
    fi
    # Must not be world-writable
    local perms
    perms=$(stat -f%Lp "$path" 2>/dev/null) || return 1
    if [[ "${perms: -1}" =~ [2367] ]]; then
        echo "macmon: ERROR: MACMON_HOME is world-writable" >&2
        return 1
    fi
}
_validate_macmon_home "$MACMON_HOME" || exit 1

# Source config loader
# shellcheck source=macmon-config.sh
source "${MACMON_HOME}/lib/macmon-config.sh"
# shellcheck source=macmon-security.sh
source "${MACMON_HOME}/lib/macmon-security.sh"

# --- Temp directory (per-user private) ---
MACMON_TMPDIR="${TMPDIR:-/tmp}"

# --- Logging ---

MACMON_LOG_DIR="${MACMON_LOG_DIR:-$HOME/.local/log/macmon}"

macmon_log() {
    local log_file="${MACMON_LOG_FILE:-${MACMON_LOG_DIR}/macmond.log}"
    local dir
    dir="$(dirname "$log_file")"
    [[ -d "$dir" ]] || mkdir -p "$dir"
    # Strip ANSI escape sequences to prevent log injection via terminal rendering
    local msg
    msg=$(printf '%s' "$*" | sed $'s/\x1b\[[0-9;]*[a-zA-Z]//g')
    printf '%s [macmon] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$msg" >> "$log_file"
}

rotate_log() {
    local log_file="${MACMON_LOG_FILE:-${MACMON_LOG_DIR}/macmond.log}"
    local max_size_bytes max_files
    max_size_bytes=$(( $(macmon_cfg "LOG_MAX_SIZE_MB" "10") * 1024 * 1024 ))
    max_files=$(macmon_cfg "LOG_MAX_FILES" "5")

    [[ -f "$log_file" ]] || return 0
    local size
    size=$(stat -f%z "$log_file" 2>/dev/null || echo 0)
    if (( size > max_size_bytes )); then
        # Rotate: .5 → delete, .4 → .5, ... .1 → .2, current → .1
        local i
        for (( i = max_files; i > 1; i-- )); do
            local prev=$(( i - 1 ))
            [[ -f "${log_file}.${prev}" ]] && mv -f "${log_file}.${prev}" "${log_file}.${i}"
        done
        mv -f "$log_file" "${log_file}.1"
        touch "$log_file"
        macmon_log "Log rotated (was ${size} bytes)"
    fi
}

# --- AppleScript Security ---

# Sanitize a string for safe interpolation in AppleScript double-quoted strings
# Escapes backslashes, double quotes, and strips control characters
_applescript_escape() {
    local input="$1"
    # Strip control characters except newline and tab
    input=$(printf '%s' "$input" | tr -d '\000-\010\013\014\016-\037')
    # Escape backslashes first, then double quotes
    input="${input//\\/\\\\}"
    input="${input//\"/\\\"}"
    printf '%s' "$input"
}

# --- Notifications & Dialogs ---

macmon_notify() {
    local title body
    title=$(_applescript_escape "${1:-macmon}")
    body=$(_applescript_escape "${2:-}")
    osascript <<EOF
display notification "$body" with title "$title"
EOF
}

macmon_ask_yes_no() {
    local title body timeout
    title=$(_applescript_escape "${1:-macmon}")
    body=$(_applescript_escape "${2:-Continue?}")
    timeout="${3:-30}"
    # Validate timeout is numeric to prevent AppleScript injection
    [[ "$timeout" =~ ^[0-9]+$ ]] || timeout=30
    local result
    result=$(osascript <<EOF 2>/dev/null || echo "No"
display dialog "$body" with title "$title" buttons {"No", "Yes"} default button "No" giving up after $timeout
set theButton to button returned of result
return theButton
EOF
    )
    [[ "$result" == "Yes" ]]
}

# --- Process Name Resolution ---

friendly_name() {
    local comm="$1"
    local args="${2:-}"
    local base
    base=$(basename "$comm")

    # Warp terminal (binary is called "stable")
    if [[ "$base" == "stable" && "$args" == *"Warp.app"* ]]; then
        echo "Warp Terminal"
        return
    fi

    # Chrome family processes
    if [[ "$base" == "Google Chrome" || "$base" == "Google Chrome Helper"* || "$comm" == *"Google Chrome"* ]]; then
        if [[ "$args" == *"--type=renderer"* ]]; then
            echo "Chrome Tab"
        elif [[ "$args" == *"--type=gpu-process"* ]]; then
            echo "Chrome GPU"
        elif [[ "$args" == *"--type=utility"* ]]; then
            echo "Chrome Utility"
        elif [[ "$args" == *"--type="* ]]; then
            echo "Chrome Helper"
        else
            echo "Google Chrome"
        fi
        return
    fi

    # SourceKitService
    if [[ "$base" == "SourceKitService" ]]; then
        echo "SourceKitService"
        return
    fi

    # Gradle daemon
    if [[ "$base" == "java" && "$args" == *"GradleDaemon"* ]]; then
        echo "Gradle Daemon"
        return
    fi

    # Android emulator
    if [[ "$base" == "qemu-system-"* || "$base" == "qemu-system-x86_64" || "$base" == "qemu-system-aarch64" ]]; then
        echo "Android Emulator"
        return
    fi

    # xcodebuild
    if [[ "$base" == "xcodebuild" ]]; then
        echo "xcodebuild"
        return
    fi

    # .app bundle extraction
    if [[ "$comm" == *".app/"* ]]; then
        local app_name
        app_name=$(printf '%s' "$comm" | sed -n 's|.*/\([^/]*\)\.app/.*|\1|p')
        if [[ -n "$app_name" ]]; then
            echo "$app_name"
            return
        fi
    fi

    # Node.js processes - show script name
    if [[ "$base" == "node" && "$args" == *"/"* ]]; then
        local script
        script=$(printf '%s' "$args" | awk '{for(i=1;i<=NF;i++) if($i ~ /\.js$|\.mjs$|\.cjs$/) {print $i; exit}}')
        if [[ -n "$script" ]]; then
            echo "Node: $(basename "$script")"
            return
        fi
    fi

    echo "$base"
}

# --- Uptime Calculation ---

# Convert lstart timestamp to human-readable duration
calc_uptime() {
    local lstart="$1"
    local start_epoch now elapsed

    # lstart format: "Mon Jan  1 12:00:00 2026"
    start_epoch=$(date -jf "%a %b %d %T %Y" "$lstart" "+%s" 2>/dev/null) || return 1
    now=$(date "+%s")
    elapsed=$(( now - start_epoch ))

    if (( elapsed < 0 )); then
        echo "0m"
        return
    fi

    local days hours minutes
    days=$(( elapsed / 86400 ))
    hours=$(( (elapsed % 86400) / 3600 ))
    minutes=$(( (elapsed % 3600) / 60 ))

    if (( days > 0 )); then
        echo "${days}d ${hours}h"
    elif (( hours > 0 )); then
        echo "${hours}h ${minutes}m"
    else
        echo "${minutes}m"
    fi
}

# Convert lstart to epoch seconds (for numeric sorting)
uptime_seconds() {
    local lstart="$1"
    local start_epoch now
    start_epoch=$(date -jf "%a %b %d %T %Y" "$lstart" "+%s" 2>/dev/null) || { echo 0; return; }
    now=$(date "+%s")
    echo $(( now - start_epoch ))
}

# --- Code Signature Verification ---

# Verify a process binary is genuinely Apple-signed
# Used to prevent system process name spoofing for kill-immunity
_verify_apple_signed() {
    local pid="$1"
    local comm
    comm=$(ps -p "$pid" -o comm= 2>/dev/null) || return 1
    [[ -n "$comm" ]] || return 1
    codesign --verify --verbose=0 -R='anchor apple' "$comm" 2>/dev/null
}

_is_apple_system_pid() {
    local pid="$1"
    local comm
    comm=$(ps -p "$pid" -o comm= 2>/dev/null || true)
    [[ -n "$comm" ]] || return 1
    case "$comm" in
        /System/*|/usr/libexec/*|/usr/sbin/*)
            _verify_apple_signed "$pid"
            ;;
        *) return 1 ;;
    esac
}

# --- System Process Protection ---

# Check if a process name is a protected system process
is_system_process() {
    local name="$1"
    local protected_list
    protected_list=$(macmon_cfg "PROTECTED" "launchd:kernel_task:WindowServer:AudioComponentRegistrar:coremediaiod:loginwindow:coreaudiod:VTDecoderXPCService:VTEncoderXPCService:bluetoothd:fseventsd:mds:mds_stores:opendirectoryd:syslogd:configd:diskarbitrationd:powerd:thermalmonitord:UserEventAgent:cfprefsd:distnoted:logd:notifyd")

    if declare -F macmon_is_blocked_process >/dev/null 2>&1; then
        if macmon_is_blocked_process "$name"; then
            return 0
        fi
    fi

    local IFS=':'
    local proc
    for proc in $protected_list; do
        [[ "$name" == "$proc" ]] && return 0
    done
    return 1
}

# --- PID Verification ---

# Verify a PID still belongs to the expected process before killing
# Uses exact basename match to prevent name spoofing (Finding #6)
verify_pid() {
    local pid="$1"
    local expected_name="$2"
    local current_comm
    current_comm=$(ps -p "$pid" -o comm= 2>/dev/null) || return 1
    local current_base
    current_base=$(basename "$current_comm")
    local expected_base
    expected_base=$(basename "$expected_name")
    # Exact match on basenames
    [[ "$current_base" == "$expected_base" ]]
}

# --- Process Collection (Batch) ---

# Collect process data as JSON using batched ps calls
# Outputs JSON to stdout
collect_processes_json() {
    local min_rss_kb="${1:-102400}"
    local idle_cpu="${2:-1.0}"
    [[ "$idle_cpu" =~ ^[0-9]+\.?[0-9]*$ ]] || idle_cpu="1.0"

    # Phase 1: Single bulk ps call — get all qualifying processes
    local ps_data
    ps_data=$(ps -Aww -o pid=,rss=,pcpu=,state=,tty=,lstart=,comm=,args= 2>/dev/null) || return 1

    # Separate comm map preserves full command names that may contain spaces
    local ps_comm_data
    ps_comm_data=$(ps -Aww -o pid=,comm= 2>/dev/null) || return 1
    local -a comm_pids=() comm_values=()
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        if [[ "$line" =~ ^[[:space:]]*([0-9]+)[[:space:]]+(.+)$ ]]; then
            comm_pids+=("${BASH_REMATCH[1]}")
            comm_values+=("${BASH_REMATCH[2]}")
        fi
    done <<< "$ps_comm_data"

    # Phase 2: Filter and collect PIDs that meet RAM threshold
    local -a pids=() rss_arr=() cpu_arr=() state_arr=() tty_arr=() lstart_arr=() comm_arr=() args_arr=()
    local line pid rss cpu state tty lstart comm full_args

    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        # Parse fixed-width ps output
        read -r pid rss cpu state tty <<< "$(echo "$line" | awk '{print $1, $2, $3, $4, $5}')"

        # Skip non-numeric or too-small
        [[ "$pid" =~ ^[0-9]+$ ]] || continue
        [[ "$rss" =~ ^[0-9]+$ ]] || continue
        [[ "$cpu" =~ ^[0-9]+\.?[0-9]*$ ]] || continue
        (( rss >= min_rss_kb )) || continue

        # Extract lstart (5 fields: day month date time year)
        lstart=$(echo "$line" | awk '{print $6, $7, $8, $9, $10}')
        # comm starts at field 11, args includes comm and everything after
        comm=$(echo "$line" | awk '{print $11}')
        full_args=$(echo "$line" | awk '{for(i=11;i<=NF;i++) printf "%s ", $i; print ""}' | sed 's/ $//')

        pids+=("$pid")
        rss_arr+=("$rss")
        cpu_arr+=("$cpu")
        state_arr+=("$state")
        tty_arr+=("$tty")
        lstart_arr+=("$lstart")
        comm_arr+=("$comm")
        args_arr+=("$full_args")
    done <<< "$ps_data"

    local count=${#pids[@]}
    (( count > 0 )) || { echo '{"processes":[],"system":{}}'; return 0; }

    # Optional Chrome tab metadata (title/url), best-effort and cached per snapshot
    local -a chrome_ids=() chrome_titles=() chrome_urls=()
    if [[ "${MACMON_DISABLE_CHROME_TABS:-0}" != "1" ]] && printf '%s\n' "${args_arr[@]}" | grep -q -- '--renderer-client-id='; then
        local chrome_json
        chrome_json=$("${MACMON_HOME}/scripts/chrome-tabs.sh" --json 2>/dev/null || echo '[]')
        while IFS=$'\t' read -r cid ctitle curl; do
            [[ -n "$cid" ]] || continue
            chrome_ids+=("$cid")
            chrome_titles+=("$ctitle")
            chrome_urls+=("$curl")
        done < <(printf '%s' "$chrome_json" | jq -r '.[] | [.id, (.title // ""), (.url // "")] | @tsv' 2>/dev/null)
    fi

    # Phase 3: Batch lsof for working directories (single call for all PIDs)
    local pid_list
    pid_list=$(IFS=,; echo "${pids[*]}")
    local -a cwd_pids=() cwd_values=()
    local lsof_limit
    lsof_limit=$(macmon_cfg "COLLECT_BATCH_LSOF_LIMIT" "50")
    if [[ "${MACMON_DISABLE_LSOF:-0}" != "1" ]] && (( count <= lsof_limit )); then
        while IFS= read -r line; do
            local lpid lcwd
            lpid=$(echo "$line" | awk -F'\t' '{print $1}')
            lcwd=$(echo "$line" | awk -F'\t' '{print $2}')
            if [[ -n "$lpid" && -n "$lcwd" ]]; then
                cwd_pids+=("$lpid")
                cwd_values+=("$lcwd")
            fi
        done < <(lsof -a -d cwd -Fn -p "$pid_list" 2>/dev/null | awk '/^p/{pid=substr($0,2)} /^n/{print pid"\t"substr($0,2)}')
    fi

    # Phase 4: Build JSON using jq
    local json_array="[]"
    local i
    for (( i = 0; i < count; i++ )); do
        pid="${pids[$i]}"
        rss="${rss_arr[$i]}"
        cpu="${cpu_arr[$i]}"
        local exec_comm="${comm_arr[$i]}"
        local j
        for (( j = 0; j < ${#comm_pids[@]}; j++ )); do
            if [[ "${comm_pids[$j]}" == "$pid" ]]; then
                exec_comm="${comm_values[$j]}"
                break
            fi
        done
        local exec_name
        exec_name=$(basename "$exec_comm")
        local name
        name=$(friendly_name "$exec_comm" "${args_arr[$i]}")
        local ram_mb
        ram_mb=$(awk "BEGIN {printf \"%.1f\", ${rss}/1024}")
        local uptime_str uptime_sec
        uptime_str=$(calc_uptime "${lstart_arr[$i]}" 2>/dev/null || echo "?")
        uptime_sec=$(uptime_seconds "${lstart_arr[$i]}" 2>/dev/null || echo "0")
        local cwd=""
        j=0
        for (( j = 0; j < ${#cwd_pids[@]}; j++ )); do
            if [[ "${cwd_pids[$j]}" == "$pid" ]]; then
                cwd="${cwd_values[$j]}"
                break
            fi
        done
        local is_idle="false"
        if awk "BEGIN {exit !(${cpu} < ${idle_cpu})}"; then
            is_idle="true"
        fi

        # Determine detail string
        local detail=""
        case "$name" in
            "Chrome Tab")
                local tab_id
                tab_id=$(printf '%s' "${args_arr[$i]}" | grep -o '\-\-renderer-client-id=[0-9]*' | head -1 | cut -d= -f2)
                local tab_title=""
                local tab_url=""
                local tab_domain=""
                if [[ -n "$tab_id" && ${#chrome_ids[@]} -gt 0 ]]; then
                    local k
                    for (( k = 0; k < ${#chrome_ids[@]}; k++ )); do
                        if [[ "${chrome_ids[$k]}" == "$tab_id" ]]; then
                            tab_title="${chrome_titles[$k]}"
                            tab_url="${chrome_urls[$k]}"
                            break
                        fi
                    done
                fi
                if [[ -n "$tab_url" ]]; then
                    tab_domain=$(printf '%s' "$tab_url" | sed -E 's#^[a-zA-Z]+://([^/]+).*#\1#' | sed 's/^www\.//')
                fi
                if [[ -n "$tab_title" ]]; then
                    if [[ -n "$tab_domain" ]]; then
                        detail="$tab_title [$tab_domain]"
                    else
                        detail="$tab_title"
                    fi
                    if [[ -n "$tab_url" ]]; then
                        cwd="$tab_url"
                    fi
                else
                    detail="Tab ID: ${tab_id:-unknown}"
                fi
                ;;
            *)
                if [[ "${tty_arr[$i]}" != "??" && "${tty_arr[$i]}" != "-" ]]; then
                    detail="Terminal: ${tty_arr[$i]}"
                fi
                ;;
        esac

        # Determine process group
        local group=""
        if [[ "$exec_comm" == *".app/"* ]]; then
            group=$(printf '%s' "$exec_comm" | sed -n 's|.*/\([^/]*\)\.app/.*|\1|p')
        fi
        if [[ "$name" == Chrome* ]]; then
            if [[ -n "${tab_domain:-}" ]]; then
                group="Chrome: ${tab_domain}"
            else
                group="Google Chrome"
            fi
        fi

        # Detect system process (with signature verification)
        local is_system="false"
        if is_system_process "$exec_name"; then
            if _verify_apple_signed "$pid"; then
                is_system="true"
            else
                macmon_log "WARNING: Process '$exec_name' (PID $pid) claims system name but is NOT Apple-signed"
            fi
        fi

        # Get thread count from proc info (cached from initial ps)
        # We'll add this via a supplementary ps call below

        # Build entry with jq (safe JSON construction)
        json_array=$(printf '%s' "$json_array" | jq \
            --argjson pid "$pid" \
            --arg name "$name" \
            --arg execName "$exec_name" \
            --argjson ramMB "$ram_mb" \
            --argjson cpuPct "$cpu" \
            --arg uptime "$uptime_str" \
            --argjson uptimeSeconds "$uptime_sec" \
            --arg cwd "$cwd" \
            --arg tty "${tty_arr[$i]}" \
            --argjson idle "$is_idle" \
            --arg detail "$detail" \
            --arg group "$group" \
            --argjson isSystem "$is_system" \
            --arg state "${state_arr[$i]}" \
            '. + [{
                pid: $pid,
                name: $name,
                execName: $execName,
                ramMB: $ramMB,
                cpuPct: $cpuPct,
                uptime: $uptime,
                uptimeSeconds: $uptimeSeconds,
                cwd: $cwd,
                tty: $tty,
                idle: $idle,
                detail: $detail,
                group: $group,
                isSystem: $isSystem,
                state: $state
            }]')
    done

    # Phase 4b: Collect disk I/O via Swift helper (optional)
    local disk_io_enabled
    disk_io_enabled=$(macmon_cfg "COLLECT_DISK_IO" "true")
    local disk_io_helper="${MACMON_HOME}/DiskIOHelper"
    if [[ "${MACMON_DISABLE_DISK_IO:-0}" != "1" && "$disk_io_enabled" == "true" && -x "$disk_io_helper" ]]; then
        local disk_io_json
        disk_io_json=$(printf '%s\n' "${pids[@]}" | "$disk_io_helper" --stdin 2>/dev/null || echo "[]")
        # Merge disk I/O data into process array
        json_array=$(printf '%s' "$json_array" | jq --argjson dio "$disk_io_json" '
            . as $procs |
            ($dio | map({(.pid | tostring): .}) | add // {}) as $dioMap |
            $procs | map(
                . + {
                    diskReadMB: ($dioMap[(.pid | tostring)].diskReadMB // 0),
                    diskWriteMB: ($dioMap[(.pid | tostring)].diskWriteMB // 0)
                }
            )
        ')
    else
        # Add zero disk I/O fields
        json_array=$(printf '%s' "$json_array" | jq 'map(. + {diskReadMB: 0, diskWriteMB: 0})')
    fi

    # Phase 5: Collect system health data (uses cached memory_pressure)
    local free_pct
    free_pct=$(get_free_ram_percent)

    local swap_info
    swap_info=$(sysctl -n vm.swapusage 2>/dev/null || echo "")
    local swap_used_mb=0
    if [[ "$swap_info" =~ used[[:space:]]*=[[:space:]]*([0-9]+(\.[0-9]+)?)M ]]; then
        swap_used_mb="${BASH_REMATCH[1]}"
    fi

    local total_procs
    total_procs=$(ps -Ae -o pid= 2>/dev/null | wc -l | tr -d ' ')

    local idle_count=0
    for (( i = 0; i < count; i++ )); do
        if awk "BEGIN {exit !(${cpu_arr[$i]} < ${idle_cpu})}"; then
            (( idle_count++ )) || true
        fi
    done

    # Physical RAM
    local phys_mem_bytes phys_mem_gb
    phys_mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
    phys_mem_gb=$(awk "BEGIN {printf \"%.1f\", ${phys_mem_bytes}/1073741824}")

    # Wrap in final structure
    jq -n \
        --argjson processes "$json_array" \
        --argjson freePercent "$free_pct" \
        --argjson swapUsedMB "${swap_used_mb%.*}" \
        --argjson totalProcesses "$total_procs" \
        --argjson idleCount "$idle_count" \
        --argjson monitoredCount "$count" \
        --argjson physMemGB "$phys_mem_gb" \
        '{
            processes: $processes,
            system: {
                freePercent: $freePercent,
                swapUsedMB: $swapUsedMB,
                totalProcesses: $totalProcesses,
                idleCount: $idleCount,
                monitoredCount: $monitoredCount,
                physMemGB: $physMemGB
            }
        }'
}

# --- Process Killing ---

kill_processes() {
    local json_file="$1"
    local -a pids_to_kill=()
    local -a names_to_kill=()
    local -a urls_to_close=()

    # Read PIDs and names from JSON (format: [{"pid":123,"name":"Foo"}, ...])
    while IFS=$'\t' read -r pid name url; do
        [[ -n "$pid" ]] || continue
        pids_to_kill+=("$pid")
        names_to_kill+=("$name")
        urls_to_close+=("$url")
    done < <(jq -r '.[] | [.pid, .name, (.url // "")] | @tsv' "$json_file" 2>/dev/null)

    local idx pid name
    for (( idx = 0; idx < ${#pids_to_kill[@]}; idx++ )); do
        pid="${pids_to_kill[$idx]}"
        name="${names_to_kill[$idx]:-unknown}"
        local target_url
        target_url="${urls_to_close[$idx]:-}"

        # Skip system processes (verified Apple-signed)
        if is_system_process "$(basename "$name")"; then
            if _verify_apple_signed "$pid"; then
                macmon_log "BLOCKED: refusing to kill system process $name (PID $pid)"
                continue
            else
                macmon_log "WARNING: PID $pid uses system name '$name' but is not Apple-signed, allowing kill"
            fi
        fi

        if _is_apple_system_pid "$pid"; then
            macmon_log "BLOCKED: refusing to kill Apple system binary (PID $pid)"
            continue
        fi

        # Chrome tabs: close via AppleScript instead of kill
        if [[ "$name" == "Chrome Tab" ]]; then
            local current_args
            current_args=$(ps -p "$pid" -o args= 2>/dev/null || true)
            if [[ "$current_args" != *"--renderer-client-id="* ]]; then
                macmon_log "SKIP: PID $pid is not a Chrome tab renderer anymore"
                continue
            fi
            macmon_log "Attempting graceful Chrome tab close for PID $pid"
            if ! "${MACMON_HOME}/scripts/graceful-quit.sh" chrome-tab "$pid" "$target_url"; then
                macmon_log "Chrome tab close failed for PID $pid; sending SIGTERM fallback"
                kill -TERM "$pid" 2>/dev/null || true
            fi
            continue
        fi

        # Verify PID still matches expected process
        if ! verify_pid "$pid" "$name"; then
            macmon_log "SKIP: PID $pid no longer matches '$name' (PID reuse detected)"
            continue
        fi

        # .app processes: try graceful quit first
        if [[ "$name" != *"/"* && "$name" != "node"* ]]; then
            "${MACMON_HOME}/scripts/graceful-quit.sh" app "$name" &
            continue
        fi

        # Default: SIGTERM
        macmon_log "Sending SIGTERM to $name (PID $pid)"
        kill -TERM "$pid" 2>/dev/null || true
    done

    # Wait for graceful shutdown
    local grace
    grace=$(macmon_cfg "INTERVALS_KILL_GRACE" "3")
    [[ "$grace" =~ ^[0-9]+$ ]] || grace=3
    sleep "$grace"

    # SIGKILL stragglers (skip Chrome tabs and .app processes already handled)
    for (( idx = 0; idx < ${#pids_to_kill[@]}; idx++ )); do
        pid="${pids_to_kill[$idx]}"
        name="${names_to_kill[$idx]:-unknown}"
        [[ "$name" == "Chrome Tab" ]] && continue
        if kill -0 "$pid" 2>/dev/null; then
            # Re-apply strict protection checks before SIGKILL
            if is_system_process "$(basename "$name")"; then
                if _verify_apple_signed "$pid"; then
                    macmon_log "BLOCKED: refusing SIGKILL for system process $name (PID $pid)"
                    continue
                else
                    macmon_log "WARNING: PID $pid uses system name '$name' but is not Apple-signed, allowing SIGKILL"
                fi
            fi

            if _is_apple_system_pid "$pid"; then
                macmon_log "BLOCKED: refusing SIGKILL for Apple system binary (PID $pid)"
                continue
            fi

            if verify_pid "$pid" "$name"; then
                macmon_log "Sending SIGKILL to $name (PID $pid)"
                kill -KILL "$pid" 2>/dev/null || true
            fi
        fi
    done
}

# Convert picker output to kill payload JSON.
# Supports current JSON array format and legacy newline PID format.
build_kill_payload_json() {
    local selection="$1"
    local output_file="$2"
    [[ -n "$output_file" ]] || return 1

    if [[ -z "$selection" ]]; then
        printf '[]\n' > "$output_file"
        return 0
    fi

    if printf '%s' "$selection" | jq -e . >/dev/null 2>&1; then
        printf '%s' "$selection" | jq -c '
            if type == "array" then
                map(
                    select((.pid | type) == "number" and (.pid > 1)) |
                    {
                        pid: .pid,
                        name: ((.name // "") | tostring),
                        url: ((.url // "") | tostring)
                    }
                )
            else
                []
            end
        ' > "$output_file"
    else
        printf '%s' "$selection" | jq -R -s '
            split("\n") | map(select(length > 0)) |
            map(select(test("^[0-9]+$"))) |
            map({pid: (. | tonumber), name: "unknown", url: ""})
        ' > "$output_file"
    fi
}

# --- Dynamic Process Monitoring ---
# Technology-agnostic: reads custom_processes from YAML config
# Each entry can define: max_instances, max_ram_mb, max_cpu_percent

# Check all custom_processes for threshold violations.
# Returns violations as lines: "name:violation_type:current_value:threshold"
check_custom_processes() {
    local violations=""

    while IFS=: read -r proc_name max_inst max_ram max_cpu; do
        [[ -z "$proc_name" ]] && continue

        local pids pids_csv count
        pids=$(pgrep -x "$proc_name" 2>/dev/null || true)
        count=0
        if [[ -n "$pids" ]]; then
            count=$(printf '%s\n' "$pids" | wc -l | tr -d ' ')
        fi

        # Check max_instances
        if (( max_inst > 0 )); then
            if (( count > max_inst )); then
                violations="${violations}${proc_name}:instances:${count}:${max_inst}"$'\n'
                macmon_log "Custom process alert: $proc_name has $count instances (max: $max_inst)"
            fi
        fi

        if [[ -n "$pids" && ( $max_ram -gt 0 || $max_cpu -gt 0 ) ]]; then
            pids_csv=$(printf '%s\n' "$pids" | paste -sd, -)
            local total_rss_kb=0 highest_cpu=0 line rss_val cpu_val
            while IFS= read -r line; do
                [[ -n "$line" ]] || continue
                read -r rss_val cpu_val <<< "$line"
                [[ "$rss_val" =~ ^[0-9]+$ ]] && (( total_rss_kb += rss_val )) || true
                [[ "$cpu_val" =~ ^[0-9]+\.?[0-9]*$ ]] || continue
                if awk "BEGIN {exit !(${cpu_val} > ${highest_cpu})}"; then
                    highest_cpu="$cpu_val"
                fi
            done < <(ps -p "$pids_csv" -o rss=,pcpu= 2>/dev/null || true)

            # Check max_ram_mb (sum of all instances)
            if (( max_ram > 0 )); then
                local total_ram_mb=$(( total_rss_kb / 1024 ))
                if (( total_ram_mb > max_ram )); then
                    violations="${violations}${proc_name}:ram:${total_ram_mb}:${max_ram}"$'\n'
                    macmon_log "Custom process alert: $proc_name using ${total_ram_mb}MB RAM (max: ${max_ram}MB)"
                fi
            fi

            # Check max_cpu_percent (highest single instance)
            if (( max_cpu > 0 )); then
                local highest_int
                highest_int=$(printf '%.0f' "$highest_cpu")
                if (( highest_int > max_cpu )); then
                    violations="${violations}${proc_name}:cpu:${highest_int}:${max_cpu}"$'\n'
                    macmon_log "Custom process alert: $proc_name at ${highest_int}% CPU (max: ${max_cpu}%)"
                fi
            fi
        fi
    done < <(macmon_get_custom_processes)

    violations="${violations%$'\n'}"
    if [[ -n "$violations" ]]; then
        printf '%s\n' "$violations"
        return 0
    fi
    return 1
}

# Kill all instances of a named process (SIGTERM then SIGKILL)
kill_process_by_name() {
    local proc_name="$1"
    local pids
    pids=$(pgrep -x "$proc_name" 2>/dev/null || true)
    [[ -z "$pids" ]] && return 0

    macmon_log "Killing $proc_name processes"
    local pid
    while IFS= read -r pid; do
        [[ -n "$pid" ]] || continue
        if macmon_is_blocked_process "$proc_name" || is_system_process "$proc_name"; then
            macmon_log "BLOCKED: refusing to kill system process $proc_name (PID $pid)"
            continue
        fi
        if _is_apple_system_pid "$pid"; then
            macmon_log "BLOCKED: refusing to kill Apple system binary (PID $pid)"
            continue
        fi
        kill -TERM "$pid" 2>/dev/null || true
    done <<< "$pids"

    sleep 2

    # SIGKILL stragglers — re-apply blocklist checks before escalation
    pids=$(pgrep -x "$proc_name" 2>/dev/null || true)
    while IFS= read -r pid; do
        [[ -n "$pid" ]] || continue
        if macmon_is_blocked_process "$proc_name" || is_system_process "$proc_name"; then
            macmon_log "BLOCKED: refusing SIGKILL for system process $proc_name (PID $pid)"
            continue
        fi
        if _is_apple_system_pid "$pid"; then
            macmon_log "BLOCKED: refusing SIGKILL for Apple system binary (PID $pid)"
            continue
        fi
        kill -KILL "$pid" 2>/dev/null || true
    done <<< "$pids"
}

# --- Orphan Build Daemon Detection ---

# Generalized orphan daemon checker
# Checks all configured orphan_daemons and returns info about detected orphans
check_orphan_daemons() {
    local -a results=()

    # SourceKitService: flag if running without Xcode
    local sk_count
    sk_count=$(pgrep -x SourceKitService 2>/dev/null | wc -l | tr -d ' ') || true
    if (( sk_count > 0 )) && ! pgrep -x Xcode >/dev/null 2>&1; then
        results+=("SourceKitService:$sk_count:orphan (Xcode not running)")
    fi

    # Gradle daemons: flag if >3
    local gradle_count
    gradle_count=$(pgrep -f 'GradleDaemon' 2>/dev/null | wc -l | tr -d ' ') || true
    if (( gradle_count > 3 )); then
        results+=("GradleDaemon:$gradle_count:excessive count")
    fi

    # xcodebuild: flag if running without Xcode
    local xb_count
    xb_count=$(pgrep -x xcodebuild 2>/dev/null | wc -l | tr -d ' ') || true
    if (( xb_count > 0 )) && ! pgrep -x Xcode >/dev/null 2>&1; then
        results+=("xcodebuild:$xb_count:orphan (Xcode not running)")
    fi

    # Android emulator (qemu-system): flag if >2 instances
    local qemu_count
    qemu_count=$(pgrep -f 'qemu-system' 2>/dev/null | wc -l | tr -d ' ') || true
    if (( qemu_count > 2 )); then
        results+=("qemu-system:$qemu_count:excessive count")
    fi

    if (( ${#results[@]} > 0 )); then
        printf '%s\n' "${results[@]}"
        return 0  # orphans detected
    fi
    return 1  # no orphans
}

# Kill orphan processes by name pattern
kill_orphan_by_pattern() {
    local pattern="$1"
    local pids
    pids=$(pgrep -f "$pattern" 2>/dev/null || true)
    [[ -z "$pids" ]] && return 0

    macmon_log "Killing orphan $pattern processes"
    local pid
    while IFS= read -r pid; do
        [[ -n "$pid" ]] || continue
        if is_system_process "$(ps -p "$pid" -o comm= 2>/dev/null | xargs basename 2>/dev/null)"; then
            continue
        fi
        kill -TERM "$pid" 2>/dev/null || true
    done <<< "$pids"

    sleep 2

    # SIGKILL stragglers — re-apply blocklist checks before escalation
    pids=$(pgrep -f "$pattern" 2>/dev/null || true)
    while IFS= read -r pid; do
        [[ -n "$pid" ]] || continue
        local proc_comm
        proc_comm=$(ps -p "$pid" -o comm= 2>/dev/null | xargs basename 2>/dev/null || true)
        if [[ -n "$proc_comm" ]] && is_system_process "$proc_comm"; then
            macmon_log "BLOCKED: refusing SIGKILL for system process $proc_comm (PID $pid)"
            continue
        fi
        if _is_apple_system_pid "$pid"; then
            macmon_log "BLOCKED: refusing SIGKILL for Apple system binary (PID $pid)"
            continue
        fi
        kill -KILL "$pid" 2>/dev/null || true
    done <<< "$pids"
}

# --- Swift Picker Management ---

ensure_picker_compiled() {
    local swift_src="${MACMON_HOME}/src/gui/ProcessPicker.swift"
    local swift_model_src="${MACMON_HOME}/src/gui/ProcessPickerModel.swift"
    local swift_i18n_src="${MACMON_HOME}/src/gui/Localization.swift"
    local swift_ai_src="${MACMON_HOME}/src/gui/AIService.swift"
    local swift_prefs_src="${MACMON_HOME}/src/gui/PreferencesWindow.swift"
    local swift_telemetry_src="${MACMON_HOME}/src/gui/TelemetryRecorder.swift"
    local swift_killer_src="${MACMON_HOME}/src/gui/ProcessKiller.swift"
    local binary="${MACMON_HOME}/ProcessPicker"

    if [[ ! -f "$swift_src" || ! -f "$swift_model_src" || ! -f "$swift_i18n_src" || ! -f "$swift_ai_src" ]]; then
        macmon_log "ERROR: Swift source not found at $swift_src"
        return 1
    fi

    # Compile if binary missing or source is newer
    if [[ ! -f "$binary" || "$swift_src" -nt "$binary" || "$swift_model_src" -nt "$binary" || "$swift_i18n_src" -nt "$binary" || "$swift_ai_src" -nt "$binary" || "$swift_prefs_src" -nt "$binary" || "$swift_telemetry_src" -nt "$binary" || "$swift_killer_src" -nt "$binary" ]]; then
        macmon_log "Compiling ProcessPicker (universal)..."
        if swiftc -O -target arm64-apple-macos13 -framework Cocoa -o "${binary}-arm64" "$swift_model_src" "$swift_i18n_src" "$swift_ai_src" "$swift_prefs_src" "$swift_telemetry_src" "$swift_killer_src" "$swift_src" 2>&1 \
           && swiftc -O -target x86_64-apple-macos13 -framework Cocoa -o "${binary}-x86_64" "$swift_model_src" "$swift_i18n_src" "$swift_ai_src" "$swift_prefs_src" "$swift_telemetry_src" "$swift_killer_src" "$swift_src" 2>&1 \
           && lipo -create -output "$binary" "${binary}-arm64" "${binary}-x86_64" 2>&1; then
            rm -f "${binary}-arm64" "${binary}-x86_64"
            macmon_log "ProcessPicker compiled successfully"
        else
            rm -f "${binary}-arm64" "${binary}-x86_64"
            macmon_log "ERROR: Failed to compile ProcessPicker"
            return 1
        fi
    fi
    return 0
}

# Show the process picker UI and return selected PIDs
# Usage: show_process_picker [--standalone]
show_process_picker() {
    local standalone_flag=""
    if [[ "${1:-}" == "--standalone" ]]; then
        standalone_flag="--standalone"
    fi

    ensure_picker_compiled || return 1

    local json_file
    json_file=$(mktemp "${MACMON_TMPDIR}/macmon-procs.XXXXXX.json")

    # Collect process data
    collect_processes_json \
        "$(macmon_cfg "THRESHOLDS_PROCESS_RAM_MIN_KB" "102400")" \
        "$(macmon_cfg "THRESHOLDS_IDLE_CPU_PERCENT" "1.0")" \
        > "$json_file"

    local proc_count
    proc_count=$(jq '.processes | length' "$json_file" 2>/dev/null || echo 0)

    if (( proc_count == 0 )); then
        rm -f "$json_file"
        macmon_log "No qualifying processes found"
        return 1
    fi

    # Launch picker
    local binary="${MACMON_HOME}/ProcessPicker"

    if [[ -n "$standalone_flag" ]]; then
        # Standalone mode: picker kills processes internally, no stdout capture
        "$binary" --file "$json_file" --standalone 2>/dev/null
        local exit_code=$?
        rm -f "$json_file"
        if (( exit_code == 2 )); then
            macmon_log "Picker cancelled by user"
        fi
        return "$exit_code"
    fi

    # Legacy mode: capture JSON output from picker
    local result
    result=$("$binary" --file "$json_file" 2>/dev/null) || {
        local exit_code=$?
        rm -f "$json_file"
        if (( exit_code == 2 )); then
            macmon_log "Picker cancelled by user"
        fi
        return "$exit_code"
    }

    rm -f "$json_file"

    # result contains selected PIDs, one per line
    if [[ -n "$result" ]]; then
        echo "$result"
        return 0
    fi
    return 1
}

# --- Memory Pressure Cache ---

_cached_mem_pressure=""
_cached_mem_pressure_time=0

get_memory_pressure() {
    local now
    now=$(date +%s)
    # Cache for 30 seconds
    if (( now - _cached_mem_pressure_time < 30 )) && [[ -n "$_cached_mem_pressure" ]]; then
        echo "$_cached_mem_pressure"
        return
    fi
    _cached_mem_pressure=$(memory_pressure 2>/dev/null | tail -1 || echo "")
    _cached_mem_pressure_time=$now
    echo "$_cached_mem_pressure"
}

get_free_ram_percent() {
    local output
    output=$(get_memory_pressure)
    if [[ "$output" =~ ([0-9]+)% ]]; then
        echo "${BASH_REMATCH[1]}"
    else
        echo "0"
    fi
}
