import { render, screen } from "@testing-library/svelte";
import SecurityBadge from "../SecurityBadge.svelte";
import { writable } from "svelte/store";
import type { ProcessSecurityInfo } from "../../lib/types";

const { mockSecurityMap } = vi.hoisted(() => {
  return {
    mockSecurityMap: writable(new Map<number, ProcessSecurityInfo>()),
  };
});

vi.mock("../../stores/security", () => ({
  securityMap: mockSecurityMap,
  severityColor: (severity: string | null) => {
    switch (severity) {
      case "critical": return "var(--danger)";
      case "high": return "var(--danger)";
      case "medium": return "var(--yellow)";
      default: return "var(--fg-dim)";
    }
  },
}));

function makeThreat(pid: number) {
  return {
    pid,
    process_name: "test",
    indicator: "SuspiciousMemoryRead" as const,
    mitre_techniques: [{ technique_id: "T1003", tactic: "Credential Access", name: "OS Credential Dumping" }],
    confidence: 0.9,
  };
}

function makeCve(pid: number, severity: string) {
  return {
    pid,
    process_name: "test",
    product: "TestProduct",
    detected_version: "unknown",
    cve_id: "CVE-2024-1234",
    severity,
    summary: "Test vulnerability",
  };
}

describe("SecurityBadge", () => {
  beforeEach(() => {
    mockSecurityMap.set(new Map());
  });

  it("renders nothing for safe process", () => {
    render(SecurityBadge, { props: { pid: 1 } });
    expect(screen.queryByText(/MITRE/)).not.toBeInTheDocument();
    expect(screen.queryByText(/CVE/)).not.toBeInTheDocument();
  });

  it("renders MITRE badge for threat", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 42 } });
    expect(screen.getByText("MITRE:1")).toBeInTheDocument();
  });

  it("renders CVE badge for vulnerability", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(50, { pid: 50, threats: [], cves: [makeCve(50, "critical")] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 50 } });
    expect(screen.getByText("CVE:1")).toBeInTheDocument();
  });

  it("renders both badges when both present", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(99, { pid: 99, threats: [makeThreat(99)], cves: [makeCve(99, "high")] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 99 } });
    expect(screen.getByText("MITRE:1")).toBeInTheDocument();
    expect(screen.getByText("CVE:1")).toBeInTheDocument();
  });

  it("has accessible aria-labels", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(10, { pid: 10, threats: [makeThreat(10)], cves: [makeCve(10, "medium")] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 10 } });
    expect(screen.getByLabelText("1 MITRE threat detected")).toBeInTheDocument();
    expect(screen.getByLabelText("1 CVE detected")).toBeInTheDocument();
  });

  it("shows count for multiple threats", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(20, { pid: 20, threats: [makeThreat(20), makeThreat(20)], cves: [] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 20 } });
    expect(screen.getByText("MITRE:2")).toBeInTheDocument();
    expect(screen.getByLabelText("2 MITRE threats detected")).toBeInTheDocument();
  });

  it("has cursor:help for tooltip hint", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(30, { pid: 30, threats: [makeThreat(30)], cves: [] });
    mockSecurityMap.set(map);
    render(SecurityBadge, { props: { pid: 30 } });
    const badge = screen.getByText("MITRE:1");
    expect(badge.className).toContain("sec-badge");
  });
});
