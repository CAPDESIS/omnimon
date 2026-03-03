#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
    # Load the default config
    macmon_load_config ""
}

@test "macmon_cfg: reads default RAM threshold" {
    result=$(macmon_cfg "THRESHOLDS_RAM_FREE_PERCENT" "99")
    [ "$result" = "25" ]
}

@test "macmon_cfg: reads default flutter threshold" {
    result=$(macmon_cfg "THRESHOLDS_FLUTTER_PROCESS_COUNT" "99")
    [ "$result" = "10" ]
}

@test "macmon_cfg: reads default swap threshold" {
    result=$(macmon_cfg "THRESHOLDS_SWAP_USED_MB" "99")
    [ "$result" = "2048" ]
}

@test "macmon_cfg: reads default check interval" {
    result=$(macmon_cfg "INTERVALS_CHECK" "99")
    [ "$result" = "60" ]
}

@test "macmon_cfg: returns fallback for nonexistent key" {
    result=$(macmon_cfg "NONEXISTENT_KEY" "fallback_value")
    [ "$result" = "fallback_value" ]
}

@test "macmon_cfg: protected list contains launchd" {
    result=$(macmon_cfg "PROTECTED" "")
    [[ "$result" == *"launchd"* ]]
}

@test "macmon_cfg: protected list is colon-delimited" {
    result=$(macmon_cfg "PROTECTED" "")
    [[ "$result" == *":"* ]]
}

@test "macmon_cfg: reads default idle CPU threshold" {
    result=$(macmon_cfg "THRESHOLDS_IDLE_CPU_PERCENT" "99")
    [ "$result" = "1.0" ]
}

@test "macmon_cfg: reads log max size" {
    result=$(macmon_cfg "LOG_MAX_SIZE_MB" "99")
    [ "$result" = "10" ]
}
