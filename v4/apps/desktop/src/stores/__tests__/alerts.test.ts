import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import {
  alertRules,
  firedAlerts,
  addAlertRule,
  removeAlertRule,
  evaluateAlerts,
  clearFiredAlerts,
  _resetAlerts,
} from "../alerts";
import type { ProcessEntry, SystemStats } from "../../lib/types";
import { _resetToasts } from "../toasts";

function makeStats(overrides?: Partial<SystemStats>): SystemStats {
  return {
    ram_total_gb: 16,
    ram_used_pct: 45,
    swap_used_mb: 128,
    total_processes: 200,
    net_rx_bytes_per_sec: 5000,
    net_tx_bytes_per_sec: 2000,
    ...overrides,
  };
}

function makeProc(overrides?: Partial<ProcessEntry>): ProcessEntry {
  return {
    pid: 1,
    name: "Chrome",
    exec_name: "Google Chrome",
    exe_path: "/Applications/Google Chrome.app",
    bundle_id: null,
    icon_data_url: null,
    ram_mb: 512,
    cpu_pct: 25,
    disk_read_mb: 0,
    disk_write_mb: 0,
    net_rx_bytes_per_sec: 0,
    net_tx_bytes_per_sec: 0,
    energy_impact_score: 0,
    uptime: "1h",
    group: "Browser",
    group_key: "browser:chrome",
    group_identity_type: "browser_family",
    grouped_name: "Chrome",
    process_count: 1,
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

describe("alerts store", () => {
  beforeEach(() => {
    _resetAlerts();
    _resetToasts();
  });

  it("starts with no rules or fired alerts", () => {
    expect(get(alertRules)).toEqual([]);
    expect(get(firedAlerts)).toEqual([]);
  });

  it("addAlertRule adds a rule", () => {
    addAlertRule({ metric: "cpu", operator: ">", threshold: 80, action: "toast" });
    expect(get(alertRules)).toHaveLength(1);
    expect(get(alertRules)[0].threshold).toBe(80);
  });

  it("removeAlertRule removes by index", () => {
    addAlertRule({ metric: "cpu", operator: ">", threshold: 80, action: "toast" });
    addAlertRule({ metric: "ram", operator: ">=", threshold: 90, action: "toast" });
    removeAlertRule(0);
    expect(get(alertRules)).toHaveLength(1);
    expect(get(alertRules)[0].metric).toBe("ram");
  });

  it("evaluateAlerts fires when threshold exceeded", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    evaluateAlerts(makeStats({ ram_used_pct: 50 }), []);
    expect(get(firedAlerts)).toHaveLength(1);
    expect(get(firedAlerts)[0].value).toBe(50);
  });

  it("evaluateAlerts does not fire when below threshold", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 60, action: "toast" });
    evaluateAlerts(makeStats({ ram_used_pct: 50 }), []);
    expect(get(firedAlerts)).toHaveLength(0);
  });

  it("evaluateAlerts works with per-process rules", () => {
    addAlertRule({ metric: "cpu", operator: ">", threshold: 20, processName: "Chrome", action: "toast" });
    evaluateAlerts(makeStats(), [makeProc({ cpu_pct: 30 })]);
    expect(get(firedAlerts)).toHaveLength(1);
    expect(get(firedAlerts)[0].processName).toBe("Chrome");
  });

  it("per-process rule does not fire when process not found", () => {
    addAlertRule({ metric: "cpu", operator: ">", threshold: 20, processName: "Firefox", action: "toast" });
    evaluateAlerts(makeStats(), [makeProc({ name: "Chrome" })]);
    expect(get(firedAlerts)).toHaveLength(0);
  });

  it("respects cooldown (same rule doesn't fire twice within 30s)", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    evaluateAlerts(makeStats({ ram_used_pct: 50 }), []);
    evaluateAlerts(makeStats({ ram_used_pct: 55 }), []);
    // Second call should be suppressed by cooldown
    expect(get(firedAlerts)).toHaveLength(1);
  });

  it("evaluates <= operator", () => {
    addAlertRule({ metric: "cpu", operator: "<=", threshold: 5, action: "toast" });
    const procs = [makeProc({ cpu_pct: 3 })];
    // System-wide CPU avg = 3
    evaluateAlerts(makeStats(), procs);
    expect(get(firedAlerts)).toHaveLength(1);
  });

  it("clearFiredAlerts empties the list", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    evaluateAlerts(makeStats({ ram_used_pct: 50 }), []);
    expect(get(firedAlerts)).toHaveLength(1);
    clearFiredAlerts();
    expect(get(firedAlerts)).toHaveLength(0);
  });

  it("does nothing with null stats", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    evaluateAlerts(null, []);
    expect(get(firedAlerts)).toHaveLength(0);
  });

  it("does nothing with no rules", () => {
    evaluateAlerts(makeStats({ ram_used_pct: 90 }), []);
    expect(get(firedAlerts)).toHaveLength(0);
  });

  it("evaluates net_rx metric", () => {
    addAlertRule({ metric: "net_rx", operator: ">", threshold: 1000, action: "toast" });
    evaluateAlerts(makeStats({ net_rx_bytes_per_sec: 5000 }), []);
    expect(get(firedAlerts)).toHaveLength(1);
  });

  it("evaluates swap metric", () => {
    addAlertRule({ metric: "swap", operator: ">=", threshold: 100, action: "toast" });
    evaluateAlerts(makeStats({ swap_used_mb: 128 }), []);
    expect(get(firedAlerts)).toHaveLength(1);
  });

  it("per-process ram rule", () => {
    addAlertRule({ metric: "ram", operator: ">", threshold: 400, processName: "Chrome", action: "highlight" });
    evaluateAlerts(makeStats(), [makeProc({ ram_mb: 600 })]);
    expect(get(firedAlerts)).toHaveLength(1);
  });
});
