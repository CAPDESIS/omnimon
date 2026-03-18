import { writable, derived, get } from "svelte/store";
import { ipcGetMetrics, ipcKillProcess, ipcKillProcesses, ipcGetBrowserTabs, ipcSaveAiConfig, ipcAnalyzeProcesses } from "../lib/ipc";
import type { ProcessEntry, SystemStats, BrowserTab, ProcessSuggestion, Metrics } from "../lib/types";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { confirmAction } from "../lib/confirm";
import { t } from "../lib/i18n";
import { idleThreshold, refreshInterval, activeProfilePreset, profilePresets } from "./preferences";
import { pushMetrics } from "./metricsHistory";
import { evaluateAlerts } from "./alerts";
import { refreshSecurityAnalysis, refreshNetworkConnections } from "./security";
import { toast } from "./toasts";
import { detectBrowser } from "../lib/browser";
import { askAiRequest } from "./uiActions";

// --- Security analysis throttle ---
const SECURITY_ANALYSIS_INTERVAL_MS = 5000;
let lastSecurityAnalysisTime = 0;

// --- Core stores ---

/** Writable store holding the current list of system processes. */
export const processes = writable<ProcessEntry[]>([]);

/** Writable store holding system-wide statistics (RAM, swap, network), or null before first fetch. */
export const stats = writable<SystemStats | null>(null);

/** Writable store holding the list of open browser tabs across all supported browsers. */
export const browserTabs = writable<BrowserTab[]>([]);

/** Whether the initial metrics fetch is still in progress. */
export const loading = writable(true);

/** Current search/filter query entered by the user in the process table. */
export const search = writable("");

/** Set of PIDs currently selected by the user for batch actions. */
export const selectedPids = writable<Set<number>>(new Set());

// --- Derived stores ---

/** Keywords that match idle/inactive processes in the search filter. */
const IDLE_KEYWORDS = ["inact", "idle", "inactivo", "inactive", "inactivos"];

/** Derived store of processes filtered by the current search query (matches name, PID, group, or idle status). */
let lastFilterQuery = "";
let lastFilterMeta: Array<{ pid: number; name: string; group: string; idle: boolean }> = [];
let lastFilterMatches: number[] = [];
export const filtered = derived([processes, search], ([$processes, $search]) => {
  const q = $search.trim().toLowerCase();
  if (!q) {
    lastFilterQuery = "";
    lastFilterMeta = [];
    lastFilterMatches = [];
    return $processes;
  }

  const sameQuery = q === lastFilterQuery;
  const sameMeta =
    sameQuery &&
    $processes.length === lastFilterMeta.length &&
    $processes.every((proc, index) => {
      const cached = lastFilterMeta[index];
      return cached !== undefined && cached.pid === proc.pid && cached.name === proc.name && cached.group === proc.group && cached.idle === proc.idle;
    });

  if (sameMeta) {
    return lastFilterMatches.map((index) => $processes[index]).filter((proc): proc is ProcessEntry => proc !== undefined);
  }

  // Check if the query is an idle-status filter (e.g. "inact", "idle", "inactivo")
  const isIdleFilter = IDLE_KEYWORDS.some((k) => q === k || q === k + "s");

  const matches: number[] = [];
  const next = $processes.filter((proc, index) => {
    const included = isIdleFilter
      ? proc.idle
      : proc.name.toLowerCase().includes(q) ||
        String(proc.pid).includes(q) ||
        proc.group.toLowerCase().includes(q);
    if (included) matches.push(index);
    return included;
  });

  lastFilterQuery = q;
  lastFilterMeta = $processes.map((proc) => ({ pid: proc.pid, name: proc.name, group: proc.group, idle: proc.idle }));
  lastFilterMatches = matches;
  return next;
});

/** Derived store containing only processes in the "Browser" group. */
export const chromeProcesses = derived(processes, ($processes) =>
  $processes.filter((p) => p.group === "Browser"),
);

/** Derived store with the count of currently selected PIDs. */
export const selectedCount = derived(selectedPids, ($pids) => $pids.size);

/** Derived store with the total RAM (in MB) consumed by all selected processes. */
export const selectedRamMB = derived(
  [processes, selectedPids],
  ([$processes, $pids]) =>
    $processes
      .filter((p) => $pids.has(p.pid))
      .reduce((sum, p) => sum + p.ram_mb, 0),
);

