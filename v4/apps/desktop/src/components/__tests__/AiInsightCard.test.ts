import { render, screen, fireEvent } from "@testing-library/svelte";
import AiInsightCard from "../AiInsightCard.svelte";
import { writable } from "svelte/store";
import type { BehaviorIndicator, ProcessSecurityInfo } from "../../lib/types";

const { mockSecurityMap } = vi.hoisted(() => {
  const { writable } = require("svelte/store");
  return {
    mockSecurityMap: writable(new Map<number, ProcessSecurityInfo>()),
  };
});

vi.mock("../../stores/security", () => ({
  securityMap: mockSecurityMap,
  severityColor: (s: string | null) => {
    if (s === "critical" || s === "high") return "var(--danger)";
    if (s === "medium") return "var(--yellow)";
    return "var(--fg-dim)";
  },
}));

vi.mock("../../stores/processes", () => {
  const { writable } = require("svelte/store");
  return {
    processes: writable([]),
  };
});

const { mockDynamicAlerts } = vi.hoisted(() => {
  const { writable } = require("svelte/store");
  return {
    mockDynamicAlerts: writable([]),
  };
});

vi.mock("../../stores/alerts", () => ({
  dynamicAlerts: mockDynamicAlerts,
}));

function makeThreat(pid: number, indicator: BehaviorIndicator = "SuspiciousMemoryRead") {
  return {
    pid,
    process_name: "mimikatz",
    indicator,
    mitre_techniques: [{ technique_id: "T1003", tactic: "Credential Access", name: "OS Credential Dumping" }],
    confidence: 0.9,
  };
}

function makeCve(pid: number, severity = "critical") {
  return {
    pid,
    process_name: "java",
    product: "Apache Log4j",
    detected_version: "unknown",
    cve_id: "CVE-2021-44228",
    severity,
    summary: "Remote code execution via JNDI lookup",
  };
}

