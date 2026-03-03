#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
    mkdir -p "$HOME/.config/macmon/profiles"
}

@test "macmon_list_profiles: returns profile names" {
    cat > "$HOME/.config/macmon/profiles/test-dev.yaml" <<'EOF'
thresholds:
  ram_free_percent: 30
EOF
    run macmon_list_profiles
    [ "$status" -eq 0 ]
    [[ "$output" == *"test-dev"* ]]
    rm -f "$HOME/.config/macmon/profiles/test-dev.yaml"
}

@test "macmon_set_active_profile: stores selected profile" {
    cat > "$HOME/.config/macmon/profiles/test-creator.yaml" <<'EOF'
thresholds:
  ram_free_percent: 20
EOF
    run macmon_set_active_profile "test-creator"
    [ "$status" -eq 0 ]
    run macmon_get_active_profile
    [ "$status" -eq 0 ]
    [ "$output" = "test-creator" ]
    rm -f "$HOME/.config/macmon/profiles/test-creator.yaml"
    rm -f "$HOME/.config/macmon/active_profile"
}
