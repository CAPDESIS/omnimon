import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import type { ProcessEntry } from "../../lib/types";
import {
  processes,
  stats,
  browserTabs,
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
  aiSuggestions,
  aiLoading,
  aiError,
  aiProfile,
  analyzeWithAi,
  saveAiConfigAction,
  dismissAiSuggestions,
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

// Mock window.confirm for confirmation dialogs
window.confirm = vi.fn(() => true);

beforeEach(() => {
  _resetForTest();
  mockInvoke.mockReset();
  // Confirm dialogs default to "yes" so existing tests pass
  (window.confirm as ReturnType<typeof vi.fn>).mockReturnValue(true);
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

  it("handles null metrics response via IPC validation failure", async () => {
    mockInvoke.mockResolvedValue(null);
    await fetchMetrics();
    expect(get(loading)).toBe(false);
    expect(get(processes)).toEqual([]);
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

  it("keeps metrics flow healthy when browser tab fetch is denied", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_browser_tabs") {
        return Promise.reject(new Error("AppleScript permission denied"));
      }
      return Promise.resolve({
        processes: [makeProc({ pid: 7 })],
        stats: { ram_total_gb: 16, ram_used_pct: 10, swap_used_mb: 0, total_processes: 1 },
      });
    });

    await fetchMetrics();

    expect(get(processes)).toHaveLength(1);
    expect(get(stats)?.total_processes).toBe(1);
    expect(get(browserTabs)).toEqual([]);
  });

  it("supports delayed IPC metrics response (latency)", async () => {
    vi.useFakeTimers();
    loading.set(true);
    mockInvoke.mockImplementation(
      (cmd: string) =>
        new Promise((resolve) => {
          setTimeout(() => {
            if (cmd === "get_browser_tabs") resolve([]);
            else {
              resolve({
                processes: [makeProc({ pid: 77, name: "Worker 🚀" })],
                stats: { ram_total_gb: 16, ram_used_pct: 30, swap_used_mb: 5, total_processes: 1 },
              });
            }
          }, 120);
        }),
    );

    const pending = fetchMetrics();
    expect(get(loading)).toBe(true);

    await vi.advanceTimersByTimeAsync(120);
    await pending;

    expect(get(loading)).toBe(false);
    expect(get(processes)[0].name).toBe("Worker 🚀");
    vi.useRealTimers();
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

  it("returns empty array when user cancels confirmation", async () => {
    (window.confirm as ReturnType<typeof vi.fn>).mockReturnValue(false);
    processes.set([makeProc({ pid: 1 })]);
    selectedPids.set(new Set([1]));
    mockInvoke.mockResolvedValue([1]);

    const killed = await killSelected();
    expect(killed).toEqual([]);
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(get(processes)).toHaveLength(1);
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

  it("returns false when user cancels confirmation", async () => {
    (window.confirm as ReturnType<typeof vi.fn>).mockReturnValue(false);
    processes.set([makeProc({ pid: 1 })]);
    mockInvoke.mockResolvedValue(true);

    const ok = await killSingle(1, "TestApp");
    expect(ok).toBe(false);
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(get(processes)).toHaveLength(1);
  });

  it("keeps other selected PIDs unchanged when killed PID was not selected", async () => {
    processes.set([makeProc({ pid: 1 }), makeProc({ pid: 2 })]);
    selectedPids.set(new Set([2]));
    mockInvoke.mockResolvedValue(true);

    const ok = await killSingle(1);

    expect(ok).toBe(true);
    expect(get(selectedPids).has(2)).toBe(true);
    expect(get(selectedPids).has(1)).toBe(false);
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

  it("filtered supports special characters and emoji names", () => {
    processes.set([
      makeProc({ pid: 1, name: "My App 🔥" }),
      makeProc({ pid: 2, name: "normal-app" }),
      makeProc({ pid: 3, name: "calc(1)+[]" }),
    ]);
    search.set("🔥");
    expect(get(filtered)).toHaveLength(1);
    expect(get(filtered)[0].pid).toBe(1);
  });

  it("chromeProcesses filters by Browser group", () => {
    processes.set([
      makeProc({ pid: 1, group: "Browser", name: "Chrome" }),
      makeProc({ pid: 2, group: "Browser", name: "Safari Renderer" }),
      makeProc({ pid: 3, group: "Utilities", name: "Chrome Helper" }),
    ]);
    expect(get(chromeProcesses)).toHaveLength(2);
    expect(get(chromeProcesses)[0].pid).toBe(1);
    expect(get(chromeProcesses)[1].pid).toBe(2);
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

// --- AI actions ---
describe("analyzeWithAi", () => {
  it("sets suggestions on success", async () => {
    const suggestions = [{ pid: 1, name: "Heavy", reason: "Using 2GB RAM" }];
    mockInvoke.mockResolvedValue(suggestions);

    await analyzeWithAi("openrouter", "google/gemini-flash-1.5-8b");

    expect(get(aiSuggestions)).toEqual(suggestions);
    expect(get(aiLoading)).toBe(false);
    expect(get(aiError)).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("analyze_processes", { profile: "general", provider: "openrouter", model: "google/gemini-flash-1.5-8b" });
  });

  it("sets error on API failure", async () => {
    mockInvoke.mockRejectedValue(new Error("No API key configured"));

    await analyzeWithAi("openrouter", "google/gemini-flash-1.5-8b");

    expect(get(aiSuggestions)).toEqual([]);
    expect(get(aiLoading)).toBe(false);
    expect(get(aiError)).toBe("No API key configured");
  });

  it("stringifies non-Error failures", async () => {
    mockInvoke.mockRejectedValue("plain-string-error");

    await analyzeWithAi("openai", "gpt-4o");

    expect(get(aiError)).toBe("plain-string-error");
  });

  it("passes current profile and provider/model to IPC", async () => {
    aiProfile.set("developer");
    mockInvoke.mockResolvedValue([]);

    await analyzeWithAi("anthropic", "claude-sonnet-4-20250514");

    expect(mockInvoke).toHaveBeenCalledWith("analyze_processes", { profile: "developer", provider: "anthropic", model: "claude-sonnet-4-20250514" });
  });

  it("uses defaults when no provider/model given", async () => {
    mockInvoke.mockResolvedValue([]);

    await analyzeWithAi();

    expect(mockInvoke).toHaveBeenCalledWith("analyze_processes", { profile: "general", provider: "openrouter", model: "meta-llama/llama-3.2-3b-instruct:free" });
  });

  it("sets loading during execution", async () => {
    let resolvePromise: (v: unknown) => void;
    mockInvoke.mockImplementation(
      () => new Promise((resolve) => { resolvePromise = resolve; }),
    );

    const promise = analyzeWithAi("openrouter", "m");
    expect(get(aiLoading)).toBe(true);

    resolvePromise!([]);
    await promise;
    expect(get(aiLoading)).toBe(false);
  });
});

describe("dismissAiSuggestions", () => {
  it("clears suggestions and error", () => {
    aiSuggestions.set([{ pid: 1, name: "X", reason: "Y" }]);
    aiError.set("some error");

    dismissAiSuggestions();

    expect(get(aiSuggestions)).toEqual([]);
    expect(get(aiError)).toBeNull();
  });
});

describe("saveAiConfigAction", () => {
  it("forwards provider/model/key to IPC", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await saveAiConfigAction("openai", "gpt-5", "secret-key");
    expect(mockInvoke).toHaveBeenCalledWith("save_ai_config", {
      provider: "openai",
      model: "gpt-5",
      key: "secret-key",
    });
  });
});

// --- Polling ---
describe("polling", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // fetchMetrics now calls both get_metrics and get_browser_tabs
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_browser_tabs") return Promise.resolve([]);
      return Promise.resolve({
        processes: [],
        stats: { ram_total_gb: 16, ram_used_pct: 0, swap_used_mb: 0, total_processes: 0 },
      });
    });
  });

  afterEach(() => {
    stopPolling();
    vi.useRealTimers();
  });

  it("startPolling calls fetchMetrics immediately and on interval", async () => {
    startPolling(1000);
    // Immediate call: get_metrics (sync), get_browser_tabs (async after await)
    const metricsCallCount = () =>
      mockInvoke.mock.calls.filter((c) => c[0] === "get_metrics").length;

    expect(metricsCallCount()).toBe(1);

    await vi.advanceTimersByTimeAsync(1000);
    expect(metricsCallCount()).toBe(2);

    await vi.advanceTimersByTimeAsync(1000);
    expect(metricsCallCount()).toBe(3);
  });

  it("stopPolling stops interval", async () => {
    startPolling(1000);
    const metricsCallCount = () =>
      mockInvoke.mock.calls.filter((c) => c[0] === "get_metrics").length;

    expect(metricsCallCount()).toBe(1);

    stopPolling();
    await vi.advanceTimersByTimeAsync(3000);
    // No additional calls after stop (just the initial one)
    expect(metricsCallCount()).toBe(1);
  });
});
