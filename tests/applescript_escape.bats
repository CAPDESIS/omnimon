#!/usr/bin/env bats

load test_helper

setup() {
    _macmon_test_setup
}

@test "_applescript_escape: normal text passes through" {
    result=$(_applescript_escape "Hello World")
    [ "$result" = "Hello World" ]
}

@test "_applescript_escape: escapes double quotes" {
    result=$(_applescript_escape 'say "hello"')
    [ "$result" = 'say \"hello\"' ]
}

@test "_applescript_escape: escapes backslashes" {
    result=$(_applescript_escape 'path\to\file')
    [ "$result" = 'path\\to\\file' ]
}

@test "_applescript_escape: escapes both quotes and backslashes" {
    result=$(_applescript_escape 'he said \"hi\"')
    [ "$result" = 'he said \\\"hi\\\"' ]
}

@test "_applescript_escape: strips control characters" {
    # \x01 is SOH control character
    input=$'hello\x01world'
    result=$(_applescript_escape "$input")
    [ "$result" = "helloworld" ]
}

@test "_applescript_escape: empty string" {
    result=$(_applescript_escape "")
    [ "$result" = "" ]
}

@test "_applescript_escape: injection attempt with semicolons" {
    result=$(_applescript_escape '"; do shell script "rm -rf /"')
    [ "$result" = '\"; do shell script \"rm -rf /\"' ]
}
