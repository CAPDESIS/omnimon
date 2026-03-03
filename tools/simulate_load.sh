#!/usr/bin/env bash
# simulate_load.sh - Generate fake load for macmon demo recordings
#
# Usage:
#   ./tools/simulate_load.sh                # spawn all demo processes
#   ./tools/simulate_load.sh flutter        # spawn only flutter_tester fakes
#   ./tools/simulate_load.sh ram            # spawn only RAM hog
#   ./tools/simulate_load.sh orphans        # spawn only fake orphan daemons
#   ./tools/simulate_load.sh cleanup        # kill all demo processes
#
# After running, use macmon to see them detected:
#   macmon status        # shows flutter count, orphan daemons, RAM pressure
#   macmon               # opens the picker with all fake processes visible
#
# The daemon will also show a native notification if thresholds are crossed.

set -euo pipefail

FLUTTER_COUNT=15
RAM_HOG_MB=200

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { printf "${CYAN}[demo]${NC} %s\n" "$*"; }
ok()    { printf "${GREEN}[demo]${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}[demo]${NC} %s\n" "$*"; }

# --- Spawn Functions ---

spawn_flutter() {
    info "Spawning $FLUTTER_COUNT fake flutter_tester processes..."
    for (( i = 1; i <= FLUTTER_COUNT; i++ )); do
        # Create a binary named flutter_tester that just sleeps
        # We use a symlink to /bin/sleep so ps shows "flutter_tester"
        local tmpbin="${TMPDIR}/flutter_tester"
        if [[ ! -f "$tmpbin" ]]; then
            cp /bin/sleep "$tmpbin"
            chmod +x "$tmpbin"
        fi
        "$tmpbin" 3600 &
    done
    ok "Spawned $FLUTTER_COUNT flutter_tester processes (PIDs will show in macmon)"
    ok "Threshold is 10 — daemon should alert within 60 seconds"
}

spawn_ram_hog() {
    info "Spawning RAM hog process (~${RAM_HOG_MB}MB)..."
    # Use Python to allocate memory and hold it
    python3 -c "
import time, sys, os
# Allocate ${RAM_HOG_MB}MB of RAM
data = bytearray(${RAM_HOG_MB} * 1024 * 1024)
# Touch every page to force physical allocation
for i in range(0, len(data), 4096):
    data[i] = 1
sys.stdout.write('RAM hog active: ${RAM_HOG_MB}MB allocated\n')
sys.stdout.flush()
# Sleep until killed
time.sleep(3600)
" &
    local pid=$!
    ok "RAM hog running (PID $pid, ~${RAM_HOG_MB}MB)"
    ok "Will appear in picker sorted by RAM"
}

spawn_orphans() {
    info "Spawning fake orphan build daemons..."

    # Fake SourceKitService (only triggers alert when Xcode is NOT running)
    local tmpsk="${TMPDIR}/SourceKitService"
    if [[ ! -f "$tmpsk" ]]; then
        cp /bin/sleep "$tmpsk"
        chmod +x "$tmpsk"
    fi
    "$tmpsk" 3600 &
    "$tmpsk" 3600 &
    ok "Spawned 2 fake SourceKitService processes"

    # Fake xcodebuild
    local tmpxb="${TMPDIR}/xcodebuild"
    if [[ ! -f "$tmpxb" ]]; then
        cp /bin/sleep "$tmpxb"
        chmod +x "$tmpxb"
    fi
    "$tmpxb" 3600 &
    ok "Spawned 1 fake xcodebuild process"

    ok "Orphan detection will trigger if Xcode is not running"
}

# --- Cleanup ---

cleanup_all() {
    warn "Cleaning up all demo processes..."
    local count=0

    # Kill flutter_tester fakes
    local ft_pids
    ft_pids=$(pgrep -f "${TMPDIR}/flutter_tester" 2>/dev/null || true)
    if [[ -n "$ft_pids" ]]; then
        echo "$ft_pids" | xargs kill 2>/dev/null || true
        count=$(echo "$ft_pids" | wc -l | tr -d ' ')
        ok "Killed $count flutter_tester processes"
    fi

    # Kill SourceKitService fakes
    local sk_pids
    sk_pids=$(pgrep -f "${TMPDIR}/SourceKitService" 2>/dev/null || true)
    if [[ -n "$sk_pids" ]]; then
        echo "$sk_pids" | xargs kill 2>/dev/null || true
        count=$(echo "$sk_pids" | wc -l | tr -d ' ')
        ok "Killed $count SourceKitService processes"
    fi

    # Kill xcodebuild fakes
    local xb_pids
    xb_pids=$(pgrep -f "${TMPDIR}/xcodebuild" 2>/dev/null || true)
    if [[ -n "$xb_pids" ]]; then
        echo "$xb_pids" | xargs kill 2>/dev/null || true
        count=$(echo "$xb_pids" | wc -l | tr -d ' ')
        ok "Killed $count xcodebuild processes"
    fi

    # Kill python RAM hog
    local py_pids
    py_pids=$(pgrep -f "RAM hog active" 2>/dev/null || true)
    if [[ -n "$py_pids" ]]; then
        echo "$py_pids" | xargs kill 2>/dev/null || true
        ok "Killed RAM hog process"
    fi

    # Clean temp binaries
    rm -f "${TMPDIR}/flutter_tester" "${TMPDIR}/SourceKitService" "${TMPDIR}/xcodebuild"

    ok "Cleanup complete"
}

# --- Main ---

case "${1:-all}" in
    flutter)
        spawn_flutter
        ;;
    ram)
        spawn_ram_hog
        ;;
    orphans)
        spawn_orphans
        ;;
    cleanup|clean)
        cleanup_all
        ;;
    all)
        echo ""
        printf '%b' "${BOLD}macmon Demo Load Generator${NC}\n"
        echo "========================="
        echo ""
        spawn_flutter
        echo ""
        spawn_ram_hog
        echo ""
        spawn_orphans
        echo ""
        printf '%b' "${BOLD}Demo processes are running.${NC} Now you can:\n"
        echo ""
        echo "  1. Wait ~60s for the daemon to detect and show notifications"
        echo "  2. Run 'macmon status' to see the alerts in the terminal"
        echo "  3. Run 'macmon' to open the picker and see all processes"
        echo "  4. Record your screen for the demo video"
        echo ""
        echo "When done:"
        echo "  ./tools/simulate_load.sh cleanup"
        echo ""
        ;;
    *)
        echo "Usage: $0 {all|flutter|ram|orphans|cleanup}" >&2
        exit 1
        ;;
esac
