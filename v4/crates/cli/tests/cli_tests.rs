use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_status_json_format() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    
    cmd.arg("status").arg("--format").arg("json");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"running gracefully""#))
        .stdout(predicate::str::contains(r#""memory_usage_bytes":"#));
}

#[test]
fn test_status_text_format() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    
    cmd.arg("status").arg("--format").arg("text");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("macmon status: running gracefully"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("cli").unwrap();
    
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OmniMon: Monitor de sistema"));
}
