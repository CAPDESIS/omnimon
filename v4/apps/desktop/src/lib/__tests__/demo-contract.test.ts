import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import fixture from "../../../src-tauri/src/demo-data.json";
import { ipcGetMetrics, ipcGetNetworkData } from "../ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("native screenshot demo fixture", () => {
  it("passes the real frontend IPC validators without reading host data", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(fixture.metrics).mockResolvedValueOnce(fixture.network);
    const metrics = await ipcGetMetrics();
    const network = await ipcGetNetworkData();
    expect(metrics.processes.map((p) => p.name)).toEqual(["Safari", "Xcode", "Docker", "node", "Terminal", "Finder"]);
    expect(network.dpi_active).toBe(false);
    expect(network.recent_connections.map((c) => c.dst_ip)).toEqual(["198.51.100.20", "203.0.113.30"]);
  });
});
