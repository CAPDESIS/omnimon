import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { ProcessEntry } from "../../lib/types";
import {
  processes,
  stats,
  loading,
  search,
  selectedPids,
  filtered,
  chromeProcesses,
  selectedCount,
  selectedRamMB,
  fetchMetrics,
  killSelected,
  killSingle,
  toggleSelect,
  selectAllVisible,
  selectNone,
  startPolling,
  stopPolling,
  applyDiff,
  _resetForTest,
} from "../processes";

const mockInvoke = vi.mocked(invoke);

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 1,
    name: "TestApp",
    exec_name: "/usr/bin/testapp",
    ram_mb: 50,
    cpu_pct: 5.0,
    uptime: "2m",
    group: "Utilities",
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

beforeEach(() => {
  _resetForTest();
  mockInvoke.mockReset();
});

// --- applyDiff ---
describe("applyDiff", () => {
  it("reuses object reference when metrics unchanged", () => {
    const proc = makeProc({ pid: 1, cpu_pct: 5, ram_mb: 50, state: "R", idle: false });
    const incoming = makeProc({ pid: 1, cpu_pct: 5, ram_mb: 50, state: "R", idle: false });
    const result = applyDiff([proc], [incoming]);
    expect(result[0]).toBe(proc); // same reference
  });

  it("creates new object when cpu_pct changes", () => {
    const proc = makeProc({ pid: 1, cpu_pct: 5 });
    const incoming = makeProc({ pid: 1, cpu_pct: 10 });
    const result = applyDiff([proc], [incoming]);
    expect(result[0]).not.toBe(proc);
    expect(result[0].cpu_pct).toBe(10);
  });

  it("creates new object when ram_mb changes", () => {
    const proc = makeProc({ pid: 1, ram_mb: 50 });
    const incoming = makeProc({ pid: 1, ram_mb: 100 });
    const result = applyDiff([proc], [incoming]);
    expect(result[0].ram_mb).toBe(100);
  });

  it("creates new object when state changes", () => {
    const proc = makeProc({ pid: 1, state: "R" });
    const incoming = makeProc({ pid: 1, state: "S" });
    const result = applyDiff([proc], [incoming]);
    expect(result[0].state).toBe("S");
  });

  it("creates new object when idle changes", () => {
    const proc = makeProc({ pid: 1, idle: false });
    const incoming = makeProc({ pid: 1, idle: true });
    const result = applyDiff([proc], [incoming]);
    expect(result[0].idle).toBe(true);
  });

  it("adds new PIDs", () => {
    const result = applyDiff([], [makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    expect(result).toHaveLength(2);
  });

  it("removes dead PIDs (not in incoming)", () => {
    const result = applyDiff([makeProc({ pid: 1 }), makeProc({ pid: 2 })], [makeProc({ pid: 2 })]);
    expect(result).toHaveLength(1);
    expect(result[0].pid).toBe(2);
  });

  it("handles empty arrays", () => {
    expect(applyDiff([], [])).toEqual([]);
  });
});

// --- fetchMetrics ---
describe("fetchMetrics", () => {
  it("populates processes and stats on success", async () => {
    const proc = makeProc({ pid: 10 });
    const st = { ram_total_gb: 16, ram_used_pct: 50, swap_used_mb: 128, total_processes: 1 };
    mockInvoke.mockResolvedValue({ processes: [proc], stats: st });

    await fetchMetrics();

    expect(get(processes)).toHaveLength(1);
    expect(get(processes)[0].pid).toBe(10);
    expect(get(stats)).toEqual(st);
    expect(get(loading)).toBe(false);
  });

  it("sets loading to false even on error", async () => {
    mockInvoke.mockRejectedValue(new Error("fail"));
    await fetchMetrics();
    expect(get(loading)).toBe(false);
  });

  it("prunes selected PIDs that are no longer alive", async () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    selectedPids.set(new Set([1, 2]));

    mockInvoke.mockResolvedValue({
      processes: [makeProc({ pid: 2 })],
      stats: { ram_total_gb: 16, ram_used_pct: 50, swap_used_mb: 0, total_processes: 1 },
    });

    await fetchMetrics();
    expect(get(selectedPids).has(1)).toBe(false);
    expect(get(selectedPids).has(2)).toBe(true);
  });

  it("applies diff instead of replacing array", async () => {
    const original = makeProc({ pid: 1, cpu_pct: 5, ram_mb: 50, state: "R", idle: false });
    processes.set([original]);

    mockInvoke.mockResolvedValue({
      processes: [makeProc({ pid: 1, cpu_pct: 5, ram_mb: 50, state: "R", idle: false })],
      stats: { ram_total_gb: 16, ram_used_pct: 50, swap_used_mb: 0, total_processes: 1 },
    });

    await fetchMetrics();
    expect(get(processes)[0]).toBe(original);
  });

  it("does not crash on empty IPC response array", async () => {
    mockInvoke.mockResolvedValue({
      processes: [],
      stats: { ram_total_gb: 16, ram_used_pct: 0, swap_used_mb: 0, total_processes: 0 },
    });
    await fetchMetrics();
    expect(get(processes)).toEqual([]);
  });

  it("retains existing processes on IPC error", async () => {
    const proc = makeProc({ pid: 1 });
    processes.set([proc]);
    mockInvoke.mockRejectedValue(new Error("network"));
    await fetchMetrics();
    expect(get(processes)).toHaveLength(1);
  });
});

// --- killSelected ---
describe("killSelected", () => {
  it("returns killed PIDs and removes them from store", async () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    selectedPids.set(new Set([1, 2]));
    mockInvoke.mockResolvedValue([1, 2]);

    const killed = await killSelected();
    expect(killed).toEqual([1, 2]);
    expect(get(processes)).toHaveLength(0);
    expect(get(selectedPids).size).toBe(0);
  });

  it("handles partial kill", async () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    selectedPids.set(new Set([1, 2]));
    mockInvoke.mockResolvedValue([1]);

    const killed = await killSelected();
    expect(killed).toEqual([1]);
    expect(get(processes)).toHaveLength(1);
    expect(get(processes)[0].pid).toBe(2);
  });

  it("returns empty array on error", async () => {
    processes.set([makeProc({ pid: 1 })]);
    selectedPids.set(new Set([1]));
    mockInvoke.mockRejectedValue(new Error("denied"));

    const killed = await killSelected();
    expect(killed).toEqual([]);
  });

  it("returns empty array when nothing selected", async () => {
    const killed = await killSelected();
    expect(killed).toEqual([]);
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

// --- killSingle ---
describe("killSingle", () => {
  it("removes process on success", async () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    selectedPids.set(new Set([1]));
    mockInvoke.mockResolvedValue(true);

    const ok = await killSingle(1);
    expect(ok).toBe(true);
    expect(get(processes)).toHaveLength(1);
    expect(get(selectedPids).has(1)).toBe(false);
  });

  it("returns false on IPC failure", async () => {
    mockInvoke.mockRejectedValue(new Error("fail"));
    expect(await killSingle(99)).toBe(false);
  });

  it("does not remove process when IPC returns false", async () => {
    processes.set([makeProc({ pid: 1 })]);
    mockInvoke.mockResolvedValue(false);

    const ok = await killSingle(1);
    expect(ok).toBe(false);
    expect(get(processes)).toHaveLength(1);
  });
});

// --- toggleSelect ---
describe("toggleSelect", () => {
  it("adds pid when not selected", () => {
    toggleSelect(5);
    expect(get(selectedPids).has(5)).toBe(true);
  });

  it("removes pid when already selected", () => {
    selectedPids.set(new Set([5]));
    toggleSelect(5);
    expect(get(selectedPids).has(5)).toBe(false);
  });
});

// --- selectAllVisible / selectNone ---
describe("selectAllVisible", () => {
  it("selects all non-system visible processes", () => {
    processes.set([
      makeProc({ pid: 1, is_system: false }),
      makeProc({ pid: 2, is_system: true }),
      makeProc({ pid: 3, is_system: false }),
    ]);
    selectAllVisible();
    const pids = get(selectedPids);
    expect(pids.has(1)).toBe(true);
    expect(pids.has(2)).toBe(false);
    expect(pids.has(3)).toBe(true);
  });

  it("respects search filter", () => {
    processes.set([
      makeProc({ pid: 1, name: "Alpha" }),
      makeProc({ pid: 2, name: "Beta" }),
    ]);
    search.set("Alpha");
    selectAllVisible();
    const pids = get(selectedPids);
    expect(pids.has(1)).toBe(true);
    expect(pids.has(2)).toBe(false);
  });
});

describe("selectNone", () => {
  it("clears all selections", () => {
    selectedPids.set(new Set([1, 2, 3]));
    selectNone();
    expect(get(selectedPids).size).toBe(0);
  });
});

// --- Derived stores ---
describe("derived stores", () => {
  it("filtered returns all processes when search is empty", () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    search.set("");
    expect(get(filtered)).toHaveLength(2);
  });

  it("filtered matches by name", () => {
    processes.set([makeProc({ pid: 1, name: "Safari" }), makeProc({ pid: 2, name: "Chrome" })]);
    search.set("chrome");
    expect(get(filtered)).toHaveLength(1);
    expect(get(filtered)[0].name).toBe("Chrome");
  });

  it("chromeProcesses filters by group and name", () => {
    processes.set([
      makeProc({ pid: 1, group: "Browser", name: "Chrome" }),
      makeProc({ pid: 2, group: "Browser", name: "Firefox" }),
      makeProc({ pid: 3, group: "Utilities", name: "Chrome Helper" }),
    ]);
    expect(get(chromeProcesses)).toHaveLength(1);
    expect(get(chromeProcesses)[0].pid).toBe(1);
  });

  it("selectedCount tracks size of selectedPids", () => {
    selectedPids.set(new Set([1, 2]));
    expect(get(selectedCount)).toBe(2);
  });

  it("selectedRamMB sums RAM of selected PIDs", () => {
    processes.set([makeProc({ pid: 1, ram_mb: 100 }), makeProc({ pid: 2, ram_mb: 200 })]);
    selectedPids.set(new Set([1, 2]));
    expect(get(selectedRamMB)).toBe(300);
  });
});

// --- Polling ---
describe("polling", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue({
      processes: [],
      stats: { ram_total_gb: 16, ram_used_pct: 0, swap_used_mb: 0, total_processes: 0 },
    });
  });

  afterEach(() => {
    stopPolling();
    vi.useRealTimers();
  });

  it("startPolling calls fetchMetrics immediately and on interval", async () => {
    startPolling(1000);
    // Immediate call
    expect(mockInvoke).toHaveBeenCalledTimes(1);

    // Advance timer
    await vi.advanceTimersByTimeAsync(1000);
    expect(mockInvoke).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(1000);
    expect(mockInvoke).toHaveBeenCalledTimes(3);
  });

  it("stopPolling stops interval", async () => {
    startPolling(1000);
    expect(mockInvoke).toHaveBeenCalledTimes(1);

    stopPolling();
    await vi.advanceTimersByTimeAsync(3000);
    // No additional calls after stop (just the initial one)
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});
