import { beforeEach, describe, expect, it, vi } from "vitest";

const mockListen = vi.fn();

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockListen,
}));

describe("network store", () => {
  beforeEach(() => {
    vi.resetModules();
    mockListen.mockReset();
  });

  it("starts with default state", async () => {
    const network = await import("../network.svelte");
    const state = network.getNetworkState();

    expect(state.snapshot).toBeNull();
    expect(state.history).toEqual([]);
    expect(state.filter.protocol).toBe("");
    expect(state.isCapturing).toBe(true);
  });

  it("appends snapshots when listener receives updates", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    mockListen.mockImplementation(async (_eventName: string, cb: (event: { payload: unknown }) => void) => {
      handler = cb;
      return vi.fn();
    });

    const network = await import("../network.svelte");
    await network.initNetworkListener();

    handler?.({
      payload: {
        timestamp: 10,
        total_bytes_per_sec_up: 1_024,
        total_bytes_per_sec_down: 2_048,
        active_connections: 1,
        processes_with_network: 1,
        connections: [
          {
            process_id: 5,
            process_name: "Chrome",
            protocol: "TCP",
            local_address: "127.0.0.1",
            local_port: 50000,
            remote_address: "8.8.8.8",
            remote_port: 443,
            remote_hostname: "dns.google",
            state: "ESTABLISHED",
            bytes_up: 2_048,
            bytes_down: 4_096,
            bytes_per_sec_up: 1_024,
            bytes_per_sec_down: 2_048,
          },
        ],
      },
    });

    const state = network.getNetworkState();
    expect(state.snapshot?.active_connections).toBe(1);
    expect(state.history).toHaveLength(1);
    expect(network.getTotalUp()).toBe(1_024);
    expect(network.getTotalDown()).toBe(2_048);
    expect(network.getFilteredConnections()).toHaveLength(1);
    expect(network.getPerProcessSummary()[0].name).toBe("Chrome");
  });

  it("applies filters to derived connections and summaries", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    mockListen.mockImplementation(async (_eventName: string, cb: (event: { payload: unknown }) => void) => {
      handler = cb;
      return vi.fn();
    });

    const network = await import("../network.svelte");
    await network.initNetworkListener();

    handler?.({
      payload: {
        timestamp: 10,
        total_bytes_per_sec_up: 3_000,
        total_bytes_per_sec_down: 4_000,
        active_connections: 2,
        processes_with_network: 2,
        connections: [
          {
            process_id: 5,
            process_name: "Chrome",
            protocol: "TCP",
            local_address: "127.0.0.1",
            local_port: 50000,
            remote_address: "8.8.8.8",
            remote_port: 443,
            remote_hostname: "dns.google",
            state: "ESTABLISHED",
            bytes_up: 2_048,
            bytes_down: 4_096,
            bytes_per_sec_up: 2_048,
            bytes_per_sec_down: 2_048,
          },
          {
            process_id: 6,
            process_name: "Firefox",
            protocol: "UDP",
            local_address: "127.0.0.1",
            local_port: 51000,
            remote_address: "127.0.0.1",
            remote_port: 53,
            remote_hostname: "localhost",
            state: "LISTEN",
            bytes_up: 512,
            bytes_down: 256,
            bytes_per_sec_up: 128,
            bytes_per_sec_down: 128,
          },
        ],
      },
    });

    const state = network.getNetworkState();
    state.filter.protocol = "TCP";
    state.filter.process = "chrome";
    state.filter.host = "google";
    state.filter.hideLocalhost = true;
    state.filter.onlyEstablished = true;
    state.filter.minSpeed = 1;

    expect(network.getFilteredConnections()).toHaveLength(1);
    expect(network.getFilteredConnections()[0].process_name).toBe("Chrome");
    expect(network.getPerProcessSummary()).toHaveLength(1);
    expect(network.getPerProcessSummary()[0].connectionsCount).toBe(1);
  });

  it("defaultFilter returns expected defaults", async () => {
    const network = await import("../network.svelte");
    expect(network.defaultFilter()).toEqual({
      protocol: "",
      process: "",
      host: "",
      hideLocalhost: false,
      onlyEstablished: false,
      minSpeed: 0,
    });
  });

  it("returns zero totals and empty results when there is no snapshot", async () => {
    const network = await import("../network.svelte");
    expect(network.getFilteredConnections()).toEqual([]);
    expect(network.getPerProcessSummary()).toEqual([]);
    expect(network.getTotalUp()).toBe(0);
    expect(network.getTotalDown()).toBe(0);
  });

  it("filters by remote address when hostname is empty and groups unknown process names", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    mockListen.mockImplementation(async (_eventName: string, cb: (event: { payload: unknown }) => void) => {
      handler = cb;
      return vi.fn();
    });

    const network = await import("../network.svelte");
    await network.initNetworkListener();

    handler?.({
      payload: {
        timestamp: 11,
        total_bytes_per_sec_up: 2048,
        total_bytes_per_sec_down: 4096,
        active_connections: 2,
        processes_with_network: 2,
        connections: [
          {
            process_id: 1,
            process_name: "",
            protocol: "TCP",
            local_address: "10.0.0.2",
            local_port: 50001,
            remote_address: "203.0.113.10",
            remote_port: 443,
            remote_hostname: "",
            state: "ESTABLISHED",
            bytes_up: 1024,
            bytes_down: 2048,
            bytes_per_sec_up: 1024,
            bytes_per_sec_down: 2048,
          },
          {
            process_id: 2,
            process_name: "Worker",
            protocol: "TCP",
            local_address: "10.0.0.3",
            local_port: 50002,
            remote_address: "198.51.100.2",
            remote_port: 8080,
            remote_hostname: "api.service.local",
            state: "LISTEN",
            bytes_up: 50,
            bytes_down: 40,
            bytes_per_sec_up: 50,
            bytes_per_sec_down: 40,
          },
        ],
      },
    });

    const state = network.getNetworkState();
    state.filter.host = "203.0.113";
    expect(network.getFilteredConnections()).toHaveLength(1);
    expect(network.getPerProcessSummary()[0].name).toBe("Unknown");
    expect(network.getPerProcessSummary()[0].topDest).toBe("203.0.113.10");
  });

  it("limits history to the last 60 snapshots", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    mockListen.mockImplementation(async (_eventName: string, cb: (event: { payload: unknown }) => void) => {
      handler = cb;
      return vi.fn();
    });

    const network = await import("../network.svelte");
    await network.initNetworkListener();

    for (let index = 0; index < 65; index += 1) {
      handler?.({
        payload: {
          timestamp: index,
          total_bytes_per_sec_up: index,
          total_bytes_per_sec_down: index,
          active_connections: 0,
          processes_with_network: 0,
          connections: [],
        },
      });
    }

    const state = network.getNetworkState();
    expect(state.history).toHaveLength(60);
    expect(state.history[0]?.timestamp).toBe(5);
    expect(state.snapshot?.timestamp).toBe(64);
  });
});
