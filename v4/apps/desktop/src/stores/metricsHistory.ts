import { writable, derived } from "svelte/store";
import type { SystemStats } from "../lib/types";

export interface MetricPoint {
  time: number; // epoch seconds
  value: number;
}

export interface MetricsSnapshot {
  time: number;
  cpuAvg: number;
  ramPct: number;
  netRx: number;
  netTx: number;
  swapMb: number;
  processCount: number;
}

const MAX_HISTORY = 300; // 5 min at 1s intervals, 10 min at 2s

/** Rolling buffer of system metric snapshots. */
export const metricsHistory = writable<MetricsSnapshot[]>([]);

/** Push a new stats snapshot into the rolling history buffer. */
export function pushMetrics(stats: SystemStats, cpuAvg: number): void {
  metricsHistory.update((history) => {
    const snap: MetricsSnapshot = {
      time: Math.floor(Date.now() / 1000),
      cpuAvg,
      ramPct: stats.ram_used_pct,
      netRx: stats.net_rx_bytes_per_sec,
      netTx: stats.net_tx_bytes_per_sec,
      swapMb: stats.swap_used_mb,
      processCount: stats.total_processes,
    };
    history.push(snap);
    if (history.length > MAX_HISTORY) history.splice(0, history.length - MAX_HISTORY);
    return history;
  });
}

/**
 * Derived series for each metric, ready for chart consumption.
 * Each .map() creates a new array on every update, but this is acceptable:
 * the buffer is bounded by MAX_HISTORY (300 points), so allocation is O(300).
 */
export const cpuSeries = derived(metricsHistory, ($h) =>
  $h.map((s) => ({ time: s.time, value: s.cpuAvg })),
);

export const ramSeries = derived(metricsHistory, ($h) =>
  $h.map((s) => ({ time: s.time, value: s.ramPct })),
);

export const netRxSeries = derived(metricsHistory, ($h) =>
  $h.map((s) => ({ time: s.time, value: s.netRx })),
);

export const netTxSeries = derived(metricsHistory, ($h) =>
  $h.map((s) => ({ time: s.time, value: s.netTx })),
);

export const swapSeries = derived(metricsHistory, ($h) =>
  $h.map((s) => ({ time: s.time, value: s.swapMb })),
);

export function _resetMetricsHistory(): void {
  metricsHistory.set([]);
}
