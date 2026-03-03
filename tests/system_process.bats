#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
}

@test "is_system_process: launchd is protected" {
    run is_system_process "launchd"
    [ "$status" -eq 0 ]
}

@test "is_system_process: kernel_task is protected" {
    run is_system_process "kernel_task"
    [ "$status" -eq 0 ]
}

@test "is_system_process: WindowServer is protected" {
    run is_system_process "WindowServer"
    [ "$status" -eq 0 ]
}

@test "is_system_process: loginwindow is protected" {
    run is_system_process "loginwindow"
    [ "$status" -eq 0 ]
}

@test "is_system_process: random app is NOT protected" {
    run is_system_process "myapp"
    [ "$status" -eq 1 ]
}

@test "is_system_process: Chrome is NOT protected" {
    run is_system_process "Google Chrome"
    [ "$status" -eq 1 ]
}

@test "is_system_process: Slack is NOT protected" {
    run is_system_process "Slack"
    [ "$status" -eq 1 ]
}

@test "is_system_process: empty string is NOT protected" {
    run is_system_process ""
    [ "$status" -eq 1 ]
}
