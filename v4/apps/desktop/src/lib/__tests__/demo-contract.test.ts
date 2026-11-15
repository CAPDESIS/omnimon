import { describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import fixture from "../../../src-tauri/src/demo-data.json";
import { ipcGetMetrics, ipcGetNetworkData } from "../ipc";
import { get } from "svelte/store";
import { refreshNetworkConnections, networkConnections, _resetSecurity } from "../../stores/security";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("native screenshot demo fixture", () => {
  it("passes the real frontend IPC validators without reading host data", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(fixture.metrics).mockResolvedValueOnce(fixture.network);
    const metrics = await ipcGetMetrics();
    const network = await ipcGetNetworkData();
    expect(metrics.processes.length).toBeGreaterThanOrEqual(18);
    expect(metrics.processes.length).toBeLessThanOrEqual(24);
    expect(metrics.stats.cpu_usage_pct).toBeCloseTo(metrics.processes.reduce((n,p) => n+p.cpu_pct,0)/metrics.processes.length, 2);
    expect(metrics.stats.total_processes).toBe(metrics.processes.length);
    expect(metrics.processes.some(p => p.idle)).toBe(true);
    expect(metrics.stats.ram_used_pct).toBeCloseTo(metrics.processes.reduce((n,p) => n+p.ram_mb,0)/32768*100, 2);
    for (const key of ["net_rx_bytes_per_sec", "net_tx_bytes_per_sec"] as const) {
      expect(metrics.stats[key]).toBe(metrics.processes.reduce((n,p) => n+p[key],0));
      expect(network[key]).toBe(metrics.stats[key]);
    }
    _resetSecurity();
    vi.mocked(invoke).mockResolvedValueOnce(fixture.network);
    await refreshNetworkConnections([], []);
    expect(get(networkConnections)[0].process_name).toMatch(/^pid:/);
    vi.mocked(invoke).mockResolvedValueOnce(fixture.network);
    await refreshNetworkConnections(metrics.processes, []);
    expect(get(networkConnections).map(c => c.process_name)).toContain("Safari");
    expect(get(networkConnections).map(c => c.process_name)).toContain("node");
    expect(get(networkConnections).every(c => !c.process_name.startsWith("pid:"))).toBe(true);
    expect(network.dpi_active).toBe(false);
    expect(network.recent_connections).toHaveLength(8);
    for (const c of network.recent_connections) {
      expect(metrics.processes.find(p => p.pid === c.pid)?.name).toBeTruthy();
      expect(["198.51.100.20", "203.0.113.30", "192.0.2.80"]).toContain(c.dst_ip);
    }
  });
});
