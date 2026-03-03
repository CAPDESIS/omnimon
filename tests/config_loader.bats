#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
    # Load the default config
    macmon_load_config ""
}

teardown() {
    _macmon_test_teardown
}

@test "macmon_cfg: reads default RAM threshold" {
    result=$(macmon_cfg "THRESHOLDS_RAM_FREE_PERCENT" "99")
    [ "$result" = "25" ]
}

@test "macmon_get_custom_processes: reads default dynamic process list" {
    result=$(macmon_get_custom_processes)
    [[ "$result" == *"flutter_tester:10:0:0"* ]]
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

@test "macmon_load_config: falls back on tab-indented YAML" {
    mkdir -p "$HOME/.config/macmon"
    tmp_cfg=$(mktemp "$HOME/.config/macmon/macmon-tabcfg.XXXXXX.yaml")
    cat > "$tmp_cfg" <<'EOF'
thresholds:
	ram_free_percent: 5
EOF

    macmon_load_config "$tmp_cfg"
    result=$(macmon_cfg "THRESHOLDS_RAM_FREE_PERCENT" "99")
    [ "$result" = "25" ]
    [ "${MACMON_CFG_CONFIG_ERROR}" = "tabs_in_yaml" ]
    rm -f "$tmp_cfg"
}

@test "macmon_get_custom_processes: invalid custom_processes falls back to defaults" {
    mkdir -p "$HOME/.config/macmon"
    tmp_cfg=$(mktemp "$HOME/.config/macmon/macmon-badcustom.XXXXXX.yaml")
    cat > "$tmp_cfg" <<'EOF'
custom_processes:
  - wrong_key: "node"
EOF

    export MACMON_CONFIG="$tmp_cfg"
    macmon_load_config "$tmp_cfg"
    result=$(macmon_get_custom_processes)
    [[ "$result" == *"flutter_tester:10:0:0"* ]]
    [ "${MACMON_CFG_CONFIG_ERROR}" = "invalid_custom_processes" ]
    rm -f "$tmp_cfg"
    unset MACMON_CONFIG
}
