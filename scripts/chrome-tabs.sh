#!/usr/bin/env bash
# chrome-tabs.sh - Enumerate Chrome tab titles via AppleScript
# Usage: chrome-tabs.sh [renderer-client-id]
# Outputs: tab_id\ttitle\turl for each tab

set -euo pipefail

enumerate_tabs() {
    osascript <<'APPLESCRIPT' 2>/dev/null || true
on sanitizeText(inputText)
    set t to inputText as text
    return do shell script "printf %s " & quoted form of t & " | tr '\t\r\n' '   '"
end sanitizeText

tell application "Google Chrome"
    set sep to (character id 31)
    set output to ""
    repeat with w in windows
        repeat with t in tabs of w
            set tabID to (id of t as text)
            set tabTitle to my sanitizeText(title of t as text)
            set tabURL to my sanitizeText(URL of t as text)
            set output to output & tabID & sep & tabTitle & sep & tabURL & linefeed
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
        map(split("\u001f") | {id: .[0], title: (.[1] // ""), url: (.[2] // "")})
    '
else
    enumerate_tabs
fi
