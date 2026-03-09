import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/svelte";
import AlertPanel from "../AlertPanel.svelte";
import { writable } from "svelte/store";

type AlertRuleMock = {
  metric: string;
  operator: string;
  threshold: number;
  action: string;
  processName?: string;
};

type FiredAlertMock = {
  id: string;
  timestamp: number;
  processName?: string;
  value: number;
  rule: { metric: string; operator: string; threshold: number };
};

type NetworkAlertMock = {
  id: string;
  rule_id: string;
  rule_name: string;
  severity: "info" | "warning" | "critical";
  condition_kind: string;
  message: string;
  triggered_at_unix_ms: number;
  notify_ai: boolean;
  process_name: string | null;
  pid: number | null;
  destination: string | null;
  bandwidth_mbps: number | null;
  connection_count: number | null;
  details: string[];
};

const { mockAlertRules, mockFiredAlerts, mockNetworkAlerts, mockNetworkAlertFilter, mockClearFired, mockRemoveRule } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockAlertRules: writable<AlertRuleMock[]>([]),
    mockFiredAlerts: writable<FiredAlertMock[]>([]),
    mockNetworkAlerts: writable<NetworkAlertMock[]>([]),
    mockNetworkAlertFilter: writable({ severity: "all", query: "" }),
    mockClearFired: vi.fn(),
    mockRemoveRule: vi.fn(),
  };
});

vi.mock("../../stores/alerts", () => ({
  alertRules: mockAlertRules,
  firedAlerts: mockFiredAlerts,
  networkAlerts: mockNetworkAlerts,
  networkAlertFilter: mockNetworkAlertFilter,
  clearFiredAlerts: mockClearFired,
  clearNetworkAlerts: vi.fn(),
  removeAlertRule: mockRemoveRule,
  investigateNetworkAlert: vi.fn(),
  askAiAboutNetworkAlert: vi.fn(),
  matchesNetworkAlertFilter: vi.fn(() => true),
}));

describe("AlertPanel", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockAlertRules.set([]);
    mockFiredAlerts.set([]);
    mockNetworkAlerts.set([]);
    mockNetworkAlertFilter.set({ severity: "all", query: "" });
    mockClearFired.mockClear();
    mockRemoveRule.mockClear();
  });

  it("renders nothing when no rules and no alerts", () => {
    render(AlertPanel);
    expect(screen.queryByText("Alerts")).not.toBeInTheDocument();
  });

  it("shows trigger button when rules exist", () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    render(AlertPanel);
    expect(screen.getByText("\u2713")).toBeInTheDocument();
  });

  it("shows warning icon and badge when alerts fired", () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    mockFiredAlerts.set([
      {
        id: "a1", timestamp: Date.now(), processName: "Chrome",
        value: 95, rule: { metric: "cpu", operator: ">", threshold: 80 },
      },
    ]);
    render(AlertPanel);
    expect(screen.getByText("\u26A0")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("opens panel on button click", async () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    render(AlertPanel);
    const btn = screen.getByTitle("Alerts (0)");
    await fireEvent.click(btn);
    expect(screen.getByText("Active Rules")).toBeInTheDocument();
  });

  it("displays alert rules in panel", async () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast", processName: "Chrome" },
    ]);
    render(AlertPanel);
    await fireEvent.click(screen.getByTitle("Alerts (0)"));
    expect(screen.getByText(/Chrome.*cpu.*>.*80/)).toBeInTheDocument();
  });

  it("shows 'No alerts fired yet' when panel open but no fired", async () => {
    mockAlertRules.set([
      { metric: "ram", operator: ">", threshold: 1000, action: "toast" },
    ]);
    render(AlertPanel);
    await fireEvent.click(screen.getByTitle("Alerts (0)"));
    expect(screen.getByText("No alerts fired yet.")).toBeInTheDocument();
  });

  it("displays fired alerts in panel", async () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    mockFiredAlerts.set([
      {
        id: "a1", timestamp: new Date(2025, 0, 1, 14, 30, 0).getTime(),
        processName: "Firefox", value: 92.5,
        rule: { metric: "cpu", operator: ">", threshold: 80 },
      },
    ]);
    render(AlertPanel);
    await fireEvent.click(screen.getByTitle("Alerts (1)"));
    expect(screen.getByText(/Firefox.*cpu.*92\.5/)).toBeInTheDocument();
  });

  it("calls clearFiredAlerts when 'Clear All' clicked", async () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    mockFiredAlerts.set([
      {
        id: "a1", timestamp: Date.now(), value: 90,
        rule: { metric: "cpu", operator: ">", threshold: 80 },
      },
    ]);
    render(AlertPanel);
    await fireEvent.click(screen.getByTitle("Alerts (1)"));
    await fireEvent.click(screen.getByText("Clear All"));
    expect(mockClearFired).toHaveBeenCalled();
  });

  it("has a close button in the panel", async () => {
    mockAlertRules.set([
      { metric: "cpu", operator: ">", threshold: 80, action: "toast" },
    ]);
    render(AlertPanel);
    await fireEvent.click(screen.getByTitle("Alerts (0)"));
    expect(screen.getByText("Active Rules")).toBeInTheDocument();
    const closeBtn = document.querySelector(".close-btn");
    expect(closeBtn).toBeInTheDocument();
  });

  it("shows badge count even without rules if alerts fired", () => {
    mockFiredAlerts.set([
      {
        id: "a1", timestamp: Date.now(), value: 90,
        rule: { metric: "cpu", operator: ">", threshold: 80 },
      },
    ]);
    render(AlertPanel);
    expect(screen.getByText("1")).toBeInTheDocument();
  });
});
