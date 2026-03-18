import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";

import SystemMetricModal from "../SystemMetricModal.svelte";
import type { ProcessEntry } from "../../lib/types";
import {
  activeSeriesForMetric,
  defaultSortKey,
  getSparklineColor,
  metricSummaryLabel,
} from "../../lib/systemMetricModal";


const {
  mockFiltered,
  mockStats,
  mockMetricsHistory,
  mockCpuSeries,
  mockRamSeries,
  mockNetRxSeries,
  mockNetTxSeries,
  mockSwapSeries,
  mockFocusFirstFocusable,
  mockTrapFocus,
  mockLoadNetworkMap,
} = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockFiltered: writable<ProcessEntry[]>([]),
    mockStats: writable({
      cpu_usage_pct: 25,
      ram_total_gb: 16,
      ram_used_pct: 42,
      swap_used_mb: 64,
      total_processes: 3,
      net_rx_bytes_per_sec: 2048,
      net_tx_bytes_per_sec: 1024,
    }),
    mockMetricsHistory: writable([{ time: 1, cpuAvg: 20, ramPct: 40, netRx: 1000, netTx: 500, swapMb: 64, processCount: 3 }]),
    mockCpuSeries: writable([
      { time: 1, value: 10 },
      { time: 2, value: 40 },
      { time: 3, value: 20 },
    ]),
    mockRamSeries: writable([
      { time: 1, value: 35 },
      { time: 2, value: 45 },
    ]),
    mockNetRxSeries: writable([]),
    mockNetTxSeries: writable([]),
    mockSwapSeries: writable([
      { time: 1, value: 64 },
      { time: 2, value: 128 },
    ]),
    mockFocusFirstFocusable: vi.fn(),
    mockTrapFocus: vi.fn(),
    mockLoadNetworkMap: vi.fn(async () => ({ default: function NetworkMapStub() {} })),
  };
});

