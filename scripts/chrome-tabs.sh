#!/usr/bin/env bash
# chrome-tabs.sh - Enumerate Chrome tab titles via AppleScript
# Usage: chrome-tabs.sh [renderer-client-id]
# Outputs: tab_id\ttitle\turl for each tab

set -euo pipefail

enumerate_tabs() {
    osascript <<'APPLESCRIPT' 2>/dev/null || true
tell application "Google Chrome"
    set output to ""
    repeat with w in windows
        repeat with t in tabs of w
            set output to output & (id of t) & tab & (title of t) & tab & (URL of t) & linefeed
        end repeat
    end repeat
    return output
end tell
APPLESCRIPT
}

# If Chrome is not running, exit silently
if ! pgrep -x "Google Chrome" > /dev/null 2>&1; then
    exit 0
fi

if [[ "${1:-}" == "--json" ]]; then
    # Output as JSON array
    enumerate_tabs | jq -R -s '
        split("\n") | map(select(length > 0)) |
        map(split("\t") | {id: .[0], title: (.[1] // ""), url: (.[2] // "")})
    '
else
    enumerate_tabs
fi