/** Shallow comparison for SystemStats to avoid unnecessary re-renders on poll. */
function shallowEqualStats(a: SystemStats, b: SystemStats): boolean {
  return (
    a.ram_total_gb === b.ram_total_gb &&
    a.ram_used_pct === b.ram_used_pct &&
    a.swap_used_mb === b.swap_used_mb &&
    a.total_processes === b.total_processes &&
    Math.abs(a.net_rx_bytes_per_sec - b.net_rx_bytes_per_sec) < 100 &&
    Math.abs(a.net_tx_bytes_per_sec - b.net_tx_bytes_per_sec) < 100
  );
}

/**
 * Applies a diff-based update to the process list: adds new PIDs, updates changed metrics
 * (reusing object references when unchanged), and removes dead PIDs.
 */
export function applyDiff(current: ProcessEntry[], incoming: ProcessEntry[]): ProcessEntry[] {
  const incomingMap = new Map<number, ProcessEntry>();
  for (const p of incoming) incomingMap.set(p.pid, p);

  const currentMap = new Map<number, ProcessEntry>();
  for (const p of current) currentMap.set(p.pid, p);

  const result: ProcessEntry[] = [];

  // Update existing + add new
  for (const p of incoming) {
    const existing = currentMap.get(p.pid);
    if (existing) {
      // Only create new object if metrics actually changed
      if (
        existing.cpu_pct !== p.cpu_pct ||
        existing.ram_mb !== p.ram_mb ||
        existing.net_rx_bytes_per_sec !== p.net_rx_bytes_per_sec ||
        existing.net_tx_bytes_per_sec !== p.net_tx_bytes_per_sec ||
        existing.energy_impact_score !== p.energy_impact_score ||
        existing.state !== p.state ||
        existing.idle !== p.idle ||
        existing.group !== p.group ||
        existing.group_key !== p.group_key ||
        existing.grouped_name !== p.grouped_name ||
        existing.process_count !== p.process_count ||
        existing.group_identity_type !== p.group_identity_type
      ) {
        result.push(p);
      } else {
        result.push(existing); // Reuse same object reference
      }
    } else {
      result.push(p); // New process
    }
  }

  return result;
}

/** Fetches metrics from the backend, applies diff updates to the process store, and prunes stale selected PIDs. */
export function handleMetricsUpdate(data: Metrics): void {
  try {
    consecutiveErrors = 0; // Reset on success
    const current = get(processes);
    const updated = applyDiff(current, data.processes);
    // Only trigger subscribers if the list actually changed (different length or different entries)
    if (
      updated.length !== current.length ||
      updated.some((p, i) => p !== current[i])
    ) {
      processes.set(updated);
    }
    // Only update stats if values actually changed to avoid unnecessary re-renders
    const prevStats = get(stats);
    if (!prevStats || !shallowEqualStats(prevStats, data.stats)) {
      stats.set(data.stats);
    }

    // Feed time-series history & alert evaluation
    // System CPU from backend (sysinfo global_cpu_info, normalized 0-100%)
    pushMetrics(data.stats, data.stats.cpu_usage_pct ?? 0);
    evaluateAlerts(data.stats, updated);
    // Throttle security analysis to avoid re-scanning every poll cycle
    const now = Date.now();
    if (now - lastSecurityAnalysisTime >= SECURITY_ANALYSIS_INTERVAL_MS) {
      lastSecurityAnalysisTime = now;
      refreshSecurityAnalysis(updated);
    }

    // Prune selected PIDs that no longer exist
    const livePids = new Set(updated.map((p) => p.pid));
    selectedPids.update(($pids) => {
      let changed = false;
      const next = new Set($pids);
      for (const pid of $pids) {
        if (!livePids.has(pid)) {
          next.delete(pid);
          changed = true;
        }
      }
      return changed ? next : $pids;
    });
  } finally {
    loading.set(false);
  }
}

export async function fetchMetrics(): Promise<void> {
  try {
    const data = await ipcGetMetrics(get(idleThreshold));
    handleMetricsUpdate(data);
  } catch (e) {
    consecutiveErrors++;
    console.error("Failed to fetch metrics:", e);
    if (consecutiveErrors === ERROR_TOAST_THRESHOLD) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Metrics fetch failed", `Repeated errors (${consecutiveErrors}×): ${msg}`);
    }
    loading.set(false);
  }
}