vi.mock("../../lib/systemMetricModal", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/systemMetricModal")>();
  return {
    ...actual,
    loadNetworkMap: mockLoadNetworkMap,
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

vi.mock("../../lib/focusTrap", () => ({
  focusFirstFocusable: mockFocusFirstFocusable,
  trapFocus: mockTrapFocus,
}));

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 1,
    name: "Chrome",
    exec_name: "/Applications/Chrome",
    exe_path: "/Applications/Chrome",
    bundle_id: null,
    icon_data_url: null,
    ram_mb: 500,
    cpu_pct: 20,
    disk_read_mb: 0,
    disk_write_mb: 0,
    net_rx_bytes_per_sec: 2048,
    net_tx_bytes_per_sec: 1024,
    energy_impact_score: 10,
    uptime: "1h",
    group: "Browser",
    group_key: "browser:chrome",
    group_identity_type: "normalized_name",
    grouped_name: "Chrome",
    process_count: 1,
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

describe("SystemMetricModal", () => {
  beforeEach(() => {
    mockFiltered.set([
      makeProc({ pid: 1, name: "Chrome", ram_mb: 500, cpu_pct: 20, uptime: "1h", state: "R" }),
      makeProc({ pid: 2, name: "Node", ram_mb: 900, cpu_pct: 60, uptime: "3h", state: "S", net_rx_bytes_per_sec: 10, net_tx_bytes_per_sec: 15 }),
      makeProc({ pid: 3, name: "Safari", ram_mb: 100, cpu_pct: 5, uptime: "30m", state: "I", net_rx_bytes_per_sec: 0, net_tx_bytes_per_sec: 0 }),
    ]);
    mockStats.set({
      cpu_usage_pct: 25,
      ram_total_gb: 16,
      ram_used_pct: 42,
      swap_used_mb: 64,
      total_processes: 3,
      net_rx_bytes_per_sec: 2048,
      net_tx_bytes_per_sec: 1024,
    });
    mockMetricsHistory.set([{ time: 1, cpuAvg: 20, ramPct: 40, netRx: 1000, netTx: 500, swapMb: 64, processCount: 3 }]);
    mockCpuSeries.set([
      { time: 1, value: 10 },
      { time: 2, value: 40 },
      { time: 3, value: 20 },
    ]);
    mockRamSeries.set([
      { time: 1, value: 35 },
      { time: 2, value: 45 },
    ]);
    mockSwapSeries.set([
      { time: 1, value: 64 },
      { time: 2, value: 128 },
    ]);
    mockFocusFirstFocusable.mockClear();
    mockTrapFocus.mockClear();
    mockLoadNetworkMap.mockReset();
    mockLoadNetworkMap.mockImplementation(async () => ({ default: function NetworkMapStub() {} }));
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renderiza sin errores con metricas de CPU", () => {
    render(SystemMetricModal, {
      props: {
        metric: "cpu",
        onclose: vi.fn(),
      },
    });

    expect(screen.getByRole("dialog", { name: "CPU" })).toBeInTheDocument();
    expect(screen.getByText("Deep Dive")).toBeInTheDocument();
    expect(screen.getByText(/Now 20.0% - Avg 23.3% - Max 40.0%/)).toBeInTheDocument();
    expect(document.querySelector(".chart-container")).toBeInTheDocument();
  });

  it("ordena filas al hacer click en encabezados", async () => {
    render(SystemMetricModal, {
      props: { metric: "cpu", onclose: vi.fn() },
    });

    const before = screen.getAllByRole("row").slice(1).map((row) => row.textContent);
    expect(before[0]).toContain("Node");

    await fireEvent.click(screen.getByRole("button", { name: /Name/i }));
    const afterDesc = screen.getAllByRole("row").slice(1).map((row) => row.textContent);
    expect(afterDesc[0]).toContain("Safari");

    await fireEvent.click(screen.getByRole("button", { name: /Name/i }));
    const afterAsc = screen.getAllByRole("row").slice(1).map((row) => row.textContent);
    expect(afterAsc[0]).toContain("Chrome");
  });

  it("muestra boton show more cuando hay mas procesos que el limite", async () => {
    mockFiltered.set(Array.from({ length: 35 }, (_, index) => makeProc({ pid: index + 1, name: `Proc ${index + 1}`, ram_mb: 1000 - index })));

    render(SystemMetricModal, {
      props: { metric: "ram", onclose: vi.fn() },
    });

    const button = screen.getByRole("button", { name: /Show more/i });
    expect(button).toHaveTextContent("Show more (5 remaining)");
    await fireEvent.click(button);

    await waitFor(() => {
      expect(screen.getAllByRole("row")).toHaveLength(36);
    });
  });

  it("renderiza resumen de red y usa el modo recibido", async () => {
    render(SystemMetricModal, {
      props: { metric: "network", mode: "basic", onclose: vi.fn() },
    });

    expect(screen.getByText("RX")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s")).toBeInTheDocument();
    expect(screen.getByText("1.0 KB/s")).toBeInTheDocument();

    await waitFor(() => {
      expect(document.body.textContent).toContain("Processes");
    });
  });

  it("usa RAM como orden por defecto para processes", () => {
    expect(defaultSortKey("processes")).toBe("ram");

    render(SystemMetricModal, {
      props: { metric: "processes", onclose: vi.fn() },
    });

    const rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Node");
    expect(rows[1]).toContain("Chrome");
  });

  it("usa color accent para sparklines que no son cpu o ram", () => {
    expect(getSparklineColor("network", [{ value: 10 }])).toBe("var(--accent)");
  });

  it("usa color default cuando la serie esta vacia", () => {
    expect(getSparklineColor("cpu", [])).toBe("var(--accent)");
  });

  it("devuelve serie vacia para network y processes", () => {
    const series = {
      cpuSeries: [{ time: 1, value: 10 }],
      ramSeries: [{ time: 1, value: 20 }],
      swapSeries: [{ time: 1, value: 30 }],
    };

    expect(activeSeriesForMetric("network", series)).toEqual([]);
    expect(activeSeriesForMetric("processes", series)).toEqual([]);
  });

  it("muestra etiqueta correcta para network y processes", () => {
    expect(
      metricSummaryLabel("network", {
        cpuSeries: [],
        ramSeries: [],
        swapSeries: [],
        totalProcesses: 3,
      }),
    ).toBe("3 visible");
    expect(
      metricSummaryLabel("processes", {
        cpuSeries: [],
        ramSeries: [],
        swapSeries: [],
        totalProcesses: 3,
      }),
    ).toBe("3 visible");
  });

  it("maneja rejection del import dinamico de network map", async () => {
    mockFiltered.set([]);
    mockLoadNetworkMap.mockRejectedValueOnce(new Error("import failed"));

    render(SystemMetricModal, {
      props: { metric: "network", onclose: vi.fn() },
    });

    await waitFor(() => {
      expect(screen.getByText("Failed to load network map.")).toBeInTheDocument();
    });
  });

  it("ordena por PID, Net, State y Uptime", async () => {
    render(SystemMetricModal, {
      props: { metric: "ram", onclose: vi.fn() },
    });

    await fireEvent.click(screen.getByRole("button", { name: /PID/i }));
    let rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Safari");

    await fireEvent.click(screen.getByRole("button", { name: /Net/i }));
    rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Chrome");

    await fireEvent.click(screen.getByRole("button", { name: /State/i }));
    rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Node");

    await fireEvent.click(screen.getByRole("button", { name: /State/i }));
    rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Safari");

    await fireEvent.click(screen.getByRole("button", { name: /Uptime/i }));
    rows = screen.getAllByRole("row").slice(1).map((row) => row.textContent ?? "");
    expect(rows[0]).toContain("Node");
  });

  it("muestra fallbacks de estado y uptime cuando faltan", () => {
    mockFiltered.set([
      makeProc({ pid: 9, name: "Broken", state: null as unknown as string, uptime: null as unknown as string }),
    ]);

    render(SystemMetricModal, {
      props: { metric: "ram", onclose: vi.fn() },
    });

    const placeholders = screen.getAllByText("—");
    expect(placeholders.length).toBeGreaterThanOrEqual(2);
  });

  it("muestra metricas de red en MB/s cuando el throughput es alto", () => {
    mockStats.set({
      cpu_usage_pct: 25,
      ram_total_gb: 16,
      ram_used_pct: 42,
      swap_used_mb: 64,
      total_processes: 3,
      net_rx_bytes_per_sec: 3 * 1024 * 1024,
      net_tx_bytes_per_sec: 2 * 1024 * 1024,
    });

    render(SystemMetricModal, {
      props: { metric: "network", onclose: vi.fn() },
    });

    expect(screen.getByText("3.00 MB/s")).toBeInTheDocument();
    expect(screen.getByText("2.00 MB/s")).toBeInTheDocument();
  });

  it("usa 0 cuando faltan stats de red", () => {
    mockStats.set({
      cpu_usage_pct: 25,
      ram_total_gb: 16,
      ram_used_pct: 42,
      swap_used_mb: 64,
      total_processes: 0,
      net_rx_bytes_per_sec: undefined,
      net_tx_bytes_per_sec: undefined,
    } as unknown as { cpu_usage_pct: number; ram_total_gb: number; ram_used_pct: number; swap_used_mb: number; total_processes: number; net_rx_bytes_per_sec: number; net_tx_bytes_per_sec: number });

    render(SystemMetricModal, {
      props: { metric: "network", onclose: vi.fn() },
    });

    expect(screen.getAllByText("0 B/s").length).toBeGreaterThanOrEqual(2);
  });

  it("no renderiza chart cuando la serie tiene menos de dos puntos", () => {
    mockCpuSeries.set([{ time: 1, value: 10 }]);

    render(SystemMetricModal, {
      props: { metric: "cpu", onclose: vi.fn() },
    });

    expect(document.querySelector(".chart-container")).toBeNull();
  });

  it("no muestra boton show more cuando filtered no supera el limite", () => {
    mockFiltered.set(Array.from({ length: 30 }, (_, index) => makeProc({ pid: index + 1, name: `Proc ${index + 1}` })));

    render(SystemMetricModal, {
      props: { metric: "ram", onclose: vi.fn() },
    });

    expect(screen.queryByRole("button", { name: /Show more/i })).not.toBeInTheDocument();
  });

  it("cierra con escape y atrapa tab focus", async () => {
    const onclose = vi.fn();
    render(SystemMetricModal, {
      props: { metric: "swap", onclose },
    });

    const dialog = screen.getByRole("dialog", { name: "Swap" });
    // Tab must be fired on an element inside modalEl (child of dialog),
    // since the onkeydown={closeOnEscape} handler is on the inner div, not on the dialog itself.
    const closeBtn = screen.getByRole("button", { name: "Close" });
    await fireEvent.keyDown(closeBtn, { key: "Tab" });
    expect(mockTrapFocus).toHaveBeenCalled();

    await fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it("cierra con backdrop y boton close", async () => {
    const onclose = vi.fn();
    render(SystemMetricModal, {
      props: { metric: "cpu", onclose },
    });

    await fireEvent.mouseDown(document.querySelector(".backdrop") as HTMLElement);
    await fireEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(onclose).toHaveBeenCalledTimes(2);
  });

  it("tolera series vacias y datos malformados en procesos", () => {
    mockCpuSeries.set([]);
    mockFiltered.set([
      makeProc({ pid: 11, name: "Broken", state: "", uptime: "", net_rx_bytes_per_sec: 0, net_tx_bytes_per_sec: 0 }),
    ]);

    render(SystemMetricModal, {
      props: { metric: "cpu", onclose: vi.fn() },
    });

    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(2);
    expect(document.querySelector(".chart-container")).toBeNull();
  });

  it("intenta enfocar el primer elemento al montar", () => {
    render(SystemMetricModal, {
      props: { metric: "cpu", onclose: vi.fn() },
    });

    expect(mockFocusFirstFocusable).toHaveBeenCalled();
  });
});
