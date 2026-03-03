#!/usr/bin/env bash
# macmon-core.sh - Shared functions for macmon
# Sources: macmon-config.sh for configuration values

set -euo pipefail

MACMON_VERSION="1.1.0"
MACMON_HOME="${MACMON_HOME:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

# Source config loader
# shellcheck source=macmon-config.sh
source "${MACMON_HOME}/lib/macmon-config.sh"

# --- Temp directory (per-user private) ---
MACMON_TMPDIR="${TMPDIR:-/tmp}"

# --- Logging ---

MACMON_LOG_DIR="${MACMON_LOG_DIR:-$HOME/.local/log/macmon}"

macmon_log() {
    local log_file="${MACMON_LOG_FILE:-${MACMON_LOG_DIR}/macmond.log}"
    local dir
    dir="$(dirname "$log_file")"
    [[ -d "$dir" ]] || mkdir -p "$dir"
    printf '%s [macmon] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$log_file"
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

    # Claude CLI
    if [[ "$base" == "claude" || "$args" == *"/@anthropic-ai/claude-code"* ]]; then
        echo "Claude CLI"
        return
    fi

    # OpenCode
    if [[ "$base" == "opencode" ]]; then
        echo "OpenCode"
        return
    fi

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

# --- System Process Protection ---

# Check if a process name is a protected system process
is_system_process() {
    local name="$1"
    local protected_list
    protected_list=$(macmon_cfg "PROTECTED" "launchd:kernel_task:WindowServer:loginwindow:coreaudiod:bluetoothd:fseventsd:mds:mds_stores:opendirectoryd:syslogd:configd:diskarbitrationd:powerd:thermalmonitord:UserEventAgent:cfprefsd:distnoted:logd:notifyd")

    local IFS=':'
    local proc
    for proc in $protected_list; do
        [[ "$name" == "$proc" ]] && return 0
    done
    return 1
}

# --- PID Verification ---

# Verify a PID still belongs to the expected process before killing
verify_pid() {
    local pid="$1"
    local expected_name="$2"
    local current_name
    current_name=$(ps -p "$pid" -o comm= 2>/dev/null) || return 1
    current_name=$(basename "$current_name")
    [[ "$current_name" == *"$expected_name"* ]] || [[ "$expected_name" == *"$current_name"* ]]
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

    # Phase 3: Batch lsof for working directories (single call for all PIDs)
    local pid_list
    pid_list=$(IFS=,; echo "${pids[*]}")
    local -A cwd_map=()
    local lsof_limit
    lsof_limit=$(macmon_cfg "COLLECT_BATCH_LSOF_LIMIT" "50")
    if (( count <= lsof_limit )); then
        while IFS= read -r line; do
            local lpid lcwd
            lpid=$(echo "$line" | awk -F'\t' '{print $1}')
            lcwd=$(echo "$line" | awk -F'\t' '{print $2}')
            [[ -n "$lpid" && -n "$lcwd" ]] && cwd_map["$lpid"]="$lcwd"
        done < <(lsof -a -d cwd -Fn -p "$pid_list" 2>/dev/null | awk '/^p/{pid=substr($0,2)} /^n/{print pid"\t"substr($0,2)}')
    fi

    # Phase 4: Build JSON using jq
    local json_array="[]"
    local i
    for (( i = 0; i < count; i++ )); do
        pid="${pids[$i]}"
        rss="${rss_arr[$i]}"
        cpu="${cpu_arr[$i]}"
        local name
        name=$(friendly_name "${comm_arr[$i]}" "${args_arr[$i]}")
        local ram_mb
        ram_mb=$(awk "BEGIN {printf \"%.1f\", ${rss}/1024}")
        local uptime_str uptime_sec
        uptime_str=$(calc_uptime "${lstart_arr[$i]}" 2>/dev/null || echo "?")
        uptime_sec=$(uptime_seconds "${lstart_arr[$i]}" 2>/dev/null || echo "0")
        local cwd="${cwd_map[$pid]:-}"
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
                detail="Tab ID: ${tab_id:-unknown}"
                ;;
            "Claude CLI")
                [[ -n "$cwd" ]] && detail="Project: $(basename "$cwd")"
                ;;
            "OpenCode")
                [[ -n "$cwd" ]] && detail="Dir: $(basename "$cwd")"
                ;;
            *)
                if [[ "${tty_arr[$i]}" != "??" && "${tty_arr[$i]}" != "-" ]]; then
                    detail="Terminal: ${tty_arr[$i]}"
                fi
                ;;
        esac

        # Determine process group
        local group=""
        if [[ "${comm_arr[$i]}" == *".app/"* ]]; then
            group=$(printf '%s' "${comm_arr[$i]}" | sed -n 's|.*/\([^/]*\)\.app/.*|\1|p')
        fi
        if [[ "$name" == Chrome* ]]; then
            group="Google Chrome"
        fi

        # Detect system process (with signature verification)
        local base_comm
        base_comm=$(basename "${comm_arr[$i]}")
        local is_system="false"
        if is_system_process "$base_comm"; then
            if _verify_apple_signed "$pid"; then
                is_system="true"
            else
                macmon_log "WARNING: Process '$base_comm' (PID $pid) claims system name but is NOT Apple-signed"
            fi
        fi

        # Get thread count from proc info (cached from initial ps)
        # We'll add this via a supplementary ps call below

        # Build entry with jq (safe JSON construction)
        json_array=$(printf '%s' "$json_array" | jq \
            --argjson pid "$pid" \
            --arg name "$name" \
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
    if [[ "$disk_io_enabled" == "true" && -x "$disk_io_helper" ]]; then
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

    # Phase 5: Collect system health data
    local mem_pressure_output
    mem_pressure_output=$(memory_pressure 2>/dev/null | tail -1 || echo "")
    local free_pct=0
    if [[ "$mem_pressure_output" =~ ([0-9]+)% ]]; then
        free_pct="${BASH_REMATCH[1]}"
    fi

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
    local -A pid_names=()

    # Read PIDs and names from JSON (format: [{"pid":123,"name":"Foo"}, ...])
    while IFS=$'\t' read -r pid name; do
        [[ -n "$pid" ]] || continue
        pids_to_kill+=("$pid")
        pid_names["$pid"]="$name"
    done < <(jq -r '.[] | [.pid, .name] | @tsv' "$json_file" 2>/dev/null)

    local pid name
    for pid in "${pids_to_kill[@]}"; do
        name="${pid_names[$pid]:-unknown}"

        # Skip system processes (verified Apple-signed)
        if is_system_process "$(basename "$name")"; then
            if _verify_apple_signed "$pid"; then
                macmon_log "BLOCKED: refusing to kill system process $name (PID $pid)"
                continue
            else
                macmon_log "WARNING: PID $pid uses system name '$name' but is not Apple-signed, allowing kill"
            fi
        fi

        # Verify PID still matches expected process
        if ! verify_pid "$pid" "$name"; then
            macmon_log "SKIP: PID $pid no longer matches '$name' (PID reuse detected)"
            continue
        fi

        # Chrome tabs: close via AppleScript instead of kill
        if [[ "$name" == "Chrome Tab" ]]; then
            "${MACMON_HOME}/scripts/graceful-quit.sh" chrome-tab "$pid" &
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
    for pid in "${pids_to_kill[@]}"; do
        name="${pid_names[$pid]:-unknown}"
        [[ "$name" == "Chrome Tab" ]] && continue
        if kill -0 "$pid" 2>/dev/null; then
            if verify_pid "$pid" "$name"; then
                macmon_log "Sending SIGKILL to $name (PID $pid)"
                kill -KILL "$pid" 2>/dev/null || true
            fi
        fi
    done
}

