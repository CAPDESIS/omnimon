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
});
