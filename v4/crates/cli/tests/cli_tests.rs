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

#[test]
fn test_settings_help_mentions_presets() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("settings").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("presets"))
        .stdout(predicate::str::contains("use"));
}

#[test]
fn test_network_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("network telemetry"));
}

#[test]
fn test_network_json_format() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--format").arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""capture_backend""#))
        .stdout(predicate::str::contains(r#""net_rx_bytes_per_sec""#));
}

#[test]
fn test_network_text_format() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--format").arg("text");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OmniMon Network Telemetry"))
        .stdout(predicate::str::contains("Backend:"));
}

#[test]
fn test_network_connections_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--connections");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Filtered connections:"));
}

#[test]
fn test_network_filter_and_port_json_output() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network")
        .arg("--format")
        .arg("json")
        .arg("--filter")
        .arg("tcp")
        .arg("--port")
        .arg("443");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"view\": \"connections\""))
        .stdout(predicate::str::contains("\"filters\""))
        .stdout(predicate::str::contains("443"));
}

#[test]
fn test_network_alerts_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--alerts");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Network alerts:"));
}

#[test]
fn test_network_top_flag_json_output() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network").arg("--top").arg("--format").arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"view\": \"top\""))
        .stdout(predicate::str::contains("\"top_processes\""));
}

#[test]
fn test_network_watch_single_iteration() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("network")
        .arg("--watch")
        .arg("--watch-iterations")
        .arg("1")
        .arg("--watch-interval-ms")
        .arg("10");

    cmd.assert().success().stdout(predicate::str::contains(
        "Watching network telemetry every 10 ms",
    ));
}

#[test]
fn test_rules_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("rules").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("security alert rules"));
}

#[test]
fn test_rules_list() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("rules").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("security rules"));
}

#[test]
fn test_rules_schema() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("rules").arg("schema");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("schema_version"))
        .stdout(predicate::str::contains("process_cidr"));
}

#[test]
fn test_rules_load_nonexistent_file() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("rules")
        .arg("load")
        .arg("/tmp/nonexistent_rules_file.json");

    cmd.assert().failure();
}

#[test]
fn test_rules_remove_nonexistent() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("cli");

    cmd.arg("rules").arg("remove").arg("nonexistent-rule-id");

    cmd.assert().failure();
}
