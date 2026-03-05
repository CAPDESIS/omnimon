import { writable, derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { ProcessEntry, SystemStats, Metrics } from "../lib/types";

// --- Core stores ---
export const processes = writable<ProcessEntry[]>([]);
export const stats = writable<SystemStats | null>(null);
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
  $processes.filter(
    (p) => p.group === "Browser" && p.name.includes("Chrome"),
  ),
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
function applyDiff(current: ProcessEntry[], incoming: ProcessEntry[]): ProcessEntry[] {
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
    const data: Metrics = await invoke("get_metrics");
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
    const killed: number[] = await invoke("kill_processes", { pids });
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
    const ok: boolean = await invoke("kill_process", { pid });
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

// --- UI state ---
export const focusedPid = writable<number | null>(null);
export const grouping = writable(false);

// --- Polling lifecycle ---
let intervalId: ReturnType<typeof setInterval> | null = null;

export function startPolling(intervalMs = 2000): void {
  stopPolling();
  fetchMetrics();
  intervalId = setInterval(fetchMetrics, intervalMs);
}

export function stopPolling(): void {
  if (intervalId !== null) {
    clearInterval(intervalId);
    intervalId = null;
  }
}
