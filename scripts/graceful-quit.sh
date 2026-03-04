#!/usr/bin/env bash
# graceful-quit.sh - Graceful app/tab closing via AppleScript
# Usage: graceful-quit.sh app "AppName"
#        graceful-quit.sh chrome-tab PID [URL]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${SCRIPT_DIR}/lib/macmon-core.sh" 2>/dev/null || true

log_msg() {
    if declare -F macmon_log >/dev/null 2>&1; then
        macmon_log "$*"
    else
        printf '%s [graceful-quit] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >&2
    fi
}

quit_app() {
    local app_name="$1"
    local safe_name
    safe_name=$(_applescript_escape "$app_name")
    osascript <<EOF 2>/dev/null || true
tell application "$safe_name"
    quit
end tell
EOF
}

close_chrome_tab() {
    local target_pid="$1"
    local target_url="${2:-}"
    local args renderer_id result

    if [[ -n "$target_url" ]]; then
        result=$(osascript - "$target_url" <<'APPLESCRIPT' 2>/dev/null || true
on run argv
    set targetURL to item 1 of argv
    tell application "Google Chrome"
        repeat with w in windows
            repeat with t in tabs of w
                if (URL of t as text) is targetURL then
                    close t
                    return "closed"
                end if
            end repeat
        end repeat
    end tell
    return "not_found"
end run
APPLESCRIPT
)
        if [[ "$result" == "closed" ]]; then
            log_msg "Closed Chrome tab via URL match ($target_url)"
            return 0
        fi
    fi

    args=$(ps -p "$target_pid" -o args= 2>/dev/null || true)
    renderer_id=$(printf '%s' "$args" | sed -n 's/.*--renderer-client-id=\([0-9][0-9]*\).*/\1/p' | head -1)

    if [[ -z "$renderer_id" ]]; then
        log_msg "Chrome tab close skipped: PID $target_pid has no renderer-client-id"
        kill -TERM "$target_pid" 2>/dev/null || true
        return 1
    fi

    result=$(osascript - "$renderer_id" <<'APPLESCRIPT' 2>/dev/null || true
on run argv
    set targetID to item 1 of argv
    tell application "Google Chrome"
        repeat with w in windows
            repeat with t in tabs of w
                if (id of t as text) is targetID then
                    close t
                    return "closed"
                end if
            end repeat
        end repeat
    end tell
    return "not_found"
end run
APPLESCRIPT
)

    if [[ "$result" == "closed" ]]; then
        log_msg "Closed Chrome tab id=$renderer_id via AppleScript (PID $target_pid)"
        return 0
    fi

    log_msg "Chrome tab id=$renderer_id not found; SIGTERM fallback for PID $target_pid"
    kill -TERM "$target_pid" 2>/dev/null || true
    return 1
}

case "${1:-}" in
    app)
        [[ -n "${2:-}" ]] && quit_app "$2"
        ;;
    chrome-tab)
        [[ -n "${2:-}" ]] && close_chrome_tab "$2" "${3:-}"
        ;;
    *)
        echo "Usage: $0 {app <name> | chrome-tab <pid> [url]}" >&2
        exit 1
        ;;
esac
