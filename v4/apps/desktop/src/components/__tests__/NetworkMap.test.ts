import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import NetworkMap from "../NetworkMap.svelte";
import type { NetworkConnection } from "../../lib/types";

vi.mock("lightweight-charts", () => ({
  createChart: () => ({
    addSeries: () => ({
      setData: vi.fn(),
      update: vi.fn(),
    }),
    timeScale: () => ({ fitContent: vi.fn() }),
    applyOptions: vi.fn(),
    remove: vi.fn(),
  }),
  AreaSeries: {},
}));

const { mockNetworkConnections, mockNetworkTelemetryStatus } = vi.hoisted(() => {
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
  };
});

vi.mock("../../stores/security", () => ({
  networkConnections: mockNetworkConnections,
  networkTelemetryStatus: mockNetworkTelemetryStatus,
}));

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

  it("shows traffic tab when throughput exists even without connections", async () => {
    mockNetworkTelemetryStatus.set({
      captureBackend: "watcher",
      dpiActive: false,
      usingFallback: false,
      lastUpdated: null,
      totalRxBytesPerSec: 4096,
      totalTxBytesPerSec: 2048,
    });
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Traffic"));
    await waitFor(() => expect(screen.getByText(/Inbound/)).toBeInTheDocument());
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

});
