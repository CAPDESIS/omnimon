# Network Analysis Guide

## Scope

OmniMon exposes two complementary network data paths:

- `watcher` telemetry for per-process throughput, recent flow events, and evaluated network alerts.
- `network_analysis` snapshots for connection-level inspection, filtering, reverse-DNS caching, and short history windows.

Use throughput when you want to know who is using bandwidth right now. Use connection snapshots when you need to inspect endpoints, ports, states, or process-specific sockets.

## CLI Workflows

### Top throughput

```bash
omnimon network --top
omnimon network --format json
```

- Shows global RX/TX rates, capture backend, and the hottest processes.
- JSON mode is useful for scripts that need `top_processes` plus aggregate byte rates.

### Connection inspection

```bash
omnimon network --connections
omnimon network --filter tcp --port 443
```

- `--connections` prints the latest analyzed socket snapshot.
- `--filter` narrows by protocol: `tcp`, `udp`, `icmp`, or `other`.
- `--port` matches either the local or remote port.
- Filtered output is backed by `NetworkFilter`, which can also be driven from Tauri IPC.

### Alert review

```bash
omnimon network --alerts
```

- Prints the currently evaluated network alerts held by the watcher.
- Alerts include severity, rule id, destination, and the human-readable message.

### Watch mode

```bash
omnimon network --watch
omnimon network --alerts --watch --watch-interval-ms 1000
```

- Re-renders the selected view until interrupted.
- `--watch-iterations` is available for automation and tests.

## Frontend / IPC

Desktop IPC commands:

- `get_network_data` returns watcher telemetry (`top_processes`, `recent_connections`, backend, DPI, totals).
- `get_network_connections` returns the latest analyzed `NetworkSnapshot`.
- `get_network_history(seconds)` returns recent analyzed snapshots.
- `get_filtered_connections(filter)` applies the backend `NetworkFilter`.

The desktop store in `v4/apps/desktop/src/stores/network.svelte.ts` keeps the current snapshot, rolling history, UI filters, and derived per-process summaries.

## Alert Evaluation Model

Network alert rules live in `v4/crates/core/src/network_alerts.rs` and support:

- `high_bandwidth`
- `new_external_connection`
- `unusual_port`
- `process_network_spike`
- `connection_count_exceeded`
- `suspicious_destination`

### Configuring each alert type

- `high_bandwidth`
  - use when a process or total traffic should exceed a Mbps threshold
  - fields: `threshold_mbps`, `direction`, optional `process`
  - example: alert if `chrome` exceeds `400 Mbps` upload
- `new_external_connection`
  - use when you want to know about newly observed outbound destinations
  - field: `exclude_known`
  - example: alert only for destinations not seen before in the current evaluator state
- `unusual_port`
  - use to watch suspicious or non-standard destination ports
  - field: `suspicious_ports`
  - example: `4444, 6667, 31337`
- `process_network_spike`
  - use when a named process suddenly exceeds its recent baseline
  - fields: `process_name`, `multiplier`
  - example: alert if `chrome` goes above `5x` its recent average
- `connection_count_exceeded`
  - use when a process opens too many concurrent/recent connections
  - fields: `max_connections`, optional `process`
  - example: alert if any process exceeds `200` connections
- `suspicious_destination`
  - use regex patterns for IP ranges, host indicators, or IOC strings
  - field: `patterns`
  - example: `(^198\.51\.100\.)|malware|botnet`

In the desktop UI, these map directly to `NetworkAlertConfig` inputs: type, severity, cooldown, optional AI notification, and the condition-specific fields above.

Important behavior:

- Rules require 3 consecutive matching snapshots before emitting.
- Cooldowns suppress duplicate emissions for the same debounce key.
- Known external destinations can be remembered and skipped when `exclude_known` is enabled.

## DNS Cache Notes

Reverse-DNS enrichment in `network_analysis` is best-effort:

- cache TTL: 300 seconds
- stale entries can survive until the 2x TTL eviction window
- concurrency is capped to avoid too many background resolutions
- loopback and cached entries are skipped by bulk enqueue logic

If hostnames are missing, the connection data is still valid; only enrichment is deferred.

## Validation

Recommended checks for network-analysis changes:

```bash
cargo test --workspace -- --test-threads=1
cd v4/apps/desktop && bun run test && bun run build
```

For narrower iteration:

```bash
cargo test -p core network_analysis -- --test-threads=1
cargo test -p core network_alerts -- --test-threads=1
cargo test -p cli -- --test-threads=1
cd v4/apps/desktop && bun run test src/stores/__tests__/network.test.ts
```
