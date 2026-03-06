import { writable, get } from "svelte/store";
import type { AlertRule } from "../lib/aiConfigBridge";
import type { ProcessEntry, SystemStats, DynamicAlert } from "../lib/types";
import { toast } from "./toasts";

export interface FiredAlert {
  id: string;
  rule: AlertRule;
  value: number;
  timestamp: number;
  processName?: string;
}

/** Configured alert rules (user or AI-generated). */
export const alertRules = writable<AlertRule[]>([]);

/** History of alerts that have fired. */
export const firedAlerts = writable<FiredAlert[]>([]);

let alertId = 0;
const MAX_FIRED = 100;

/** Cooldown: don't re-fire the same rule within this window (ms). */
const COOLDOWN_MS = 30_000;
const lastFired = new Map<string, number>();

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
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<DynamicAlert>("security-alert", (event) => {
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
    return unlisten;
  } catch {
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
  lastFired.clear();
  alertId = 0;
}
