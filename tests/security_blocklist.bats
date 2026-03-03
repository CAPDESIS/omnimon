#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
}

@test "macmon_is_blocked_process: protects coreaudiod" {
    run macmon_is_blocked_process "coreaudiod"
    [ "$status" -eq 0 ]
}

@test "macmon_is_blocked_process: protects AudioComponentRegistrar" {
    run macmon_is_blocked_process "AudioComponentRegistrar"
    [ "$status" -eq 0 ]
}

@test "macmon_is_blocked_process: protects VTDecoderXPCService" {
    run macmon_is_blocked_process "VTDecoderXPCService"
    [ "$status" -eq 0 ]
}

@test "macmon_is_blocked_process: allows user process" {
    run macmon_is_blocked_process "my-user-app"
    [ "$status" -eq 1 ]
}
