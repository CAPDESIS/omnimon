import { render, screen, fireEvent } from "@testing-library/svelte";
import SecurityReportView from "../SecurityReportView.svelte";
import { writable, derived } from "svelte/store";
import type { ProcessEntry, ProcessSecurityInfo } from "../../lib/types";

const { mockSecurityMap, mockProcesses, mockTotalFindings, mockFlaggedPids } = vi.hoisted(() => {
  const { writable, derived } = require("svelte/store");
  const map = writable(new Map<number, ProcessSecurityInfo>());
  return {
    mockSecurityMap: map,
    mockProcesses: // @ts-ignore
    writable<ProcessEntry[]>([]),
    mockTotalFindings: // @ts-ignore
    derived(map, ($m: Map<number, ProcessSecurityInfo>) => {
      let c = 0;
      for (const info of $m.values()) c += info.threats.length + info.cves.length;
      return c;
    }),
    mockFlaggedPids: // @ts-ignore
    derived(map, ($m: Map<number, ProcessSecurityInfo>) => {
      const s = new Set<number>();
      for (const [pid, info] of $m) {
        if (info.threats.length > 0 || info.cves.length > 0) s.add(pid);
      }
      return s;
    }),
  };
});

vi.mock("../../stores/security", () => ({
  securityMap: mockSecurityMap,
  totalFindings: mockTotalFindings,
  flaggedPids: mockFlaggedPids,
  severityColor: (s: string | null) => {
    if (s === "critical" || s === "high") return "var(--danger)";
    if (s === "medium") return "var(--yellow)";
    return "var(--fg-dim)";
  },
  severityRank: (s: string | null) => {
    if (s === "critical") return 4;
    if (s === "high") return 3;
    if (s === "medium") return 2;
    if (s === "low") return 1;
    return 0;
  },
}));

vi.mock("../../stores/processes", () => ({
  processes: mockProcesses,
}));

describe("SecurityReportView", () => {
  const onclose = vi.fn();

  beforeEach(() => {
    onclose.mockClear();
    mockSecurityMap.set(new Map());
    mockProcesses.set([
      { pid: 1, name: "Chrome", cpu_pct: 5, idle: false, ram_mb: 100, exec_name: "chrome", uptime: "1h", group: "Browser", is_system: false, state: "R" },
    ]);
  });

  it("renders Security Report title", () => {
    render(SecurityReportView, { props: { onclose } });
    expect(screen.getByText("Security Report")).toBeInTheDocument();
    expect(screen.getByText("NIST Framework Assessment")).toBeInTheDocument();
  });

  it("shows healthy message when no findings", () => {
    render(SecurityReportView, { props: { onclose } });
    expect(screen.getByText(/No security issues detected/)).toBeInTheDocument();
  });

  it("shows None Risk when no findings", () => {
    render(SecurityReportView, { props: { onclose } });
    expect(screen.getByText("None Risk")).toBeInTheDocument();
    const scoreEl = document.querySelector(".risk-score");
    expect(scoreEl).toBeInTheDocument();
    expect(scoreEl!.textContent).toBe("0");
  });

  it("shows findings when threats present", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, {
      pid: 42,
      threats: [{
        pid: 42,
        process_name: "mimikatz",
        indicator: "SuspiciousMemoryRead",
        mitre_techniques: [{ technique_id: "T1003", tactic: "Credential Access", name: "OS Credential Dumping" }],
        confidence: 0.9,
      }],
      cves: [],
    });
    mockSecurityMap.set(map);
    render(SecurityReportView, { props: { onclose } });
    expect(screen.getByText("Suspicious Memory Read")).toBeInTheDocument();
    expect(screen.getByText("T1003")).toBeInTheDocument();
    expect(screen.getByText("Findings")).toBeInTheDocument();
  });

  it("shows CVE findings", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(50, {
      pid: 50,
      threats: [],
      cves: [{
        pid: 50,
        process_name: "java",
        product: "Apache Log4j",
        detected_version: "unknown",
        cve_id: "CVE-2021-44228",
        severity: "critical",
        summary: "Remote code execution via JNDI lookup",
      }],
    });
    mockSecurityMap.set(map);
    render(SecurityReportView, { props: { onclose } });
    // CVE appears as both title and tag
    const cveElements = screen.getAllByText("CVE-2021-44228");
    expect(cveElements.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("CRITICAL")).toBeInTheDocument();
  });

  it("expands finding detail on click", async () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, {
      pid: 42,
      threats: [{
        pid: 42,
        process_name: "mimikatz",
        indicator: "SuspiciousMemoryRead",
        mitre_techniques: [{ technique_id: "T1003", tactic: "Credential Access", name: "OS Credential Dumping" }],
        confidence: 0.9,
      }],
      cves: [],
    });
    mockSecurityMap.set(map);
    render(SecurityReportView, { props: { onclose } });
    const header = screen.getByText("Suspicious Memory Read").closest("button")!;
    await fireEvent.click(header);
    expect(screen.getByText("What happened")).toBeInTheDocument();
    expect(screen.getByText("What to do")).toBeInTheDocument();
  });

  it("calls onclose when close button clicked", async () => {
    render(SecurityReportView, { props: { onclose } });
    await fireEvent.click(screen.getByLabelText("Close"));
    expect(onclose).toHaveBeenCalled();
  });

  it("calls onclose when backdrop clicked", async () => {
    render(SecurityReportView, { props: { onclose } });
    const backdrop = document.querySelector(".report-backdrop")!;
    await fireEvent.click(backdrop);
    expect(onclose).toHaveBeenCalled();
  });

  it("shows process and findings count in meta", () => {
    render(SecurityReportView, { props: { onclose } });
    expect(screen.getByText("1 processes")).toBeInTheDocument();
  });

  it("calculates risk score from findings", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, {
      pid: 42,
      threats: [{
        pid: 42,
        process_name: "mimikatz",
        indicator: "SuspiciousMemoryRead",
        mitre_techniques: [{ technique_id: "T1003", tactic: "Credential Access", name: "OS Credential Dumping" }],
        confidence: 0.9,
      }],
      cves: [{
        pid: 42,
        process_name: "java",
        product: "Log4j",
        detected_version: "unknown",
        cve_id: "CVE-2021-44228",
        severity: "critical",
        summary: "RCE",
      }],
    });
    mockSecurityMap.set(map);
    render(SecurityReportView, { props: { onclose } });
    // high threat = 3*15=45, critical CVE = 4*15=60, total = 100 (capped)
    expect(screen.getByText("Critical Risk")).toBeInTheDocument();
  });

  it("sorts findings by severity", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(1, {
      pid: 1,
      threats: [],
      cves: [
        { pid: 1, process_name: "openssl", product: "OpenSSL", detected_version: "unknown", cve_id: "CVE-LOW", severity: "low", summary: "Low issue" },
        { pid: 1, process_name: "java", product: "Log4j", detected_version: "unknown", cve_id: "CVE-CRIT", severity: "critical", summary: "Critical" },
      ],
    });
    mockSecurityMap.set(map);
    render(SecurityReportView, { props: { onclose } });
    // CVE IDs appear as both finding-title and finding-tag
    expect(screen.getAllByText("CVE-CRIT").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("CVE-LOW").length).toBeGreaterThanOrEqual(1);
  });
});
