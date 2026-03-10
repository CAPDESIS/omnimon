import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import {
  securityMap,
  networkConnections,
  networkTelemetryStatus,
  flaggedPids,
  totalFindings,
  refreshSecurityAnalysis,
  refreshNetworkConnections,
  getSecurityForPid,
  severityRank,
  severityColor,
  _resetSecurity,
} from "../security";
import type { ProcessEntry } from "../../lib/types";

vi.mock("../../lib/ipc", () => ({
  ipcGetNetworkData: vi.fn(),
}));

import { ipcGetNetworkData } from "../../lib/ipc";

const mockIpcGetNetworkData = vi.mocked(ipcGetNetworkData);

function makeProc(overrides?: Partial<ProcessEntry>): ProcessEntry {
  return {
    pid: 100,
    name: "SafeApp",
    exec_name: "safe-app",
    exe_path: "/usr/bin/safe-app",
    bundle_id: null,
    icon_data_url: null,
    ram_mb: 128,
    cpu_pct: 5,
    disk_read_mb: 0,
    disk_write_mb: 0,
    net_rx_bytes_per_sec: 0,
    net_tx_bytes_per_sec: 0,
    energy_impact_score: 0,
    uptime: "1h",
    group: "Other",
    group_key: "other:safeapp",
    group_identity_type: "normalized_name",
    grouped_name: "SafeApp",
    process_count: 1,
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

describe("security store", () => {
  beforeEach(() => {
    _resetSecurity();
    mockIpcGetNetworkData.mockReset();
    mockIpcGetNetworkData.mockRejectedValue(new Error("backend unavailable"));
  });

  it("starts empty", () => {
    expect(get(securityMap).size).toBe(0);
    expect(get(networkConnections)).toEqual([]);
    expect(get(flaggedPids).size).toBe(0);
    expect(get(totalFindings)).toBe(0);
  });

  describe("refreshSecurityAnalysis", () => {
    it("flags no processes for normal apps", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 1, name: "Chrome" }),
        makeProc({ pid: 2, name: "Finder" }),
      ]);
      expect(get(securityMap).size).toBe(0);
      expect(get(flaggedPids).size).toBe(0);
    });

    it("detects mimikatz as credential-access threat", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 42, name: "mimikatz" }),
      ]);
      const map = get(securityMap);
      expect(map.size).toBe(1);
      const info = map.get(42)!;
      expect(info.threats).toHaveLength(1);
      expect(info.threats[0].indicator).toBe("SuspiciousMemoryRead");
      expect(info.threats[0].mitre_techniques[0].technique_id).toBe("T1003");
      expect(info.threats[0].confidence).toBe(0.9);
    });

    it("detects netcat as suspicious", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 10, name: "nc" }),
      ]);
      expect(get(flaggedPids).has(10)).toBe(true);
      const info = get(securityMap).get(10)!;
      expect(info.threats[0].mitre_techniques[0].technique_id).toBe("T1059.004");
    });

    it("detects rundll32 as defense-evasion", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 20, name: "rundll32" }),
      ]);
      const info = get(securityMap).get(20)!;
      expect(info.threats[0].indicator).toBe("DllInjection");
      expect(info.threats[0].mitre_techniques[0].technique_id).toBe("T1218");
    });

    it("matches known CVEs by process name", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 50, name: "java" }),
      ]);
      const info = get(securityMap).get(50)!;
      expect(info.cves).toHaveLength(1);
      expect(info.cves[0].cve_id).toBe("CVE-2021-44228");
      expect(info.cves[0].severity).toBe("critical");
    });

    it("matches openssl CVE", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 60, name: "openssl" }),
      ]);
      const info = get(securityMap).get(60)!;
      expect(info.cves[0].cve_id).toBe("CVE-2024-5535");
    });

    it("matches httpd CVE", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 70, name: "httpd" }),
      ]);
      const info = get(securityMap).get(70)!;
      expect(info.cves[0].cve_id).toBe("CVE-2024-38476");
    });

    it("matches threats and CVEs using exec_name too", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 88, name: "renamed", exec_name: "powershell" }),
        makeProc({ pid: 89, name: "alias", exec_name: "openssl" }),
      ]);

      expect(get(securityMap).get(88)?.threats[0].indicator).toBe("UnsignedModuleLoad");
      expect(get(securityMap).get(89)?.cves[0].cve_id).toBe("CVE-2024-5535");
    });

    it("totalFindings counts threats + cves", () => {
      refreshSecurityAnalysis([
        makeProc({ pid: 42, name: "mimikatz" }),
        makeProc({ pid: 50, name: "java" }),
      ]);
      expect(get(totalFindings)).toBe(2); // 1 threat + 1 CVE
    });

    it("clears stale entries on re-analysis", () => {
      refreshSecurityAnalysis([makeProc({ pid: 42, name: "mimikatz" })]);
      expect(get(securityMap).size).toBe(1);
      refreshSecurityAnalysis([makeProc({ pid: 1, name: "Chrome" })]);
      expect(get(securityMap).size).toBe(0);
    });
  });

  describe("refreshNetworkConnections", () => {
    it("derives connections from browser tabs (fallback when backend unavailable)", async () => {
      const procs = [
        makeProc({ pid: 1, name: "Chrome", group: "Browser" }),
      ];
      const tabs = [
        { url: "https://example.com/page", browser: "Chrome" },
        { url: "https://github.com/repo", browser: "Chrome" },
      ];
      await refreshNetworkConnections(procs, tabs);
      const conns = get(networkConnections);
      expect(conns).toHaveLength(2);
      expect(conns[0].remote_addr).toBe("example.com");
      expect(conns[0].remote_port).toBe(443);
      expect(conns[0].protocol).toBe("tcp");
      expect(conns[0].direction).toBe("outbound");
      expect(conns[1].remote_addr).toBe("github.com");
    });

    it("handles http URLs with port 80", async () => {
      await refreshNetworkConnections(
        [makeProc({ pid: 1, name: "Firefox", group: "Browser" })],
        [{ url: "http://localhost:3000/dev", browser: "Firefox" }],
      );
      const conns = get(networkConnections);
      expect(conns[0].remote_addr).toBe("localhost");
      expect(conns[0].remote_port).toBe(3000);
    });

    it("skips invalid URLs gracefully", async () => {
      await refreshNetworkConnections(
        [makeProc({ pid: 1, name: "Chrome", group: "Browser" })],
        [{ url: "not-a-url", browser: "Chrome" }],
      );
      expect(get(networkConnections)).toHaveLength(0);
    });

    it("assigns pid 0 when no matching browser process", async () => {
      await refreshNetworkConnections(
        [],
        [{ url: "https://example.com", browser: "Chrome" }],
      );
      expect(get(networkConnections)[0].pid).toBe(0);
    });

    it("stores telemetry status when backend data is unavailable and fallback is used", async () => {
      await refreshNetworkConnections(
        [makeProc({ pid: 1, name: "Chrome", group: "Browser" })],
        [{ url: "https://example.com", browser: "Chrome" }],
      );
      const status = get(networkTelemetryStatus);
      expect(status.usingFallback).toBe(true);
      expect(status.captureBackend).toBe("browser-tabs-fallback");
      expect(status.lastUpdated).not.toBeNull();
    });

    it("uses backend data when available and aggregates duplicate flows", async () => {
      mockIpcGetNetworkData.mockResolvedValue({
        capture_backend: "ebpf",
        dpi_active: true,
        net_rx_bytes_per_sec: 1600,
        net_tx_bytes_per_sec: 1300,
        top_processes: [],
        recent_connections: [
          {
            pid: 1,
            dst_ip: "8.8.8.8",
            dst_port: 443,
            protocol: "Tcp",
            direction: "Outbound",
            bytes: 600,
          },
          {
            pid: 1,
            dst_ip: "8.8.8.8",
            dst_port: 443,
            protocol: "Tcp",
            direction: "Outbound",
            bytes: 400,
          },
          {
            pid: 1,
            dst_ip: "8.8.8.8",
            dst_port: 443,
            protocol: "Tcp",
            direction: "Inbound",
            bytes: 250,
          },
        ],
      } as any);

      await refreshNetworkConnections([makeProc({ pid: 1, name: "Chrome" })], []);

      const conns = get(networkConnections);
      expect(conns).toHaveLength(2);
      expect(conns[0].process_name).toBe("Chrome");
      expect(conns[0].bytes_sent).toBe(1000);
      expect(conns[1].bytes_recv).toBe(250);

      const status = get(networkTelemetryStatus);
      expect(status.captureBackend).toBe("ebpf");
      expect(status.dpiActive).toBe(true);
      expect(status.usingFallback).toBe(false);
    });

    it("no actualiza telemetry cuando el cambio de throughput es menor al umbral", async () => {
      mockIpcGetNetworkData.mockResolvedValue({
        capture_backend: "pcap",
        dpi_active: false,
        net_rx_bytes_per_sec: 1000,
        net_tx_bytes_per_sec: 1000,
        top_processes: [],
        recent_connections: [],
      } as any);

      await refreshNetworkConnections([], []);
      const firstStatus = get(networkTelemetryStatus);

      mockIpcGetNetworkData.mockResolvedValue({
        capture_backend: "pcap",
        dpi_active: false,
        net_rx_bytes_per_sec: 1050,
        net_tx_bytes_per_sec: 1080,
        top_processes: [],
        recent_connections: [],
      } as any);

      await refreshNetworkConnections([], []);
      const secondStatus = get(networkTelemetryStatus);

      expect(secondStatus.lastUpdated).toBe(firstStatus.lastUpdated);
      expect(secondStatus.totalRxBytesPerSec).toBe(firstStatus.totalRxBytesPerSec);
    });

    it("uses pid label when backend process is missing from process list", async () => {
      mockIpcGetNetworkData.mockResolvedValue({
        capture_backend: "pcap",
        dpi_active: false,
        net_rx_bytes_per_sec: 0,
        net_tx_bytes_per_sec: 0,
        top_processes: [],
        recent_connections: [
          {
            pid: 77,
            dst_ip: "9.9.9.9",
            dst_port: 53,
            protocol: "Udp",
            direction: "Inbound",
            bytes: 123,
          },
        ],
      } as any);

      await refreshNetworkConnections([], []);

      expect(get(networkConnections)[0].process_name).toBe("pid:77");
      expect(get(networkConnections)[0].protocol).toBe("udp");
      expect(get(networkConnections)[0].direction).toBe("inbound");
    });
  });

  describe("getSecurityForPid", () => {
    it("returns undefined for safe process", () => {
      refreshSecurityAnalysis([makeProc({ pid: 1, name: "Chrome" })]);
      expect(getSecurityForPid(1)).toBeUndefined();
    });

    it("returns security info for flagged process", () => {
      refreshSecurityAnalysis([makeProc({ pid: 42, name: "mimikatz" })]);
      const info = getSecurityForPid(42);
      expect(info).toBeDefined();
      expect(info!.threats).toHaveLength(1);
    });
  });

  describe("severityRank", () => {
    it("ranks critical highest", () => {
      expect(severityRank("critical")).toBe(4);
      expect(severityRank("high")).toBe(3);
      expect(severityRank("medium")).toBe(2);
      expect(severityRank("low")).toBe(1);
      expect(severityRank(null)).toBe(0);
    });
  });

  describe("severityColor", () => {
    it("returns danger for critical/high", () => {
      expect(severityColor("critical")).toBe("var(--danger)");
      expect(severityColor("high")).toBe("var(--danger)");
    });

    it("returns yellow for medium", () => {
      expect(severityColor("medium")).toBe("var(--yellow)");
    });

    it("returns dim for low/null", () => {
      expect(severityColor("low")).toBe("var(--fg-dim)");
      expect(severityColor(null)).toBe("var(--fg-dim)");
    });
  });
});
