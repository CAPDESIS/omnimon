# test_helper.bash - Common setup for macmon BATS tests

# Set MACMON_HOME to the repo root
export MACMON_HOME="$(cd "$(dirname "$BATS_TEST_FILENAME")/.." && pwd)"

# Source the shared library (but skip set -e so BATS can handle failures)
_macmon_test_setup() {
    # Override set to be non-fatal for testing
    set +euo pipefail

    # Source config loader first
    source "${MACMON_HOME}/lib/macmon-config.sh"

    # Load default config
    macmon_load_config ""

    # Source core library (functions only, no side effects)
    source "${MACMON_HOME}/lib/macmon-core.sh"
}

# Mock system commands by prepending a mock directory to PATH
MOCK_DIR=""
setup_mocks() {
    MOCK_DIR=$(mktemp -d "${TMPDIR:-/tmp}/macmon-test-mocks.XXXXXX")
    export PATH="${MOCK_DIR}:${PATH}"
}

teardown_mocks() {
    [[ -n "$MOCK_DIR" && -d "$MOCK_DIR" ]] && rm -rf "$MOCK_DIR"
}

# Create a mock command that outputs a fixed string
mock_command() {
    local cmd="$1"
    local output="$2"
    cat > "${MOCK_DIR}/${cmd}" <<EOF
#!/bin/bash
echo "$output"
EOF
    chmod +x "${MOCK_DIR}/${cmd}"
}

# Create a mock command that exits with a specific code
mock_command_exit() {
    local cmd="$1"
    local code="$2"
    cat > "${MOCK_DIR}/${cmd}" <<EOF
#!/bin/bash
exit $code
EOF
    chmod +x "${MOCK_DIR}/${cmd}"
}
