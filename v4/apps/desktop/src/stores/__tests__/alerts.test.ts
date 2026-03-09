import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import {
  alertRules,
  firedAlerts,
  smartAlerts,
  addAlertRule,
  removeAlertRule,
  evaluateAlerts,
  clearFiredAlerts,
  dismissAllSmartAlerts,
  clearNetworkAlerts,
  askAiAboutNetworkAlert,
  investigateNetworkAlert,
  matchesNetworkAlertFilter,
  networkAlerts,
  networkAlertFilter,
  _resetAlerts,
} from "../alerts";
import type { ProcessEntry, SystemStats } from "../../lib/types";
import { _resetToasts } from "../toasts";
import * as ipcModule from "../../lib/ipc";
import { askAiRequest, focusNetworkRequest } from "../uiActions";

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

  it("filters and routes network alerts", () => {
    networkAlerts.set([
      {
        id: "n1",
        rule_id: "r1",
        rule_name: "Bandwidth",
        severity: "warning",
        condition_kind: "high_bandwidth",
        message: "Chrome supero threshold",
        triggered_at_unix_ms: Date.now(),
        notify_ai: true,
        process_name: "Chrome",
        pid: 10,
        destination: "8.8.8.8:443",
        bandwidth_mbps: 22.4,
        connection_count: null,
        details: ["Threshold: 10 Mbps"],
      },
    ]);
    networkAlertFilter.set({ severity: "warning", query: "chrome" });

    const alert = get(networkAlerts)[0];
    expect(matchesNetworkAlertFilter(alert, get(networkAlertFilter))).toBe(true);

    investigateNetworkAlert(alert);
    askAiAboutNetworkAlert(alert);
    expect(get(focusNetworkRequest)).toBe("Chrome");
    expect(get(askAiRequest)).toContain("Bandwidth");

    clearNetworkAlerts();
    expect(get(networkAlerts)).toHaveLength(0);
  });
});

describe("Smart Health Alerts", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(ipcModule, "ipcAnalyzeContext").mockResolvedValue("Explicación mockeada de IA");
    _resetAlerts();
    _resetToasts();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("pico de 1 lectura NO genera alerta", async () => {
    const stats = makeStats();
    evaluateAlerts(stats, [makeProc({ name: "HighCpuProc", cpu_pct: 90 })]);
    
    // Dejamos que las promesas resuelvan
    await vi.runAllTimersAsync();
    expect(get(smartAlerts)).toHaveLength(0);
  });

  it("3 lecturas consecutivas SÍ genera alerta y actualiza count", async () => {
    const stats = makeStats();
    const proc = makeProc({ name: "HighCpuProc", cpu_pct: 90 });

    // Lectura 1
    evaluateAlerts(stats, [proc]);
    await vi.advanceTimersByTimeAsync(5000);
    
    // Lectura 2
    evaluateAlerts(stats, [proc]);
    await vi.advanceTimersByTimeAsync(5000);

    // Lectura 3
    evaluateAlerts(stats, [proc]);
    await vi.advanceTimersByTimeAsync(5000);

    const alerts = get(smartAlerts);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].problem).toContain("Alto uso de CPU (90%)");
    expect(alerts[0].updateCount).toBe(1);
  });

  it("pico intermitente (alto-bajo-alto) resetea contador", async () => {
    const stats = makeStats();
    
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 90 })]);
    await vi.advanceTimersByTimeAsync(5000);
    
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 10 })]); // Baja
    await vi.advanceTimersByTimeAsync(5000);
    
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 90 })]); // Sube de nuevo
    await vi.advanceTimersByTimeAsync(5000);

    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 90 })]);
    await vi.advanceTimersByTimeAsync(5000);

    // Solo tuvimos 2 consecutivas altas después del bajón, no debe haber alerta
    expect(get(smartAlerts)).toHaveLength(0);
  });

  it("proceso que baja de 80% resetea tracking", async () => {
    const stats = makeStats();
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 90 })]);
    await vi.advanceTimersByTimeAsync(5000);
    
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 70 })]); // Baja de 80%
    await vi.advanceTimersByTimeAsync(5000);
    
    evaluateAlerts(stats, [makeProc({ name: "Proc", cpu_pct: 90 })]);
    await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(0);
  });

  it("segunda alerta del mismo proceso actualiza la existente y updateCount se incrementa correctamente", async () => {
    const stats = makeStats();
    const proc = makeProc({ name: "duetexpertd", cpu_pct: 90 });

    // Generar primera alerta (3 lecturas)
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(1);
    expect(get(smartAlerts)[0].updateCount).toBe(1);

    // Avanzar más allá del cooldown (5 minutos)
    await vi.advanceTimersByTimeAsync(5 * 60 * 1000 + 1000);

    // Generar segunda alerta para el mismo proceso
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    // Debe seguir habiendo solo 1 alerta, pero el count debe subir
    const alerts = get(smartAlerts);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].updateCount).toBe(2);
  });

  it("alertas de procesos diferentes se mantienen separadas", async () => {
    const stats = makeStats();
    
    // Proceso 1
    const p1 = makeProc({ name: "P1", cpu_pct: 90 });
    evaluateAlerts(stats, [p1]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [p1]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [p1]); await vi.advanceTimersByTimeAsync(5000);

    // Proceso 2
    const p2 = makeProc({ name: "P2", cpu_pct: 95 });
    evaluateAlerts(stats, [p2]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [p2]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [p2]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(2);
  });

  it("dismissAllSmartAlerts limpia todas las alertas", async () => {
    const stats = makeStats();
    const proc = makeProc({ name: "P1", cpu_pct: 90 });
    
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(1);
    dismissAllSmartAlerts();
    expect(get(smartAlerts)).toHaveLength(0);
  });

  it("CPU > 100% muestra 'X cores' en el mensaje", async () => {
    const stats = makeStats();
    const proc = makeProc({ name: "HeavyProc", cpu_pct: 207 });

    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    const alerts = get(smartAlerts);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].problem).toContain("207% (2.1 cores)");
  });

  it("CPU < 100% no muestra cores", async () => {
    const stats = makeStats();
    const proc = makeProc({ name: "HeavyProc", cpu_pct: 88 });

    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    const alerts = get(smartAlerts);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].problem).toContain("88%");
    expect(alerts[0].problem).not.toContain("cores");
  });
});
