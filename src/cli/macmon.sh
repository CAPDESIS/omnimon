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
MSG_OPT_COLLECTING="Collecting process data..."
MSG_OPT_ANALYZING="Asking AI to analyze processes..."
MSG_OPT_NO_KEY="No API key found in Keychain for provider"
MSG_OPT_HINT="Set one via the GUI: macmon → Smart Optimize → Preferences"
MSG_OPT_NO_SUGGESTIONS="AI returned no actionable suggestions."
MSG_OPT_CONFIRM="Proceed? [Y/n]"
MSG_OPT_CANCELLED="Aborted."
MSG_OPT_FAILED="Optimize failed"

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
            proc_count=$(pgrep -x "$proc_name" 2>/dev/null | wc -l | tr -d ' ') || true
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
    sk_count=$(pgrep -x SourceKitService 2>/dev/null | wc -l | tr -d ' ') || true
    gradle_count=$(pgrep -f GradleDaemon 2>/dev/null | wc -l | tr -d ' ') || true
    xb_count=$(pgrep -x xcodebuild 2>/dev/null | wc -l | tr -d ' ') || true
    qemu_count=$(pgrep -f qemu-system 2>/dev/null | wc -l | tr -d ' ') || true
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

    local tmpdir
    tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/macmon-update.XXXXXXXXXX")
    trap 'rm -rf "$tmpdir"' RETURN

    echo "$MSG_UPDATE_DOWNLOADING"
    curl -fSL -o "${tmpdir}/${archive_name}" "$asset_url" || {
        echo "$MSG_UPDATE_FAILED: download error"
        return 1
    }

    # Checksum verification
    local checksum_url
    checksum_url=$(printf '%s' "$release_json" | jq -r '
        [.assets[]? | select(.name == "checksums-sha256.txt") | .browser_download_url][0] // empty
    ')
    if [[ -z "$checksum_url" ]]; then
        echo "$MSG_UPDATE_FAILED: checksums-sha256.txt not found in release assets"
        return 1
    fi
    curl -fsSL -o "${tmpdir}/checksums-sha256.txt" "$checksum_url" || {
        echo "$MSG_UPDATE_FAILED: could not download checksum file"
        return 1
    }
    (cd "$tmpdir" && grep "$archive_name" checksums-sha256.txt | shasum -a 256 -c -) || {
        echo "$MSG_UPDATE_FAILED: checksum verification failed — archive may be corrupted"
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

cmd_optimize() {
    if ! command -v jq >/dev/null 2>&1; then
        echo "$MSG_OPT_FAILED: jq is required (brew install jq)"
        return 1
    fi

    local provider model
    provider=$(macmon_cfg "AI_PROVIDER" "openai")
    model=$(macmon_cfg "AI_MODEL" "gpt-4o-mini")

    # Read API key from macOS Keychain (same store used by GUI)
    local api_key
    api_key=$(security find-generic-password -s "com.macmon.ai" -a "$provider" -w 2>/dev/null) || {
        echo "$MSG_OPT_NO_KEY '$provider'"
        echo "$MSG_OPT_HINT"
        return 1
    }

    echo "$MSG_OPT_COLLECTING"
    local json
    json=$(collect_processes_json \
        "$(macmon_cfg "THRESHOLDS_PROCESS_RAM_MIN_KB" "102400")" \
        "$(macmon_cfg "THRESHOLDS_IDLE_CPU_PERCENT" "1.0")")

    # Top 50 non-system processes sorted by resource usage
    local summary
    summary=$(printf '%s' "$json" | jq '[.processes[] | select(.isSystem == false)] | sort_by(-(.cpuPct + .ramMB/1024)) | .[0:50] | map({pid, name, cpuPct, ramMB, isSystem})')

    local current_profile
    current_profile=$(macmon_get_active_profile 2>/dev/null || echo "default")

    local prompt
    prompt="You are a macOS optimization assistant. Analyze process data and return strict JSON only. Constraints: 1) Never output shell commands. 2) Never include Apple system processes or audio/video critical services. 3) Return only this shape: {\"suggestions\":[{\"pid\":123,\"reason\":\"short explanation\"}]} 4) Include only non critical user space PIDs. 5) Each reason must be a brief, human-readable explanation. Current profile: ${current_profile}. Processes: ${summary}"

    echo "$MSG_OPT_ANALYZING"

    # Build request and call API
    local response http_code body endpoint
    local header_file
    header_file=$(mktemp "${TMPDIR:-/tmp}/macmon-ai-headers.XXXXXX")
    chmod 600 "$header_file"
    trap 'rm -f "$header_file"' RETURN

    case "$provider" in
        openai)     endpoint="https://api.openai.com/v1/chat/completions" ;;
        openrouter) endpoint="https://openrouter.ai/api/v1/chat/completions" ;;
        anthropic)  endpoint="https://api.anthropic.com/v1/messages" ;;
        gemini)     endpoint="https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent" ;;
        *)          echo "$MSG_OPT_FAILED: unsupported provider '$provider'"; return 1 ;;
    esac

    local curl_args=(-sS --max-time 30 -w '\n%{http_code}' -H "Content-Type: application/json")

    case "$provider" in
        openai|openrouter)
            printf '%s\n' "Authorization: Bearer ${api_key}" > "$header_file"
            curl_args+=(-H "@${header_file}")
            curl_args+=(-d "$(jq -n --arg model "$model" --arg prompt "$prompt" '{
                model: $model, temperature: 0,
                messages: [{role:"system",content:"Return strict JSON only."},{role:"user",content:$prompt}]
            }')")
            ;;
        anthropic)
            printf '%s\n%s\n' "x-api-key: ${api_key}" "anthropic-version: 2023-06-01" > "$header_file"
            curl_args+=(-H "@${header_file}")
            curl_args+=(-d "$(jq -n --arg model "$model" --arg prompt "$prompt" '{
                model: $model, max_tokens: 512, temperature: 0,
                messages: [{role:"user",content:$prompt}]
            }')")
            ;;
        gemini)
            printf '%s\n' "x-goog-api-key: ${api_key}" > "$header_file"
            curl_args+=(-H "@${header_file}")
            curl_args+=(-d "$(jq -n --arg prompt "$prompt" '{
                generationConfig: {temperature:0, maxOutputTokens:512},
                contents: [{role:"user",parts:[{text:$prompt}]}]
            }')")
            ;;
    esac

    response=$(curl "${curl_args[@]}" "$endpoint" 2>/dev/null) || {
        echo "$MSG_OPT_FAILED: could not reach AI provider"
        return 1
    }

    # Split response body and HTTP status code
    http_code=$(printf '%s' "$response" | tail -1)
    body=$(printf '%s' "$response" | sed '$d')

    if [[ "$http_code" -ge 400 ]] 2>/dev/null; then
        local err_msg
        err_msg=$(printf '%s' "$body" | jq -r '.error.message // .message // "Unknown error"' 2>/dev/null || echo "HTTP $http_code")
        case "$http_code" in
            401) echo "$MSG_OPT_FAILED: Invalid API Key — $err_msg" ;;
            429) echo "$MSG_OPT_FAILED: Rate Limit Exceeded — $err_msg" ;;
            *)   echo "$MSG_OPT_FAILED: API Error ($http_code) — $err_msg" ;;
        esac
        return 1
    fi

    # Extract AI text content based on provider
    local content
    case "$provider" in
        openai|openrouter)
            content=$(printf '%s' "$body" | jq -r '.choices[0].message.content // empty') ;;
        anthropic)
            content=$(printf '%s' "$body" | jq -r '.content[0].text // empty') ;;
        gemini)
            content=$(printf '%s' "$body" | jq -r '.candidates[0].content.parts[0].text // empty') ;;
    esac

    if [[ -z "$content" ]]; then
        echo "$MSG_OPT_NO_SUGGESTIONS"
        return 0
    fi

    # Extract suggestions from AI response
    # New format: {"suggestions":[{"pid":123,"reason":"..."}]}
    # Fallback: {"pids":[123,456]}
    local ai_pids ai_reasons
    ai_pids=$(printf '%s' "$content" | jq -r '
        if type == "object" and has("suggestions") then
            .suggestions[] | select(.pid > 1) | .pid | tostring
        elif type == "object" and has("pids") then
            .pids[] | select(. > 1) | tostring
        else empty end
    ' 2>/dev/null || true)

    ai_reasons=$(printf '%s' "$content" | jq -r '
        if type == "object" and has("suggestions") then
            .suggestions[] | select(.pid > 1) | (.reason // "optimization candidate")
        elif type == "object" and has("pids") then
            .pids[] | select(. > 1) | "optimization candidate"
        else empty end
    ' 2>/dev/null || true)

    if [[ -z "$ai_pids" ]]; then
        echo "$MSG_OPT_NO_SUGGESTIONS"
        return 0
    fi

    # Build parallel arrays of PIDs and reasons
    local -a all_pids_arr=() all_reasons_arr=()
    while IFS= read -r line; do all_pids_arr+=("$line"); done <<< "$ai_pids"
    while IFS= read -r line; do all_reasons_arr+=("$line"); done <<< "$ai_reasons"

    # Validate and display each suggested process
    local valid_pids=() valid_names=() valid_reasons=()
    local pid proc_name
    echo ""
    echo "AI suggests closing these processes:"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    local idx=0
    for pid in "${all_pids_arr[@]}"; do
        [[ -n "$pid" ]] || { (( idx++ )) || true; continue; }
        proc_name=$(ps -p "$pid" -o comm= 2>/dev/null | xargs basename 2>/dev/null || true)
        [[ -n "$proc_name" ]] || { (( idx++ )) || true; continue; }
        # Skip blocked/system processes
        if macmon_is_blocked_process "$proc_name"; then (( idx++ )) || true; continue; fi
        if is_system_process "$proc_name"; then (( idx++ )) || true; continue; fi
        if _is_apple_system_pid "$pid" 2>/dev/null; then (( idx++ )) || true; continue; fi
        local ram_mb cpu_pct reason
        ram_mb=$(ps -p "$pid" -o rss= 2>/dev/null | awk '{printf "%.0f", $1/1024}' || echo "?")
        cpu_pct=$(ps -p "$pid" -o pcpu= 2>/dev/null | tr -d ' ' || echo "?")
        reason="${all_reasons_arr[$idx]:-optimization candidate}"
        printf "  PID %-7s %-25s RAM: %sMB  CPU: %s%%  (%s)\n" "$pid" "$proc_name" "$ram_mb" "$cpu_pct" "$reason"
        valid_pids+=("$pid")
        valid_names+=("$proc_name")
        valid_reasons+=("$reason")
        (( idx++ )) || true
    done
    echo ""

    if [[ ${#valid_pids[@]} -eq 0 ]]; then
        echo "$MSG_OPT_NO_SUGGESTIONS"
        return 0
    fi

    # Ask for confirmation
    printf '%s ' "$MSG_OPT_CONFIRM"
    local answer
    read -r answer </dev/tty || answer="n"
    case "$answer" in
        [Nn]*) echo "$MSG_OPT_CANCELLED"; return 0 ;;
    esac

    # Build kill payload and execute
    local kill_file
    kill_file=$(mktemp "${MACMON_TMPDIR}/macmon-kill.XXXXXX.json")
    local entries="["
    local idx
    for (( idx = 0; idx < ${#valid_pids[@]}; idx++ )); do
        [[ $idx -gt 0 ]] && entries+=","
        entries+=$(jq -n --argjson pid "${valid_pids[$idx]}" --arg name "${valid_names[$idx]}" '{pid:$pid,name:$name}')
    done
    entries+="]"
    printf '%s' "$entries" > "$kill_file"

    kill_processes "$kill_file"
    rm -f "$kill_file"
    echo "$MSG_CLOSED_PROCESSES ${#valid_pids[@]} process(es)"
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
  optimize        AI-powered process optimization (Smart Optimize for CLI)
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
  macmon optimize         # AI-powered process optimization
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
    optimize)       cmd_optimize ;;
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
