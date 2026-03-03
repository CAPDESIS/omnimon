#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
}

@test "calc_uptime: recent process shows minutes" {
    # Create a timestamp 5 minutes ago
    local lstart
    lstart=$(date -v-5M "+%a %b %d %T %Y")
    result=$(calc_uptime "$lstart")
    [[ "$result" =~ ^[0-9]+m$ ]]
}

@test "calc_uptime: process from hours ago shows hours" {
    local lstart
    lstart=$(date -v-3H "+%a %b %d %T %Y")
    result=$(calc_uptime "$lstart")
    [[ "$result" =~ ^[0-9]+h\ [0-9]+m$ ]]
}

@test "calc_uptime: process from days ago shows days" {
    local lstart
    lstart=$(date -v-2d "+%a %b %d %T %Y")
    result=$(calc_uptime "$lstart")
    [[ "$result" =~ ^[0-9]+d\ [0-9]+h$ ]]
}

@test "uptime_seconds: returns positive integer" {
    local lstart
    lstart=$(date -v-10M "+%a %b %d %T %Y")
    result=$(uptime_seconds "$lstart")
    [[ "$result" =~ ^[0-9]+$ ]]
    (( result >= 500 && result <= 700 ))
}

@test "uptime_seconds: bad input returns 0" {
    result=$(uptime_seconds "invalid date string")
    [ "$result" = "0" ]
}
