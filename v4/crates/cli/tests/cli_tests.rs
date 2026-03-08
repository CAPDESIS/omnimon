use predicates::prelude::*;

#[test]
fn test_status_json_format() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("status").arg("--format").arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""status": "running""#))
        .stdout(predicate::str::contains(r#""total_memory_bytes""#))
        .stdout(predicate::str::contains(r#""used_memory_bytes""#))
        .stdout(predicate::str::contains(r#""top_processes""#));
}

#[test]
fn test_status_text_format() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("status").arg("--format").arg("text");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("omnimon status: running"))
        .stdout(predicate::str::contains("Memory:"))
        .stdout(predicate::str::contains("CPU:"))
        .stdout(predicate::str::contains("Top processes by memory:"));
}

#[test]
fn test_cli_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OmniMon: Monitor de sistema"));
}

#[test]
fn test_settings_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("settings").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage settings"));
}
