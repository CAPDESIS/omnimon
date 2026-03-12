import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  metricsHistory,
  pushMetrics,
  cpuSeries,
  ramSeries,
  netRxSeries,
  netTxSeries,
  swapSeries,
  _resetMetricsHistory,
} from "../metricsHistory";
import type { SystemStats } from "../../lib/types";

function makeStats(overrides?: Partial<SystemStats>): SystemStats {
  return {
    cpu_usage_pct: 25,
    ram_total_gb: 16,
    ram_used_pct: 45,
    swap_used_mb: 128,
    total_processes: 200,
    net_rx_bytes_per_sec: 5000,
    net_tx_bytes_per_sec: 2000,
    ...overrides,
  };
}

describe("metricsHistory", () => {
  beforeEach(() => {
    _resetMetricsHistory();
  });

  it("starts empty", () => {
    expect(get(metricsHistory)).toEqual([]);
  });

  it("pushMetrics adds a snapshot", () => {
    pushMetrics(makeStats(), 15.5);
    const h = get(metricsHistory);
    expect(h).toHaveLength(1);
    expect(h[0].cpuAvg).toBe(15.5);
    expect(h[0].ramPct).toBe(45);
    expect(h[0].netRx).toBe(5000);
    expect(h[0].netTx).toBe(2000);
    expect(h[0].swapMb).toBe(128);
    expect(h[0].processCount).toBe(200);
    expect(h[0].time).toBeGreaterThan(0);
  });

  it("accumulates multiple snapshots", () => {
    pushMetrics(makeStats({ ram_used_pct: 40 }), 10);
    pushMetrics(makeStats({ ram_used_pct: 50 }), 20);
    pushMetrics(makeStats({ ram_used_pct: 60 }), 30);
    expect(get(metricsHistory)).toHaveLength(3);
  });

  it("caps history at 300 entries", () => {
    for (let i = 0; i < 310; i++) {
      pushMetrics(makeStats(), i);
    }
    expect(get(metricsHistory)).toHaveLength(300);
    // Earliest entries should have been pruned
    expect(get(metricsHistory)[0].cpuAvg).toBe(10);
  });

  it("derived series extract correct values", () => {
    pushMetrics(makeStats({ ram_used_pct: 42, net_rx_bytes_per_sec: 100, net_tx_bytes_per_sec: 50, swap_used_mb: 64 }), 7.7);

    expect(get(cpuSeries)).toHaveLength(1);
    expect(get(cpuSeries)[0].value).toBe(7.7);

    expect(get(ramSeries)[0].value).toBe(42);
    expect(get(netRxSeries)[0].value).toBe(100);
    expect(get(netTxSeries)[0].value).toBe(50);
    expect(get(swapSeries)[0].value).toBe(64);
  });
});
