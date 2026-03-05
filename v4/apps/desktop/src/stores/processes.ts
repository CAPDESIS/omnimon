import { writable, derived, get } from "svelte/store";
import { ipcGetMetrics, ipcKillProcess, ipcKillProcesses, ipcGetBrowserTabs, ipcSaveAiConfig, ipcAnalyzeProcesses } from "../lib/ipc";
import type { ProcessEntry, SystemStats, BrowserTab, ProcessSuggestion } from "../lib/types";

// --- Core stores ---
export const processes = writable<ProcessEntry[]>([]);
export const stats = writable<SystemStats | null>(null);
export const browserTabs = writable<BrowserTab[]>([]);
export const loading = writable(true);
export const search = writable("");
export const selectedPids = writable<Set<number>>(new Set());

// --- Derived stores ---
export const filtered = derived([processes, search], ([$processes, $search]) => {
  const q = $search.trim().toLowerCase();
  if (!q) return $processes;
  return $processes.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      String(p.pid).includes(q) ||
      p.group.toLowerCase().includes(q),
  );
});

export const chromeProcesses = derived(processes, ($processes) =>
  $processes.filter((p) => p.group === "Browser"),
);

export const selectedCount = derived(selectedPids, ($pids) => $pids.size);

export const selectedRamMB = derived(
  [processes, selectedPids],
  ([$processes, $pids]) =>
    $processes
      .filter((p) => $pids.has(p.pid))
      .reduce((sum, p) => sum + p.ram_mb, 0),
);

// --- Diff-based refresh ---
// Compares incoming processes with current state:
// 1. Adds new PIDs
// 2. Updates changed metrics on existing PIDs (no re-create)
// 3. Removes dead PIDs (ghost prevention)
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
        existing.state !== p.state ||
        existing.idle !== p.idle
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

// --- IPC actions ---
export async function fetchMetrics(): Promise<void> {
  try {
    const data = await ipcGetMetrics();
    const current = get(processes);
    const updated = applyDiff(current, data.processes);
    processes.set(updated);
    stats.set(data.stats);

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
  } catch (e) {
    console.error("Failed to fetch metrics:", e);
  } finally {
    loading.set(false);
  }
}

export async function killSelected(): Promise<number[]> {
  const pids = Array.from(get(selectedPids));
  if (pids.length === 0) return [];
  try {
    const killed = await ipcKillProcesses(pids);
    // Immediately remove killed processes from UI
    processes.update(($procs) => $procs.filter((p) => !killed.includes(p.pid)));
    selectedPids.set(new Set());
    return killed;
  } catch (e) {
    console.error("Kill failed:", e);
    return [];
  }
}

export async function killSingle(pid: number): Promise<boolean> {
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
    console.error("Kill single failed:", e);
    return false;
  }
}

export function toggleSelect(pid: number): void {
  selectedPids.update(($pids) => {
    const next = new Set($pids);
    if (next.has(pid)) next.delete(pid);
    else next.add(pid);
    return next;
  });
}

export function selectAllVisible(): void {
  const visible = get(filtered);
  selectedPids.set(new Set(visible.filter((p) => !p.is_system).map((p) => p.pid)));
}

export function selectNone(): void {
  selectedPids.set(new Set());
}

// --- AI stores ---
export const aiSuggestions = writable<ProcessSuggestion[]>([]);
export const aiLoading = writable(false);
export const aiError = writable<string | null>(null);
export const aiProfile = writable("general");

// --- UI state ---
export const focusedPid = writable<number | null>(null);
export const grouping = writable(false);

// --- AI actions ---
export async function analyzeWithAi(): Promise<void> {
  aiLoading.set(true);
  aiError.set(null);
  try {
    const profile = get(aiProfile);
    const suggestions = await ipcAnalyzeProcesses(profile);
    aiSuggestions.set(suggestions);
  } catch (e) {
    aiError.set(e instanceof Error ? e.message : String(e));
  } finally {
    aiLoading.set(false);
  }
}

export async function saveAiConfigAction(provider: string, model: string, key: string): Promise<void> {
  await ipcSaveAiConfig(provider, model, key);
}

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

// --- Polling lifecycle ---
let intervalId: ReturnType<typeof setInterval> | null = null;
let tabIntervalId: ReturnType<typeof setInterval> | null = null;

export function startPolling(intervalMs = 2000): void {
  stopPolling();
  fetchMetrics();
  fetchBrowserTabs();
  intervalId = setInterval(fetchMetrics, intervalMs);
  tabIntervalId = setInterval(fetchBrowserTabs, 5000); // tabs every 5s, not 2s
}

export function stopPolling(): void {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
  if (tabIntervalId !== null) {
    clearInterval(tabIntervalId);
    tabIntervalId = null;
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
}
