#!/usr/bin/env bash

# Immutable blocklist for critical system processes and command patterns.

readonly MACMON_BLOCKED_PROCESSES="WindowServer:coreaudiod:VTDecoderXPCService:kernel_task:launchd:syslogd:logd:notifyd"
readonly MACMON_BLOCKED_COMMAND_PATTERNS="rm -rf|sudo|launchctl|osascript|curl|sh -c"

macmon_is_blocked_process() {
    local name="$1"
    local IFS=':'
    local proc
    for proc in $MACMON_BLOCKED_PROCESSES; do
        [[ "$name" == "$proc" ]] && return 0
    done
    return 1
}

macmon_contains_blocked_command_pattern() {
    local input="$1"
    [[ "$input" =~ $MACMON_BLOCKED_COMMAND_PATTERNS ]]
}
