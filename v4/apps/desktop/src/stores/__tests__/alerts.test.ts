import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { get } from "svelte/store";
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

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
  initSecurityAlertListener,
  networkAlerts,
  networkAlertFilter,
  dynamicAlerts,
  clearDynamicAlerts,
  dismissSmartAlert,
  _resetAlerts,
} from "../alerts";
import type { ProcessEntry, SystemStats } from "../../lib/types";
import { _resetToasts, toasts } from "../toasts";
import * as ipcModule from "../../lib/ipc";
import { askAiRequest, focusNetworkRequest } from "../uiActions";
import { listen } from "@tauri-apps/api/event";

const mockListen = vi.mocked(listen);

function makeStats(overrides?: Partial<SystemStats>): SystemStats {
  return {
    cpu_usage_pct: 25,
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
    mockListen.mockReset();
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

  it("ignora metricas de proceso no soportadas", () => {
    addAlertRule({ metric: "net_rx" as never, operator: ">", threshold: 20, processName: "Chrome", action: "toast" });
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

  it("evaluates < operator", () => {
    addAlertRule({ metric: "ram", operator: "<", threshold: 50, action: "toast" });
    evaluateAlerts(makeStats({ ram_used_pct: 45 }), []);
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

  it("evaluates net_tx metric", () => {
    addAlertRule({ metric: "net_tx", operator: ">", threshold: 1000, action: "toast" });
    evaluateAlerts(makeStats({ net_tx_bytes_per_sec: 5000 }), []);
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

  it("filters network alerts by severity and query details", () => {
    const alert = {
      id: "n2",
      rule_id: "r2",
      rule_name: "Suspicious destination",
      severity: "critical" as const,
      condition_kind: "suspicious_destination",
      message: "Unknown host contacted",
      triggered_at_unix_ms: Date.now(),
      notify_ai: false,
      process_name: null,
      pid: null,
      destination: "evil.example:8443",
      bandwidth_mbps: null,
      connection_count: 3,
      details: ["Matched regex", "Outbound"],
    };

    expect(matchesNetworkAlertFilter(alert, { severity: "warning", query: "evil" })).toBe(false);
    expect(matchesNetworkAlertFilter(alert, { severity: "critical", query: "evil.example" })).toBe(true);
    expect(matchesNetworkAlertFilter(alert, { severity: "all", query: "missing" })).toBe(false);
  });

  it("builds AI prompt for alerts with missing optional fields", () => {
    askAiAboutNetworkAlert({
      id: "n3",
      rule_id: "r3",
      rule_name: "Connections",
      severity: "info",
      condition_kind: "connection_count_exceeded",
      message: "Many connections",
      triggered_at_unix_ms: Date.now(),
      notify_ai: false,
      process_name: null,
      pid: null,
      destination: null,
      bandwidth_mbps: null,
      connection_count: null,
      details: [],
    });

    const prompt = get(askAiRequest);
    expect(prompt).toContain("Regla: Connections");
    expect(prompt).not.toContain("Proceso:");
    expect(prompt).not.toContain("Detalles:");
  });

  it("investigateNetworkAlert usa string vacio cuando falta process_name", () => {
    investigateNetworkAlert({
      id: "n4",
      rule_id: "r4",
      rule_name: "Bandwidth",
      severity: "warning",
      condition_kind: "high_bandwidth",
      message: "Burst",
      triggered_at_unix_ms: Date.now(),
      notify_ai: false,
      process_name: null,
      pid: 44,
      destination: null,
      bandwidth_mbps: 10,
      connection_count: null,
      details: [],
    });

    expect(get(focusNetworkRequest)).toBe("");
  });

  it("initializes security and network listeners and trims history", async () => {
    const unlistenSecurity = vi.fn();
    const unlistenNetwork = vi.fn();
    const handlers: Record<string, (event: { payload: any }) => void> = {};

    mockListen.mockImplementation(async (eventName: string, cb: (event: { payload: any }) => void) => {
      handlers[eventName] = cb;
      return eventName === "security-alert" ? unlistenSecurity : unlistenNetwork;
    });

    const unsubscribe = await initSecurityAlertListener();

    for (let index = 0; index < 55; index += 1) {
      handlers["security-alert"]?.({
        payload: {
          pid: index,
          process_name: `proc-${index}`,
          rule_name: `rule-${index}`,
          message: "triggered",
        },
      });
    }

    for (let index = 0; index < 105; index += 1) {
      handlers["network-alert"]?.({
        payload: {
          id: `alert-${index}`,
          rule_id: `rule-${index}`,
          rule_name: "High bandwidth",
          severity: "warning",
          condition_kind: "high_bandwidth",
          message: index % 2 === 0 ? "details present" : "fallback message",
          triggered_at_unix_ms: Date.now(),
          notify_ai: false,
          process_name: index % 2 === 0 ? `proc-${index}` : null,
          pid: index,
          destination: index % 2 === 0 ? `1.1.1.${index % 255}:443` : null,
          bandwidth_mbps: index % 2 === 0 ? 12.345 : null,
          connection_count: index % 3 === 0 ? index : null,
          details: [],
        },
      });
    }

    expect(get(dynamicAlerts)).toHaveLength(50);
    expect(get(dynamicAlerts)[0].rule_name).toBe("rule-5");
    expect(get(networkAlerts)).toHaveLength(100);
    expect(get(networkAlerts)[0].id).toBe("alert-5");

    unsubscribe();
    expect(unlistenSecurity).toHaveBeenCalledOnce();
    expect(unlistenNetwork).toHaveBeenCalledOnce();
  });

  it("returns noop unsubscriber when tauri listener init fails", async () => {
    mockListen.mockRejectedValueOnce(new Error("no tauri"));

    const unsubscribe = await initSecurityAlertListener();
    expect(typeof unsubscribe).toBe("function");
    expect(() => unsubscribe()).not.toThrow();
  });

  it("clearDynamicAlerts vacia solo alertas dinamicas", () => {
    dynamicAlerts.set([
      {
        pid: 7,
        process_name: "proc",
        rule_name: "r1",
        message: "warn",
      },
    ] as any);
    networkAlerts.set([
      {
        id: "n1",
        rule_id: "r1",
        rule_name: "Bandwidth",
        severity: "warning",
        condition_kind: "high_bandwidth",
        message: "msg",
        triggered_at_unix_ms: Date.now(),
        notify_ai: false,
        process_name: "proc",
        pid: 1,
        destination: null,
        bandwidth_mbps: null,
        connection_count: null,
        details: [],
      },
    ]);

    clearDynamicAlerts();
    expect(get(dynamicAlerts)).toEqual([]);
    expect(get(networkAlerts)).toHaveLength(1);
  });

  it("prunea entradas viejas de cooldown cuando excede el limite", () => {
    vi.spyOn(Date, "now")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(31_000)
      .mockReturnValueOnce(62_000);

    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    addAlertRule({ metric: "swap", operator: ">", threshold: 1, action: "toast" });
    addAlertRule({ metric: "net_rx", operator: ">", threshold: 1, action: "toast" });

    evaluateAlerts(makeStats({ ram_used_pct: 50, swap_used_mb: 10, net_rx_bytes_per_sec: 10 }), []);
    evaluateAlerts(makeStats({ ram_used_pct: 50, swap_used_mb: 10, net_rx_bytes_per_sec: 10 }), []);
    evaluateAlerts(makeStats({ ram_used_pct: 50, swap_used_mb: 10, net_rx_bytes_per_sec: 10 }), []);

    expect(get(firedAlerts)).toHaveLength(6);
  });

  it("prunea cooldowns obsoletos cuando quedan menos reglas activas", () => {
    vi.spyOn(Date, "now")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(31_000);

    addAlertRule({ metric: "ram", operator: ">", threshold: 40, action: "toast" });
    addAlertRule({ metric: "swap", operator: ">", threshold: 1, action: "toast" });
    addAlertRule({ metric: "net_rx", operator: ">", threshold: 1, action: "toast" });

    evaluateAlerts(makeStats({ ram_used_pct: 50, swap_used_mb: 10, net_rx_bytes_per_sec: 10 }), []);
    removeAlertRule(2);
    removeAlertRule(1);
    clearFiredAlerts();

    evaluateAlerts(makeStats({ ram_used_pct: 55 }), []);

    expect(get(firedAlerts)).toHaveLength(1);
    expect(get(firedAlerts)[0].rule.metric).toBe("ram");
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
    vi.useRealTimers();
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

  it("dismissSmartAlert elimina solo una alerta especifica", () => {
    smartAlerts.set([
      { id: "a1", problem: "p1", explanation: "e1", timestamp: 1 },
      { id: "a2", problem: "p2", explanation: "e2", timestamp: 2 },
    ]);

    dismissSmartAlert("a1");
    expect(get(smartAlerts)).toEqual([{ id: "a2", problem: "p2", explanation: "e2", timestamp: 2 }]);
  });

  it("RAM alta genera smart alert usando el mayor consumidor", async () => {
    const stats = makeStats({ ram_used_pct: 95 });
    const proc = makeProc({ name: "Memory Hog", cpu_pct: 10, ram_mb: 2048 });

    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(1);
    expect(get(smartAlerts)[0].problem).toContain("Memoria RAM casi llena");
    expect(get(smartAlerts)[0].problem).toContain("Memory Hog");
  });

  it("disco alto genera smart alert para topDisk", async () => {
    const stats = makeStats({ ram_used_pct: 40 });
    const proc = makeProc({ name: "Disk Hog", cpu_pct: 10, disk_read_mb: 350, disk_write_mb: 200 });

    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(1);
    expect(get(smartAlerts)[0].problem).toContain("Alta actividad de Disco");
  });

  it("smart alert swallowea fallos del analisis AI", async () => {
    vi.spyOn(ipcModule, "ipcAnalyzeContext").mockRejectedValueOnce(new Error("ai down"));
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const stats = makeStats({ ram_used_pct: 95 });
    const proc = makeProc({ name: "Memory Hog", cpu_pct: 10, ram_mb: 2048 });

    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);
    evaluateAlerts(stats, [proc]); await vi.advanceTimersByTimeAsync(5000);

    expect(get(smartAlerts)).toHaveLength(0);
    expect(errorSpy).toHaveBeenCalledWith("AI Smart Alert failed", expect.any(Error));
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

  it("listener usa mensajes fallback para security y network toasts", async () => {
    const handlers: Record<string, (event: { payload: any }) => void> = {};
    mockListen.mockImplementation(async (eventName: string, cb: (event: { payload: any }) => void) => {
      handlers[eventName] = cb;
      return vi.fn();
    });

    await initSecurityAlertListener();

    handlers["security-alert"]?.({
      payload: {
        pid: 321,
        process_name: "proc-a",
        rule_name: "Rule A",
        message: "",
      },
    });

    handlers["network-alert"]?.({
      payload: {
        id: "nn",
        rule_id: "rr",
        rule_name: "Network B",
        severity: "warning",
        condition_kind: "high_bandwidth",
        message: "fallback network message",
        triggered_at_unix_ms: Date.now(),
        notify_ai: false,
        process_name: null,
        pid: 8,
        destination: null,
        bandwidth_mbps: null,
        connection_count: null,
        details: [],
      },
    });

    const toastMessages = get(toasts).map((entry) => `${entry.title} ${entry.message ?? ""}`);
    expect(toastMessages.some((entry) => entry.includes("proc-a (PID 321) triggered rule \"Rule A\""))).toBe(true);
    expect(toastMessages.some((entry) => entry.includes("Network: Network B"))).toBe(true);
    expect(toastMessages.some((entry) => entry.includes("0.00 Mbps"))).toBe(true);
  });
});
