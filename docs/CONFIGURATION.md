# Configuration

macmon uses a YAML configuration file at `~/.config/macmon/macmon.yaml`.

## Managing Configuration

```bash
macmon config         # Show current config
macmon config edit    # Open in $EDITOR
macmon config reset   # Reset to defaults
macmon config path    # Show config file path
```

After editing, reload without restart:
```bash
kill -USR1 $(cat ${TMPDIR}/macmond.pid)
```

Or simply: `macmon restart`

## All Options

### Thresholds

```yaml
thresholds:
  flutter_process_count: 10    # Alert when flutter_tester count exceeds this
  ram_free_percent: 25         # Alert when free RAM percentage drops below this
  swap_used_mb: 2048           # Alert when swap usage (MB) exceeds this
  process_ram_min_kb: 102400   # Minimum RSS (KB) for a process to appear in picker
  idle_cpu_percent: 1.0        # CPU% below this marks a process as idle
  idle_ram_trigger_percent: 40 # Only suggest idle cleanup when free RAM below this
```

### Intervals

```yaml
intervals:
  check: 60          # Seconds between monitoring cycles
  idle_check: 600    # Seconds between proactive idle process scans
  cooldown: 300      # Minimum seconds between same-type alerts
  kill_grace: 3      # Seconds to wait after SIGTERM before SIGKILL
```

### Process Collection

```yaml
collect:
  disk_io: true              # Collect per-process disk I/O via proc_pid_rusage
  batch_lsof_limit: 50       # Max processes for batch lsof (skip if more)
  chrome_tab_titles: true    # Resolve Chrome tab titles via AppleScript
```

The `disk_io` option enables the DiskIOHelper binary which uses `proc_pid_rusage` with `RUSAGE_INFO_V4` to collect per-process disk read/write bytes. This does not require root or elevated privileges.

### Logging

```yaml
log:
  max_size_mb: 10              # Rotate log when it exceeds this size
  max_files: 5                 # Keep this many rotated log files
  dir: ~/.local/log/macmon     # Log directory
```

The `~` prefix is automatically expanded to `$HOME`. Log rotation happens at the start of each daemon cycle.

### Protected Processes

Processes in this list can never be killed through macmon. Additionally, macmon verifies Apple code signatures on processes claiming protected names to prevent spoofing:

```yaml
protected:
  - launchd
  - kernel_task
  - WindowServer
  - loginwindow
  - coreaudiod
  - bluetoothd
  - fseventsd
  - mds
  - mds_stores
  - opendirectoryd
  - syslogd
  - configd
  - diskarbitrationd
  - powerd
  - thermalmonitord
  - UserEventAgent
  - cfprefsd
  - distnoted
  - logd
  - notifyd
```

Add your own protected processes by appending to this list.

## Metrics Export

Export current system snapshots or historical peaks:

```bash
macmon export           # JSON snapshot (stdout)
macmon export json      # JSON snapshot
macmon export csv       # CSV format (spreadsheet-friendly)
macmon export --peaks   # Daily peak RAM/CPU per process
```

JSON output includes both a `system` object (free RAM %, swap, process count) and a `processes` array with all fields including disk I/O.

CSV output includes columns: PID, Name, RAM_MB, CPU_Pct, Uptime, State, Idle, DiskRead_MB, DiskWrite_MB, Group, CWD.

Peak tracking stores the highest RAM and CPU values seen per process per day in `$LOG_DIR/peaks.json`.

## Orphan Daemon Detection

The daemon automatically detects orphaned build processes:

| Process | Detection Logic |
|---------|----------------|
| SourceKitService | Running but Xcode is not |
| GradleDaemon | Java process with GradleDaemon in args, no Android Studio |
| xcodebuild | Still running after Xcode closes |
| qemu-system-aarch64 | Android emulator with no Android Studio |

Orphans trigger a native macOS notification offering to open the process picker for cleanup.

## Menu Bar Monitor

The menu bar icon (`MacmonStatusBar`) displays live RAM usage and refreshes every 30 seconds using native `host_statistics64` calls (no subprocess spawning).

Launch it manually:
```bash
MACMON_HOME=~/.local/libexec/macmon ~/.local/libexec/macmon/MacmonStatusBar &
```

Or add it to Login Items for auto-start. The menu provides:
- RAM usage (color-coded: green ≥40%, yellow ≥20%, red <20%)
- Swap usage
- Total process count
- Quick actions: Open Picker, Export (JSON/CSV), Status, Quit

## Default Values

If no config file exists, macmon uses the defaults from `config/macmon.default.yaml`. The installer copies this to `~/.config/macmon/macmon.yaml` on first install.

## Environment Variables

These override the config file:

| Variable | Description |
|----------|-------------|
| `MACMON_HOME` | Base directory for macmon files |
| `MACMON_CONFIG` | Path to config YAML file |
| `MACMON_LOG_DIR` | Override log directory |
| `MACMON_LOG_FILE` | Override log file path |
