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

### Logging

```yaml
log:
  max_size_mb: 10    # Rotate log when it exceeds this size
  max_files: 5       # Keep this many rotated log files
  dir: ~/.local/log/macmon  # Log directory
```

### Process Collection

```yaml
collect:
  batch_lsof_limit: 50     # Max processes for batch lsof (skip if more)
  chrome_tab_titles: true   # Resolve Chrome tab titles via AppleScript
```

### Protected Processes

Processes in this list can never be killed through macmon:

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