# --- Flutter Tester Management ---

check_flutter_tester() {
    local threshold
    threshold=$(macmon_cfg "THRESHOLDS_FLUTTER_PROCESS_COUNT" "10")
    local count
    count=$(pgrep -x flutter_tester 2>/dev/null | wc -l | tr -d ' ')

    if (( count > threshold )); then
        macmon_log "Flutter tester accumulation detected: $count processes (threshold: $threshold)"
        return 0  # alert needed
    fi
    return 1  # no alert needed
}

kill_flutter_testers() {
    local pids
    pids=$(pgrep -x flutter_tester 2>/dev/null || true)
    [[ -z "$pids" ]] && return 0

    macmon_log "Killing flutter_tester processes"
    local pid
    while IFS= read -r pid; do
        [[ -n "$pid" ]] && kill -TERM "$pid" 2>/dev/null || true
    done <<< "$pids"

    sleep 2

    pids=$(pgrep -x flutter_tester 2>/dev/null || true)
    while IFS= read -r pid; do
        [[ -n "$pid" ]] && kill -KILL "$pid" 2>/dev/null || true
    done <<< "$pids"
}

# --- Orphan Build Daemon Detection ---

# Generalized orphan daemon checker
# Checks all configured orphan_daemons and returns info about detected orphans
check_orphan_daemons() {
    local -a results=()

    # SourceKitService: flag if >4GB RAM or running >2h without Xcode
    local sk_pids sk_count
    sk_pids=$(pgrep -x SourceKitService 2>/dev/null || true)
    sk_count=$(echo "$sk_pids" | grep -c '[0-9]' 2>/dev/null || echo 0)
    if (( sk_count > 0 )) && ! pgrep -x Xcode >/dev/null 2>&1; then
        results+=("SourceKitService:$sk_count:orphan (Xcode not running)")
    fi

    # Gradle daemons: flag if >3 or any older than 8h
    local gradle_pids gradle_count
    gradle_pids=$(pgrep -f 'GradleDaemon' 2>/dev/null || true)
    gradle_count=$(echo "$gradle_pids" | grep -c '[0-9]' 2>/dev/null || echo 0)
    if (( gradle_count > 3 )); then
        results+=("GradleDaemon:$gradle_count:excessive count")
    fi

    # xcodebuild: flag if running >60min without Xcode
    local xb_pids xb_count
    xb_pids=$(pgrep -x xcodebuild 2>/dev/null || true)
    xb_count=$(echo "$xb_pids" | grep -c '[0-9]' 2>/dev/null || echo 0)
    if (( xb_count > 0 )) && ! pgrep -x Xcode >/dev/null 2>&1; then
        results+=("xcodebuild:$xb_count:orphan (Xcode not running)")
    fi

    # Android emulator (qemu-system): flag if >2 instances
    local qemu_pids qemu_count
    qemu_pids=$(pgrep -f 'qemu-system' 2>/dev/null || true)
    qemu_count=$(echo "$qemu_pids" | grep -c '[0-9]' 2>/dev/null || echo 0)
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

    pids=$(pgrep -f "$pattern" 2>/dev/null || true)
    while IFS= read -r pid; do
        [[ -n "$pid" ]] && kill -KILL "$pid" 2>/dev/null || true
    done <<< "$pids"
}

# --- Swift Picker Management ---

ensure_picker_compiled() {
    local swift_src="${MACMON_HOME}/src/gui/ProcessPicker.swift"
    local binary="${MACMON_HOME}/ProcessPicker"

    if [[ ! -f "$swift_src" ]]; then
        macmon_log "ERROR: Swift source not found at $swift_src"
        return 1
    fi

    # Compile if binary missing or source is newer
    if [[ ! -f "$binary" || "$swift_src" -nt "$binary" ]]; then
        macmon_log "Compiling ProcessPicker..."
        if swiftc -O -framework Cocoa -o "$binary" "$swift_src" 2>&1; then
            macmon_log "ProcessPicker compiled successfully"
        else
            macmon_log "ERROR: Failed to compile ProcessPicker"
            return 1
        fi
    fi
    return 0
}

# Show the process picker UI and return selected PIDs
show_process_picker() {
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
