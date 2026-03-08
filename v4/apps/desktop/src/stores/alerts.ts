import { writable, get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import type { AlertRule } from "../lib/aiConfigBridge";
import type { ProcessEntry, SystemStats, DynamicAlert } from "../lib/types";
import { toast } from "./toasts";
import { ipcAnalyzeContext } from "../lib/ipc";
import { aiProviderConfig } from "./preferences";

export interface FiredAlert {
  id: string;
  rule: AlertRule;
  value: number;
  timestamp: number;
  processName?: string;
}

export interface SmartAlert {
  id: string;
  problem: string;
  explanation: string;
  processPid?: number;
  processName?: string;
  timestamp: number;
}

/** Configured alert rules (user or AI-generated). */
export const alertRules = writable<AlertRule[]>([]);

/** History of alerts that have fired. */
export const firedAlerts = writable<FiredAlert[]>([]);

/** Smart AI Alerts */
export const smartAlerts = writable<SmartAlert[]>([]);

/**
 * Sanitize browser helper process names to generic descriptions
 * so Health Report alerts don't expose brand names to end users.
 * E.g. "Google Chrome Helper (Renderer)" -> "browser helper process"
 */
const BROWSER_HELPER_RE =
  /^(Google Chrome|Chrome|Chromium|Brave( Browser)?|Microsoft Edge|Arc|Opera|Vivaldi|Safari|Firefox)\s*(Helper|Renderer|Web Content|Content Process|Worker)/i;
const APPLE_INTERNAL_RE =
  /^com\.apple\.(WebKit\.|Safari\.|Chrome)/i;

function sanitizeProcessName(name: string): string {
  if (BROWSER_HELPER_RE.test(name)) return "browser helper process";
  if (APPLE_INTERNAL_RE.test(name)) return "web content process";
  return name;
}

let alertId = 0;
const MAX_FIRED = 100;

/** Cooldown: don't re-fire the same rule within this window (ms). */
const COOLDOWN_MS = 30_000;
const lastFired = new Map<string, number>();

const SMART_COOLDOWN = 60_000 * 5; // 5 mins
const smartAnomalyLog = new Map<string, number>();
let lastSmartCheck = 0;

function ruleKey(rule: AlertRule): string {
  return `${rule.metric}:${rule.operator}:${rule.threshold}:${rule.processName ?? "*"}`;
}

function evaluate(op: AlertRule["operator"], value: number, threshold: number): boolean {
  switch (op) {
    case ">": return value > threshold;
    case "<": return value < threshold;
    case ">=": return value >= threshold;
    case "<=": return value <= threshold;
  }
}

/** Check all alert rules against current stats/processes. Call on each poll. */
export function evaluateAlerts(
  stats: SystemStats | null,
  processes: ProcessEntry[],
): void {
  if (!stats) return;
  const rules = get(alertRules);
  
  // Smart Health evaluation
  evaluateSmartHealth(stats, processes);

  if (rules.length === 0) return;

  const now = Date.now();

  for (const rule of rules) {
    const key = ruleKey(rule);

    // Cooldown check
    const last = lastFired.get(key);
    if (last && now - last < COOLDOWN_MS) continue;

    let value: number | null = null;

    if (rule.processName) {
      // Per-process rule
      const proc = processes.find(
        (p) => p.name.toLowerCase() === rule.processName!.toLowerCase(),
      );
      if (!proc) continue;

      switch (rule.metric) {
        case "cpu": value = proc.cpu_pct; break;
        case "ram": value = proc.ram_mb; break;
        default: continue;
      }
    } else {
      // System-wide rule
      switch (rule.metric) {
        case "cpu": {
          const avg = processes.length > 0
            ? processes.reduce((s, p) => s + p.cpu_pct, 0) / processes.length
            : 0;
          value = avg;
          break;
        }
        case "ram": value = stats.ram_used_pct; break;
        case "net_rx": value = stats.net_rx_bytes_per_sec; break;
        case "net_tx": value = stats.net_tx_bytes_per_sec; break;
        case "swap": value = stats.swap_used_mb; break;
      }
    }

    if (value === null) continue;

    if (evaluate(rule.operator, value, rule.threshold)) {
      lastFired.set(key, now);

      // Prune stale cooldown entries to prevent unbounded Map growth
      if (lastFired.size > rules.length * 2) {
        for (const [k, ts] of lastFired) {
          if (now - ts >= COOLDOWN_MS) lastFired.delete(k);
        }
      }

      const fired: FiredAlert = {
        id: `alert-${++alertId}`,
        rule,
        value,
        timestamp: now,
        processName: rule.processName,
      };

      firedAlerts.update((a) => {
        const next = [...a, fired];
        return next.length > MAX_FIRED ? next.slice(-MAX_FIRED) : next;
      });

      if (rule.action === "toast") {
        const label = rule.processName
          ? `${rule.processName}: ${rule.metric} ${rule.operator} ${rule.threshold}`
          : `System ${rule.metric} ${rule.operator} ${rule.threshold}`;
        toast.warning("Alert", `${label} (current: ${value.toFixed(1)})`);
      }
    }
  }
}

async function evaluateSmartHealth(stats: SystemStats, processes: ProcessEntry[]) {
  const now = Date.now();
  if (now - lastSmartCheck < 5000) return; // Only check every 5s max
  lastSmartCheck = now;

  let problem = "";
  let targetProc: ProcessEntry | undefined;

  const sortedCpu = [...processes].sort((a,b) => b.cpu_pct - a.cpu_pct);
  const sortedDisk = [...processes].sort((a,b) => (b.disk_read_mb + b.disk_write_mb) - (a.disk_read_mb + a.disk_write_mb));

  const topCpu = sortedCpu[0];
  const topDisk = sortedDisk[0];

  if (topCpu && topCpu.cpu_pct > 80) {
    problem = `Alto uso de CPU (${topCpu.cpu_pct.toFixed(0)}%) por ${sanitizeProcessName(topCpu.name)}`;
    targetProc = topCpu;
  } else if (topDisk && (topDisk.disk_read_mb + topDisk.disk_write_mb) > 500) {
    problem = `Alta actividad de Disco (Lectura/Escritura) por ${sanitizeProcessName(topDisk.name)}`;
    targetProc = topDisk;
  } else if (stats.ram_used_pct > 90) {
    const topRam = [...processes].sort((a,b) => b.ram_mb - a.ram_mb)[0];
    if (topRam) {
       problem = `Memoria RAM casi llena (90%+). ${sanitizeProcessName(topRam.name)} es el mayor consumidor`;
       targetProc = topRam;
    }
  }

  if (!problem || !targetProc) return;

  const anomalyKey = targetProc.name;
  const lastTime = smartAnomalyLog.get(anomalyKey);
  if (lastTime && (now - lastTime) < SMART_COOLDOWN) return;
  
  smartAnomalyLog.set(anomalyKey, now);

  // OPTIMIZACIÓN DE CONTEXTO: Filtrar procesos inactivos (0% CPU y muy poca RAM/Disco)
  const activeProcesses = processes
    .filter(p => p.cpu_pct > 0.5 || p.ram_mb > 100 || p.disk_read_mb > 1 || p.disk_write_mb > 1)
    .slice(0, 15);

  const prompt = `Actúas como un 'Health Report' traductor de telemetría para usuarios no técnicos.
Se detectó una anomalía de hardware: ${problem}. 
Genera una explicación muy breve (1-2 oraciones) usando términos coloquiales (ej. peras y manzanas) de por qué esto podría estar pasando y qué significa para la computadora. Responde en español y no uses lenguaje técnico complejo.`;
  
  const ctxStr = JSON.stringify({
    stats: {
       cpu_user: processes.length > 0
         ? Number((processes.reduce((sum, proc) => sum + proc.cpu_pct, 0) / processes.length).toFixed(2))
         : 0,
       ram_used: stats.ram_used_pct,
    },
    target_process: {
       name: targetProc.name,
       cpu: targetProc.cpu_pct,
       ram: targetProc.ram_mb,
       disk_r: targetProc.disk_read_mb,
       disk_w: targetProc.disk_write_mb
    },
    active_context_processes: activeProcesses.map(p => ({
       name: p.name, 
       cpu: p.cpu_pct, 
       ram: p.ram_mb
    }))
  });

  const cfg = get(aiProviderConfig);
  try {
     const reqPayload = `INSTRUCCIÓN DEL SISTEMA:\n${prompt}\n\nDATOS DE TELEMETRÍA:\n${ctxStr}`;
      const explanation = await ipcAnalyzeContext(reqPayload, cfg.provider, cfg.model);
     
     smartAlerts.update(s => [...s, {
        id: `smart-${Date.now()}`,
        problem,
        explanation,
        processPid: targetProc?.pid,
        processName: targetProc?.name,
        timestamp: now
     }]);
  } catch (e) {
     console.error("AI Smart Alert failed", e);
  }
}

export function dismissSmartAlert(id: string) {
  smartAlerts.update(list => list.filter(a => a.id !== id));
}

export function addAlertRule(rule: AlertRule): void {
  alertRules.update((r) => [...r, rule]);
}

export function removeAlertRule(index: number): void {
  alertRules.update((r) => r.filter((_, i) => i !== index));
}

export function clearFiredAlerts(): void {
  firedAlerts.set([]);
}

// --- Dynamic Alerts from Rust Rules Engine ---

/** Real-time alerts from the Rust rules engine, received via Tauri event. */
export const dynamicAlerts = writable<DynamicAlert[]>([]);

const MAX_DYNAMIC = 50;

/**
 * Subscribe to 'security-alert' Tauri events.
 * Returns an unsubscribe function. Call during app mount.
 */
export async function initSecurityAlertListener(): Promise<() => void> {
  try {
    const unlisten = await listen<DynamicAlert>("security-alert", (event: { payload: DynamicAlert }) => {
      const alert = event.payload;
      dynamicAlerts.update((list) => {
        const next = [...list, alert];
        return next.length > MAX_DYNAMIC ? next.slice(-MAX_DYNAMIC) : next;
      });
      toast.warning(
        `Rule: ${alert.rule_name}`,
        alert.message || `${alert.process_name} (PID ${alert.pid}) triggered rule "${alert.rule_name}"`,
      );
    });
    console.debug("[DynamicAlerts] Initialized Tauri security-alert listener");
    return unlisten;
  } catch (err) {
    console.warn("[DynamicAlerts] Failed to initialize Tauri listener (likely SSR or test env):", err);
    // Not in Tauri context (tests, SSR), return no-op
    return () => {};
  }
}

export function clearDynamicAlerts(): void {
  dynamicAlerts.set([]);
}

export function _resetAlerts(): void {
  alertRules.set([]);
  firedAlerts.set([]);
  dynamicAlerts.set([]);
  smartAlerts.set([]);
  lastFired.clear();
  alertId = 0;
}
