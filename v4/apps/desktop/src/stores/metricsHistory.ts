import { writable } from "svelte/store";
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

/**
 * Incremental series stores — appended to in pushMetrics() instead of
 * re-deriving the entire array on every update. This avoids creating
 * 5 new arrays (5 x 300 items = 1500 objects) every polling cycle.
 */
export const cpuSeries = writable<MetricPoint[]>([]);
export const ramSeries = writable<MetricPoint[]>([]);
export const netRxSeries = writable<MetricPoint[]>([]);
export const netTxSeries = writable<MetricPoint[]>([]);
export const swapSeries = writable<MetricPoint[]>([]);

/** Push a new stats snapshot into the rolling history buffer. */
export function pushMetrics(stats: SystemStats, cpuAvg: number): void {
  const snap: MetricsSnapshot = {
    time: Math.floor(Date.now() / 1000),
    cpuAvg,
    ramPct: stats.ram_used_pct,
    netRx: stats.net_rx_bytes_per_sec,
    netTx: stats.net_tx_bytes_per_sec,
    swapMb: stats.swap_used_mb,
    processCount: stats.total_processes,
  };

  metricsHistory.update((history) => {
    history.push(snap);
    if (history.length > MAX_HISTORY) history.splice(0, history.length - MAX_HISTORY);
    return history;
  });

  // Append the new point to each series and trim to MAX_HISTORY
  const point = (value: number): MetricPoint => ({ time: snap.time, value });

  cpuSeries.update((s) => {
    s.push(point(cpuAvg));
    if (s.length > MAX_HISTORY) s.splice(0, s.length - MAX_HISTORY);
    return s;
  });
  ramSeries.update((s) => {
    s.push(point(stats.ram_used_pct));
    if (s.length > MAX_HISTORY) s.splice(0, s.length - MAX_HISTORY);
    return s;
  });
  netRxSeries.update((s) => {
    s.push(point(stats.net_rx_bytes_per_sec));
    if (s.length > MAX_HISTORY) s.splice(0, s.length - MAX_HISTORY);
    return s;
  });
  netTxSeries.update((s) => {
    s.push(point(stats.net_tx_bytes_per_sec));
    if (s.length > MAX_HISTORY) s.splice(0, s.length - MAX_HISTORY);
    return s;
  });
  swapSeries.update((s) => {
    s.push(point(stats.swap_used_mb));
    if (s.length > MAX_HISTORY) s.splice(0, s.length - MAX_HISTORY);
    return s;
  });
}

export function _resetMetricsHistory(): void {
  metricsHistory.set([]);
  cpuSeries.set([]);
  ramSeries.set([]);
  netRxSeries.set([]);
  netTxSeries.set([]);
  swapSeries.set([]);
}