/** Kills all currently selected processes after user confirmation. Returns the PIDs that were killed. */
export async function killSelected(): Promise<number[]> {
  const pids = Array.from(get(selectedPids));
  if (pids.length === 0) return [];
  const allProcs = get(processes);
  const allTabs = get(browserTabs);
  const seenBrowsers = new Set<string>();
  const items = pids.map((pid) => {
    const proc = allProcs.find((p) => p.pid === pid);
    let subItems: string[] | undefined;
    if (proc) {
      const browser = detectBrowser(proc);
      if (browser && !seenBrowsers.has(browser)) {
        seenBrowsers.add(browser);
        const tabs = allTabs.filter((t) => t.browser === browser);
        if (tabs.length > 0) {
          subItems = tabs.map((t) => t.title || t.url);
        }
      }
    }
    return {
      label: proc?.name ?? `PID ${pid}`,
      detail: proc ? `PID ${pid} · ${proc.cpu_pct.toFixed(1)}% CPU · ${proc.ram_mb.toFixed(0)} MB` : `PID ${pid}`,
      icon: proc?.icon_data_url ?? null,
      subItems,
    };
  });
  const msg = pids.length === 1
    ? t("processes.confirmKillSelectedSingle")
    : t("processes.confirmKillSelected", { count: pids.length });

  // Build context for AI analysis
  const processNames = items.map((i) => i.label);
  const onAskAi = () => {
    const prompt = t("processes.aiKillQuestion", { processes: processNames.join(", ") })
      || `Is it safe to terminate these processes? ${processNames.join(", ")}`;
    askAiRequest.set(prompt);
  };

  if (!(await confirmAction(msg, items, onAskAi))) return [];
  try {
    const result = await ipcKillProcesses(pids);
    const killed = result.killed;
    // Immediately remove killed processes from UI
    processes.update(($procs) => $procs.filter((p) => !killed.includes(p.pid)));
    selectedPids.set(new Set());
    // Report any failures
    if (result.failed.length > 0) {
      const failMsgs = result.failed.map(([pid, reason]) => `PID ${pid}: ${reason}`).join(", ");
      toast.warning("Some processes could not be killed", failMsgs);
    }
    return killed;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("Rate limited")) {
      toast.warning(t("common.rateLimited") || "Rate limited", msg);
    } else {
      toast.error(t("processes.killErrorTitle") || "Kill failed", msg);
    }
    console.error("Kill failed:", e);
    return [];
  }
}

/** Kills a single process by PID after user confirmation. Returns true if successfully killed. */
export async function killSingle(pid: number, name?: string): Promise<boolean> {
  if (!(await confirmAction(t("processes.confirmKillSingle", { name: name ?? String(pid), pid })))) return false;
  try {
    const ok = await ipcKillProcess(pid);
    if (ok) {
      processes.update(($procs) => $procs.filter((p) => p.pid !== pid));
      selectedPids.update(($pids) => {
        if ($pids.has(pid)) {
          const next = new Set($pids);
          next.delete(pid);
          return next;
        }
        return $pids;
      });
    }
    return ok;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("Rate limited")) {
      toast.warning(t("common.rateLimited") || "Rate limited", msg);
    } else {
      toast.error(t("processes.killErrorTitle") || "Kill failed", msg);
    }
    console.error("Kill single failed:", e);
    return false;
  }
}

/** Toggles the selection state of a process by PID. */
export function toggleSelect(pid: number): void {
  selectedPids.update(($pids) => {
    const next = new Set($pids);
    if (next.has(pid)) next.delete(pid);
    else next.add(pid);
    return next;
  });
}

/** Selects all non-system processes currently visible in the filtered list. */
export function selectAllVisible(): void {
  const visible = get(filtered);
  selectedPids.set(new Set(visible.filter((p) => !p.is_system).map((p) => p.pid)));
}

/** Clears all process selections. */
export function selectNone(): void {
  selectedPids.set(new Set());
}

// --- AI stores ---

/** Writable store holding AI-generated suggestions for processes to kill/optimize. */
export const aiSuggestions = writable<ProcessSuggestion[]>([]);

/** Whether an AI analysis request is currently in flight. */
export const aiLoading = writable(false);

/** Error message from the most recent AI analysis, or null if none. */
export const aiError = writable<string | null>(null);

/** Current AI analysis profile (e.g., "general", "gaming", "development"). */
export const aiProfile = writable("general");

activeProfilePreset.subscribe((presetId) => {
  const preset = get(profilePresets).find((entry) => entry.id === presetId);
  if (preset) {
    aiProfile.set(preset.aiProfile);
  }
});

// --- UI state ---

/** PID of the process row currently focused/highlighted in the table, or null. */
export const focusedPid = writable<number | null>(null);

/** Whether the process table is displayed in grouped-by-category mode. */
export const grouping = writable(true);

