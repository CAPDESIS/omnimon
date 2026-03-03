#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
}

@test "friendly_name: generic binary passthrough" {
    result=$(friendly_name "helper-tool" "")
    [ "$result" = "helper-tool" ]
}

@test "friendly_name: node script path labeling" {
    result=$(friendly_name "node" "/usr/local/lib/node_modules/my-tool/cli.js")
    [ "$result" = "Node: cli.js" ]
}

@test "friendly_name: plain binary with no mapping" {
    result=$(friendly_name "agent-runner" "")
    [ "$result" = "agent-runner" ]
}

@test "friendly_name: Warp Terminal (stable binary)" {
    result=$(friendly_name "stable" "/Applications/Warp.app/Contents/MacOS/stable")
    [ "$result" = "Warp Terminal" ]
}

@test "friendly_name: Chrome main process" {
    result=$(friendly_name "Google Chrome" "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
    [ "$result" = "Google Chrome" ]
}

@test "friendly_name: Chrome tab (renderer)" {
    result=$(friendly_name "Google Chrome Helper" "--type=renderer --renderer-client-id=12")
    [ "$result" = "Chrome Tab" ]
}

@test "friendly_name: Chrome GPU process" {
    result=$(friendly_name "Google Chrome Helper" "--type=gpu-process")
    [ "$result" = "Chrome GPU" ]
}

@test "friendly_name: Chrome Utility process" {
    result=$(friendly_name "Google Chrome Helper" "--type=utility")
    [ "$result" = "Chrome Utility" ]
}

@test "friendly_name: Chrome generic helper" {
    result=$(friendly_name "Google Chrome Helper" "--type=broker")
    [ "$result" = "Chrome Helper" ]
}

@test "friendly_name: .app bundle extraction" {
    result=$(friendly_name "/Applications/Slack.app/Contents/MacOS/Slack" "")
    [ "$result" = "Slack" ]
}

@test "friendly_name: SourceKitService" {
    result=$(friendly_name "SourceKitService" "")
    [ "$result" = "SourceKitService" ]
}

@test "friendly_name: Gradle Daemon via java args" {
    result=$(friendly_name "java" "org.gradle.launcher.daemon.bootstrap.GradleDaemon")
    [ "$result" = "Gradle Daemon" ]
}

@test "friendly_name: Android Emulator (qemu)" {
    result=$(friendly_name "qemu-system-x86_64" "")
    [ "$result" = "Android Emulator" ]
}

@test "friendly_name: xcodebuild" {
    result=$(friendly_name "xcodebuild" "")
    [ "$result" = "xcodebuild" ]
}

@test "friendly_name: Node.js with script" {
    result=$(friendly_name "node" "node /home/user/app/server.js")
    [ "$result" = "Node: server.js" ]
}

@test "friendly_name: plain binary passthrough" {
    result=$(friendly_name "zsh" "")
    [ "$result" = "zsh" ]
}

@test "friendly_name: plain binary with path" {
    result=$(friendly_name "/usr/bin/python3" "")
    [ "$result" = "python3" ]
}
