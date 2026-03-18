import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import type { NetworkConnection } from "../../lib/types";

vi.mock("lightweight-charts", async () => {
  const createChart = vi.fn(() => ({
    addSeries: () => ({
      setData: vi.fn(),
      update: vi.fn(),
    }),
    timeScale: () => ({ fitContent: vi.fn() }),
    applyOptions: vi.fn(),
    remove: vi.fn(),
  }));

  return {
    createChart,
    AreaSeries: {},
  };
});

const {
  mockNetworkConnections,
  mockNetworkTelemetryStatus,
  mockMetricsHistory,
  mockTheme,
} = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockNetworkConnections: // @ts-ignore
    writable<NetworkConnection[]>([]),
    mockNetworkTelemetryStatus: writable({
      captureBackend: "watcher",
      dpiActive: false,
      usingFallback: false,
      lastUpdated: null,
      totalRxBytesPerSec: 0,
      totalTxBytesPerSec: 0,
    }),
    mockMetricsHistory: writable([]),
    mockTheme: writable("dark"),
  };
});

vi.mock("../../stores/security", () => ({
  networkConnections: mockNetworkConnections,
  networkTelemetryStatus: mockNetworkTelemetryStatus,
}));

vi.mock("../../stores/metricsHistory", () => ({
  metricsHistory: mockMetricsHistory,
}));

vi.mock("../../stores/preferences", () => ({
  theme: mockTheme,
  networkAlertRules: (() => {
    const { writable } = require("svelte/store") as typeof import("svelte/store");
    return writable([]);
  })(),
}));

import NetworkMap from "../NetworkMap.svelte";

function makeConn(overrides?: Partial<NetworkConnection>): NetworkConnection {
  return {
    pid: 1,
    process_name: "Chrome",
    remote_addr: "example.com",
    remote_port: 443,
    protocol: "tcp",
    direction: "outbound",
    bytes_sent: 0,
    bytes_recv: 0,
    state: "ESTABLISHED",
    ...overrides,
  };
}

describe("NetworkMap", () => {
  beforeEach(() => {
    mockNetworkConnections.set([]);
    mockMetricsHistory.set([]);
    mockTheme.set("dark");
    mockNetworkTelemetryStatus.set({
      captureBackend: "watcher",
      dpiActive: false,
      usingFallback: false,
      lastUpdated: null,
      totalRxBytesPerSec: 0,
      totalTxBytesPerSec: 0,
    });
  });

  it("renders nothing when no connections", () => {
    render(NetworkMap);
    expect(screen.queryByText("Chrome")).not.toBeInTheDocument();
  });

  it("shows content directly when connections exist", () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    // Component starts expanded — shows process names directly
    expect(screen.getByText("Chrome")).toBeInTheDocument();
  });

  it("displays summary cards with connection and process counts", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", pid: 1, remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", pid: 1, remote_addr: "youtube.com" }),
      makeConn({ process_name: "Firefox", pid: 2, remote_addr: "mozilla.org" }),
    ]);
    render(NetworkMap);
    expect(screen.getByText("Live throughput")).toBeInTheDocument();
    expect(screen.getByText("Active hosts")).toBeInTheDocument();
  });

  it("shows process names and domains directly", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
    ]);
    render(NetworkMap);
    expect(screen.getByText("Chrome")).toBeInTheDocument();
    expect(screen.getByText("google.com:443")).toBeInTheDocument();
  });

  it("groups connections by process", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", remote_addr: "youtube.com" }),
      makeConn({ process_name: "Firefox", pid: 2, remote_addr: "mozilla.org" }),
    ]);
    render(NetworkMap);
    const procNames = screen.getAllByText(/^(Chrome|Firefox)$/);
    expect(procNames[0].textContent).toBe("Chrome");
  });

  it("shows connection count per process", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", remote_addr: "youtube.com" }),
    ]);
    render(NetworkMap);
    expect(document.querySelector(".netmap-list .proc-count")?.textContent).toBe("2");
  });

  it("shows +N when more than 5 domains", async () => {
    const conns = [];
    for (let i = 0; i < 7; i++) {
      conns.push(makeConn({
        process_name: "Chrome",
        remote_addr: `domain${i}.com`,
      }));
    }
    mockNetworkConnections.set(conns);
    render(NetworkMap);
    await waitFor(() => expect(screen.getByText("+2")).toBeInTheDocument());
  });

  it("deduplicates same domain:port connections", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443 }),
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443 }),
    ]);
    render(NetworkMap);
    const chips = screen.getAllByText("google.com:443");
    expect(chips).toHaveLength(1);
  });

  it("renders canvas element", () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    const canvas = document.querySelector("canvas");
    expect(canvas).toBeInTheDocument();
  });

  it("shows summary cards", () => {
    mockNetworkConnections.set([makeConn({ remote_addr: "google.com" })]);
    render(NetworkMap);
    expect(screen.getByText("Live throughput")).toBeInTheDocument();
    expect(screen.getByText("Active hosts")).toBeInTheDocument();
  });

  it("basic mode hides advanced tabs and sidebar", () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap, { props: { mode: "basic" } });
    expect(screen.queryByText("Connections")).not.toBeInTheDocument();
    expect(screen.queryByText("Traffic")).not.toBeInTheDocument();
    expect(screen.getByText(/focused on the map/i)).toBeInTheDocument();
  });

  it("muestra overflow count en tabla y permite filtrar conexiones", async () => {
    mockNetworkConnections.set(
      Array.from({ length: 55 }, (_, index) =>
        makeConn({
          pid: index + 1,
          process_name: index < 40 ? "Chrome" : "Firefox",
          remote_addr: `host-${index}.example.com`,
          remote_port: index % 2 === 0 ? 443 : 8443,
        }),
      ),
    );

    render(NetworkMap, { props: { filter: "firefox" } });
    await fireEvent.click(screen.getByText("Connections"));

    await waitFor(() => {
      expect(screen.getByText(/showing 15 of 55/i)).toBeInTheDocument();
    });
    expect(screen.getAllByText("Firefox").length).toBeGreaterThan(0);
    expect(screen.queryByText("Chrome")).not.toBeInTheDocument();
  });

  it("muestra warning de heavy downloaders y fallback backend", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Dropbox", bytes_recv: 60 * 1024 * 1024, remote_addr: "sync.example.com" }),
    ]);
    mockNetworkTelemetryStatus.set({
      captureBackend: "fallback",
      dpiActive: false,
      usingFallback: true,
      lastUpdated: null,
      totalRxBytesPerSec: 0,
      totalTxBytesPerSec: 0,
    });

    render(NetworkMap);

    expect(screen.getByText(/1 process\(es\) consuming high bandwidth/i)).toBeInTheDocument();
    expect(screen.getByText(/showing browser-tab fallback because live socket telemetry/i)).toBeInTheDocument();
  });

  it("muestra detalle de conexion al hacer click en un host", async () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443, bytes_recv: 2048, bytes_sent: 1024 }),
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443, protocol: "udp", bytes_recv: 512 }),
    ]);

    render(NetworkMap);

    await fireEvent.click(screen.getByText("google.com:443"));

    expect(screen.getByText("Processes:")).toBeInTheDocument();
    expect(screen.getAllByText("Chrome").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Close" })).toBeInTheDocument();
  });

});
