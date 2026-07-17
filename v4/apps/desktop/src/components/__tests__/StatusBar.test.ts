import { cleanup, render, screen } from "@testing-library/svelte";
import { writable } from "svelte/store";
import StatusBar from "../StatusBar.svelte";

const { mockStats, mockFiltered, mockProcesses, mockNetworkTelemetryStatus } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockStats: writable({
      ram_used_pct: 40,
      ram_total_gb: 16,
      swap_used_mb: 256,
      net_rx_bytes_per_sec: 1024,
      net_tx_bytes_per_sec: 2048,
    }),
    mockFiltered: writable([
      { pid: 101 },
      { pid: 102 },
    ]),
    mockProcesses: writable([
      { pid: 101, cpu_pct: 10.0, idle: false },
      { pid: 102, cpu_pct: 20.0, idle: true },
    ]),
    mockNetworkTelemetryStatus: writable({
      dpiActive: false,
    }),
  };
});

vi.mock("../../stores/processes", () => ({
  stats: mockStats,
  filtered: mockFiltered,
  processes: mockProcesses,
}));

vi.mock("../../stores/security", () => ({
  networkTelemetryStatus: mockNetworkTelemetryStatus,
}));

describe("StatusBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renderiza metricas basicas de RAM, CPU, Swap y Procesos", () => {
    render(StatusBar);

    expect(screen.getByText("RAM")).toBeInTheDocument();
    expect(screen.getByText("40% of 16GB")).toBeInTheDocument();

    expect(screen.getByText("CPU")).toBeInTheDocument();
    // avgCpu = (10 + 20) / 2 = 15.0%
    expect(screen.getByText("15.0%")).toBeInTheDocument();

    expect(screen.getByText("Swap")).toBeInTheDocument();
    expect(screen.getByText("256 MB")).toBeInTheDocument();

    expect(screen.getByText("Net")).toBeInTheDocument();
    expect(screen.getByText(/↓/)).toBeInTheDocument();
    expect(screen.getByText(/↑/)).toBeInTheDocument();

    expect(screen.getByText("Procs")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();

    expect(screen.getByText("Idle")).toBeInTheDocument();
    // idleCount is 1 (pid 102 is idle)
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("muestra la insignia DPI cuando esta activa", () => {
    mockNetworkTelemetryStatus.set({ dpiActive: true });
    render(StatusBar);

    expect(screen.getByText("DPI")).toBeInTheDocument();
    expect(screen.getByText("active")).toBeInTheDocument();
  });
});
