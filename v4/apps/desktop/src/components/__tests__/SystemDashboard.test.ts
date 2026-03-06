import { render, screen } from "@testing-library/svelte";
import SystemDashboard from "../SystemDashboard.svelte";

const { mockStats, mockProcesses, mockHistory } = vi.hoisted(() => {
  const { writable } = require("svelte/store");
  return {
    mockStats: writable(null),
    mockProcesses: writable([]),
    mockHistory: writable([]),
  };
});

vi.mock("../../stores/processes", () => ({
  stats: mockStats,
  processes: mockProcesses,
}));

vi.mock("../../stores/metricsHistory", () => ({
  metricsHistory: mockHistory,
}));

function makeProc(overrides?: Record<string, unknown>) {
  return {
    pid: 1, name: "A", cpu_pct: 10, idle: false,
    ram_mb: 100, exec_name: "a", uptime: "1h", group: "Other",
    is_system: false, state: "R",
    ...overrides,
  };
}

function makeStats(overrides?: Record<string, unknown>) {
  return {
    ram_total_gb: 16, ram_used_pct: 50, swap_used_mb: 0,
    total_processes: 1, net_rx_bytes_per_sec: 0, net_tx_bytes_per_sec: 0,
    ...overrides,
  };
}

describe("SystemDashboard", () => {
  beforeEach(() => {
    mockStats.set(null);
    mockProcesses.set([]);
    mockHistory.set([]);
  });

  it("renders nothing when stats is null", () => {
    render(SystemDashboard);
    expect(screen.queryByText("CPU")).not.toBeInTheDocument();
  });

  it("renders nothing when collapsed", () => {
    mockStats.set(makeStats());
    render(SystemDashboard, { props: { collapsed: true } });
    expect(screen.queryByText("CPU")).not.toBeInTheDocument();
  });

  it("renders CPU, RAM, Network cards when stats present", () => {
    mockStats.set(makeStats());
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    expect(screen.getByText("CPU")).toBeInTheDocument();
    expect(screen.getByText("RAM")).toBeInTheDocument();
    expect(screen.getByText("Network")).toBeInTheDocument();
  });

  it("shows average CPU percentage", () => {
    mockStats.set(makeStats({ total_processes: 2 }));
    mockProcesses.set([
      makeProc({ pid: 1, cpu_pct: 20 }),
      makeProc({ pid: 2, cpu_pct: 40 }),
    ]);
    render(SystemDashboard);
    expect(screen.getByText("30.0%")).toBeInTheDocument();
  });

  it("shows RAM percentage and total", () => {
    mockStats.set(makeStats({ ram_total_gb: 32, ram_used_pct: 65 }));
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    expect(screen.getByText("65%")).toBeInTheDocument();
    expect(screen.getByText("/ 32GB")).toBeInTheDocument();
  });

  it("formats network rates as MB/s", () => {
    mockStats.set(makeStats({ net_rx_bytes_per_sec: 1_500_000, net_tx_bytes_per_sec: 2_100_000 }));
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    expect(screen.getByText("1.4 MB/s")).toBeInTheDocument();
    expect(screen.getByText("2.0 MB/s")).toBeInTheDocument();
  });

  it("formats network rates as KB/s", () => {
    mockStats.set(makeStats({ net_rx_bytes_per_sec: 5120, net_tx_bytes_per_sec: 2048 }));
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    expect(screen.getByText("5.0 KB/s")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s")).toBeInTheDocument();
  });

  it("formats network rates as B/s", () => {
    mockStats.set(makeStats({ net_rx_bytes_per_sec: 500, net_tx_bytes_per_sec: 100 }));
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    expect(screen.getByText("500 B/s")).toBeInTheDocument();
    expect(screen.getByText("100 B/s")).toBeInTheDocument();
  });

  it("shows swap, process count, and idle count", () => {
    mockStats.set(makeStats({ swap_used_mb: 256, total_processes: 42 }));
    mockProcesses.set([
      makeProc({ pid: 1, idle: false }),
      makeProc({ pid: 2, idle: true }),
    ]);
    render(SystemDashboard);
    expect(screen.getByText("256 MB")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument(); // 1 idle
  });

  it("renders canvas elements for charts", () => {
    mockStats.set(makeStats());
    mockProcesses.set([makeProc()]);
    render(SystemDashboard);
    const canvases = document.querySelectorAll("canvas");
    expect(canvases.length).toBe(3);
  });
});
