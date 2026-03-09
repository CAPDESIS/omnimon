import { cleanup, render, screen } from "@testing-library/svelte";
import { writable, derived } from "svelte/store";

import SystemMetricModal from "../SystemMetricModal.svelte";

const { mockFiltered, mockStats, mockMetricsHistory, mockCpuSeries, mockRamSeries, mockNetRxSeries, mockNetTxSeries, mockSwapSeries } = vi.hoisted(() => {
  const { writable, derived } = require("svelte/store") as typeof import("svelte/store");
  const metricsHistory = writable([]);
  return {
    mockFiltered: writable([]),
    mockStats: writable({
      ram_total_gb: 16,
      ram_used_pct: 42,
      swap_used_mb: 64,
      total_processes: 3,
      net_rx_bytes_per_sec: 2048,
      net_tx_bytes_per_sec: 1024,
    }),
    mockMetricsHistory: metricsHistory,
    mockCpuSeries: derived(metricsHistory, () => []),
    mockRamSeries: derived(metricsHistory, () => []),
    mockNetRxSeries: derived(metricsHistory, () => []),
    mockNetTxSeries: derived(metricsHistory, () => []),
    mockSwapSeries: derived(metricsHistory, () => []),
  };
});

vi.mock("../../stores/metricsHistory", () => ({
  metricsHistory: mockMetricsHistory,
  cpuSeries: mockCpuSeries,
  ramSeries: mockRamSeries,
  netRxSeries: mockNetRxSeries,
  netTxSeries: mockNetTxSeries,
  swapSeries: mockSwapSeries,
}));

vi.mock("../../stores/processes", () => ({
  filtered: mockFiltered,
  stats: mockStats,
}));

describe("SystemMetricModal", () => {
  afterEach(() => {
    cleanup();
  });

  it("renderiza sin errores", () => {
    render(SystemMetricModal, {
      props: {
        metric: "cpu",
        onclose: vi.fn(),
      },
    });

    expect(screen.getByRole("dialog", { name: "CPU" })).toBeInTheDocument();
    expect(screen.getByText("Deep Dive")).toBeInTheDocument();
    expect(screen.getByText(/Top processes/i)).toBeInTheDocument();
  });
});