/** Triggers AI-powered process analysis using the selected profile and provider. Updates aiSuggestions/aiError stores. */
export async function analyzeWithAi(provider?: string, model?: string): Promise<void> {
  aiLoading.set(true);
  aiError.set(null);
  try {
    const profile = get(aiProfile);
    const p = provider ?? "openrouter";
    const m = model ?? "meta-llama/llama-3.2-3b-instruct:free";
    const suggestions = await ipcAnalyzeProcesses(profile, p, m);
    aiSuggestions.set(suggestions);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg.includes("No matching entry") || msg.includes("not found in secure storage") || msg.includes("keyring")) {
      aiError.set(t("processes.noApiKey"));
    } else {
      aiError.set(msg);
    }
  } finally {
    aiLoading.set(false);
  }
}

/** Persists AI provider configuration (provider, model, API key) to the backend's secure storage. */
export async function saveAiConfigAction(provider: string, model: string, key: string): Promise<void> {
  await ipcSaveAiConfig(provider, model, key);
}

/** Clears AI suggestions and errors from the UI. */
export function dismissAiSuggestions(): void {
  aiSuggestions.set([]);
  aiError.set(null);
}

// --- Browser tabs (separate, slower polling) ---
async function fetchBrowserTabs(): Promise<void> {
  try {
    const tabs = await ipcGetBrowserTabs();
    browserTabs.set(tabs);
  } catch {
    // Best-effort — don't block anything
  }
}

async function fetchNetworkConnections(): Promise<void> {
  try {
    await refreshNetworkConnections(get(processes), get(browserTabs));
  } catch {
    // Best-effort — don't block anything
  }
}

// --- Fetch error tracking ---
let consecutiveErrors = 0;
const ERROR_TOAST_THRESHOLD = 3;

// --- Polling lifecycle ---
let metricsUnlisten: UnlistenFn | null = null;
let isPollingActive = false;
let tabIntervalId: ReturnType<typeof setInterval> | null = null;
let networkIntervalId: ReturnType<typeof setInterval> | null = null;
let pollingIntervalMs = 3000;

const pollingTargets = {
  browserTabs: true,
  network: false,
};

function syncBrowserTabsPolling(): void {
  if (tabIntervalId !== null) {
    clearInterval(tabIntervalId);
    tabIntervalId = null;
  }
  if (!pollingTargets.browserTabs) return;
  fetchBrowserTabs();
  tabIntervalId = setInterval(fetchBrowserTabs, 10000);
}

function syncNetworkPolling(): void {
  if (networkIntervalId !== null) {
    clearInterval(networkIntervalId);
    networkIntervalId = null;
  }
  if (!pollingTargets.network) return;
  fetchNetworkConnections();
  networkIntervalId = setInterval(fetchNetworkConnections, pollingIntervalMs);
}

export function setPollingTarget(target: "browserTabs" | "network", active: boolean): void {
  if (pollingTargets[target] === active) return;
  pollingTargets[target] = active;
  if (!isPollingActive) return;
  if (target === "browserTabs") syncBrowserTabsPolling();
  else syncNetworkPolling();
}

/** Starts periodic polling for metrics (every intervalMs) and browser tabs (every 5s). */
export function startPolling(intervalMs = 3000): void {
  pollingIntervalMs = intervalMs > 0 ? intervalMs : get(refreshInterval);
  stopPolling();
  isPollingActive = true;
  fetchMetrics();
  listen<Metrics>("metrics-update", (event: { payload: Metrics }) => {
    handleMetricsUpdate(event.payload);
  }).then((unlisten: () => void) => {
    if (isPollingActive) {
      metricsUnlisten = unlisten;
    } else {
      unlisten();
    }
  }).catch((e) => {
    console.warn("[processes] Failed to listen for metrics-update:", e);
  });
  syncBrowserTabsPolling();
  syncNetworkPolling();
}

/** Stops all active polling intervals for metrics and browser tabs. */
export function stopPolling(): void {
  isPollingActive = false;
  if (metricsUnlisten !== null) {
    metricsUnlisten();
    metricsUnlisten = null;
  }
  if (tabIntervalId !== null) {
    clearInterval(tabIntervalId);
    tabIntervalId = null;
  }
  if (networkIntervalId !== null) {
    clearInterval(networkIntervalId);
    networkIntervalId = null;
  }
}

/** Reset all stores to initial state — test use only. */
export function _resetForTest(): void {
  stopPolling();
  processes.set([]);
  stats.set(null);
  browserTabs.set([]);
  loading.set(true);
  search.set("");
  selectedPids.set(new Set());
  focusedPid.set(null);
  grouping.set(false);
  aiSuggestions.set([]);
  aiLoading.set(false);
  aiError.set(null);
  aiProfile.set("general");
  consecutiveErrors = 0;
  lastSecurityAnalysisTime = 0;
}