describe("AiInsightCard", () => {
  beforeEach(() => {
    mockSecurityMap.set(new Map());
    mockDynamicAlerts.set([]);
  });

  it("renders nothing when no findings", () => {
    render(AiInsightCard);
    expect(screen.queryByText("AI Security Insights")).not.toBeInTheDocument();
  });

  it("renders insight section when threats present", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText("AI Security Insights")).toBeInTheDocument();
    expect(screen.getByText("1 finding")).toBeInTheDocument();
  });

  it("translates SuspiciousMemoryRead to human language", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42, "SuspiciousMemoryRead")], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText(/trying to read memory from other apps/)).toBeInTheDocument();
  });

  it("translates DllInjection threat", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(10, {
      pid: 10,
      threats: [{
        pid: 10,
        process_name: "rundll32",
        indicator: "DllInjection",
        mitre_techniques: [{ technique_id: "T1218", tactic: "Defense Evasion", name: "System Binary Proxy Execution" }],
        confidence: 0.7,
      }],
      cves: [],
    });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText(/system tool that attackers often abuse/)).toBeInTheDocument();
  });

  it("translates CVE to human language", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(50, { pid: 50, threats: [], cves: [makeCve(50, "critical")] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText(/Critical vulnerability found in Apache Log4j/)).toBeInTheDocument();
  });

  it("shows CVE with high severity message", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(50, { pid: 50, threats: [], cves: [makeCve(50, "high")] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText(/Serious vulnerability found in Apache Log4j/)).toBeInTheDocument();
  });

  it("expands detail on click", async () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    const header = screen.getByText(/trying to read memory/).closest("button")!;
    await fireEvent.click(header);
    expect(screen.getByText("What was detected")).toBeInTheDocument();
    expect(screen.getByText("Recommended action")).toBeInTheDocument();
  });

  it("shows meta chips with technique ID", async () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    const header = screen.getByText(/trying to read memory/).closest("button")!;
    await fireEvent.click(header);
    expect(screen.getByText("MITRE: T1003")).toBeInTheDocument();
    expect(screen.getByText("PID 42")).toBeInTheDocument();
    expect(screen.getByText("Confidence: Very High")).toBeInTheDocument();
  });

  it("shows correct findings count for multiple items", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [makeCve(42)] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByText("2 findings")).toBeInTheDocument();
  });

  it("has severity-colored left border class", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    const card = document.querySelector(".insight-card.severity-high");
    expect(card).toBeInTheDocument();
  });

  it("has accessible region role", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(42, { pid: 42, threats: [makeThreat(42)], cves: [] });
    mockSecurityMap.set(map);
    render(AiInsightCard);
    expect(screen.getByRole("region", { name: "AI Security Insights" })).toBeInTheDocument();
  });

  it("traduce CVE de severidad baja a mensaje generico", () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(50, { pid: 50, threats: [], cves: [makeCve(50, "low")] });
    mockSecurityMap.set(map);

    render(AiInsightCard);

    expect(screen.getByText(/A known vulnerability was found in Apache Log4j/)).toBeInTheDocument();
  });

  it("usa texto y confianza por defecto para amenazas no mapeadas", async () => {
    const map = new Map<number, ProcessSecurityInfo>();
    map.set(77, {
      pid: 77,
      threats: [{
        pid: 77,
        process_name: "unknown-agent",
        indicator: "SuspiciousNetworkConnection",
        mitre_techniques: [],
        confidence: 0.4,
      }],
      cves: [],
    });
    mockSecurityMap.set(map);

    render(AiInsightCard);

    const header = screen.getByText(/shows unusual behavior/).closest("button")!;
    await fireEvent.click(header);

    expect(screen.getByText("Confidence: Low")).toBeInTheDocument();
    expect(screen.getByText(/Anomalous behavior detected by the security engine/)).toBeInTheDocument();
    expect(screen.getByText(/Investigate this process and verify it's operating within expected parameters/)).toBeInTheDocument();
  });

  it("muestra insight de regla con explicacion de descarga y fallback de headline", async () => {
    mockDynamicAlerts.set([
      {
        rule_id: "rule-download",
        rule_name: "Large download detected",
        pid: 90,
        process_name: "Dropbox",
        dst_ip: "8.8.8.8",
        dst_port: 443,
        country_code: null,
        mitre_technique_id: "T1105",
        message: "",
      },
    ]);

    render(AiInsightCard);

    const header = screen.getByText(/triggered rule "Large download detected"/).closest("button")!;
    await fireEvent.click(header);

    expect(screen.getByText(/may be updating or syncing files/)).toBeInTheDocument();
    expect(screen.getByText(/RULE:\s*T1105/)).toBeInTheDocument();
  });

  it("muestra explicacion de memoria para reglas con memory en el nombre", async () => {
    mockDynamicAlerts.set([
      {
        rule_id: "rule-memory",
        rule_name: "Memory pressure",
        pid: 91,
        process_name: "node",
        dst_ip: "10.0.0.2",
        dst_port: 8080,
        country_code: null,
        mitre_technique_id: "T1055",
        message: "Memory rule fired",
      },
    ]);

    render(AiInsightCard);

    const header = screen.getByText(/triggered rule "Memory pressure"/).closest("button")!;
    await fireEvent.click(header);

    expect(screen.getByText(/consuming more memory than usual/)).toBeInTheDocument();
  });

  it("muestra explicacion por pais cuando la regla no coincide con descarga ni memoria", async () => {
    mockDynamicAlerts.set([
      {
        rule_id: "rule-country",
        rule_name: "Unexpected outbound",
        pid: 92,
        process_name: "curl",
        dst_ip: "203.0.113.5",
        dst_port: 22,
        country_code: "DE",
        mitre_technique_id: "T1041",
        message: "Outbound connection detected",
      },
    ]);

    render(AiInsightCard);

    const header = screen.getByText(/triggered rule "Unexpected outbound"/).closest("button")!;
    await fireEvent.click(header);

    expect(screen.getByText(/tried to connect to a server in DE/)).toBeInTheDocument();
  });
});
