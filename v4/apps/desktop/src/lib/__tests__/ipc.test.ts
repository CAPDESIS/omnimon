import { invoke } from "@tauri-apps/api/core";
import {
  ipcGetMetrics,
  ipcKillProcess,
  ipcKillProcesses,
  ipcGetBrowserTabs,
  ipcCloseBrowserTab,
  ipcSaveAiConfig,
  ipcAnalyzeProcesses,
  IPCValidationError,
} from "../ipc";

const mockInvoke = vi.mocked(invoke);

function validProcess(overrides: Record<string, unknown> = {}) {
  return {
    pid: 1,
    name: "test",
    exec_name: "/usr/bin/test",
    ram_mb: 10.5,
    cpu_pct: 1.2,
    uptime: "1h",
    group: "Utilities",
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

function validStats(overrides: Record<string, unknown> = {}) {
  return {
    ram_total_gb: 16,
    ram_used_pct: 55.3,
    swap_used_mb: 128,
    total_processes: 300,
    ...overrides,
  };
}

function validMetrics(overrides: Record<string, unknown> = {}) {
  return {
    processes: [validProcess()],
    stats: validStats(),
    ...overrides,
  };
}

function validTab(overrides: Record<string, unknown> = {}) {
  return {
    id: "tab-1",
    title: "Example",
    url: "https://example.com",
    browser: "Chrome",
    ...overrides,
  };
}

describe("ipcGetMetrics", () => {
  it("returns validated metrics on valid data", async () => {
    mockInvoke.mockResolvedValue(validMetrics());
    const result = await ipcGetMetrics();
    expect(result.processes).toHaveLength(1);
    expect(result.processes[0].pid).toBe(1);
    expect(result.stats.ram_total_gb).toBe(16);
  });

  it("rejects null response", async () => {
    mockInvoke.mockResolvedValue(null);
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects non-array processes", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: "not-array" }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects process with string pid", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: [validProcess({ pid: "abc" })] }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects process entry that is not an object", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: [null] }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects process with NaN ram_mb", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: [validProcess({ ram_mb: NaN })] }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects process with Infinity cpu_pct", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: [validProcess({ cpu_pct: Infinity })] }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects process with non-boolean is_system", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ processes: [validProcess({ is_system: 1 })] }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects stats with missing ram_total_gb", async () => {
    const badStats = validStats();
    delete (badStats as Record<string, unknown>).ram_total_gb;
    mockInvoke.mockResolvedValue(validMetrics({ stats: badStats }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });

  it("rejects stats as null", async () => {
    mockInvoke.mockResolvedValue(validMetrics({ stats: null }));
    await expect(ipcGetMetrics()).rejects.toThrow(IPCValidationError);
  });
});

describe("ipcKillProcess", () => {
  it("returns true on valid boolean response", async () => {
    mockInvoke.mockResolvedValue(true);
    expect(await ipcKillProcess(42)).toBe(true);
  });

  it("rejects non-boolean response", async () => {
    mockInvoke.mockResolvedValue("yes");
    await expect(ipcKillProcess(42)).rejects.toThrow(IPCValidationError);
  });
});

describe("ipcKillProcesses", () => {
  it("returns number array on valid response", async () => {
    mockInvoke.mockResolvedValue([1, 2, 3]);
    expect(await ipcKillProcesses([1, 2, 3])).toEqual([1, 2, 3]);
  });

  it("rejects non-array response", async () => {
    mockInvoke.mockResolvedValue(42);
    await expect(ipcKillProcesses([1])).rejects.toThrow(IPCValidationError);
  });

  it("rejects array containing strings", async () => {
    mockInvoke.mockResolvedValue([1, "two"]);
    await expect(ipcKillProcesses([1, 2])).rejects.toThrow(IPCValidationError);
  });
});

describe("ipcGetBrowserTabs", () => {
  it("returns validated browser tabs on valid data", async () => {
    mockInvoke.mockResolvedValue([validTab(), validTab({ id: "tab-2", browser: "Safari" })]);

    const result = await ipcGetBrowserTabs();

    expect(result).toHaveLength(2);
    expect(result[0].id).toBe("tab-1");
    expect(result[1].browser).toBe("Safari");
    expect(mockInvoke).toHaveBeenCalledWith("get_browser_tabs");
  });

  it("rejects non-array response", async () => {
    mockInvoke.mockResolvedValue({ id: "bad" });
    await expect(ipcGetBrowserTabs()).rejects.toThrow(IPCValidationError);
  });

  it("rejects invalid tab payload", async () => {
    mockInvoke.mockResolvedValue([validTab({ url: 42 })]);
    await expect(ipcGetBrowserTabs()).rejects.toThrow(IPCValidationError);
  });

  it("rejects tab entry that is not an object", async () => {
    mockInvoke.mockResolvedValue([null]);
    await expect(ipcGetBrowserTabs()).rejects.toThrow(IPCValidationError);
  });

  it("supports delayed IPC response (latency)", async () => {
    vi.useFakeTimers();
    mockInvoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => resolve([validTab({ title: "Slow Tab 🐢" })]), 80);
        }),
    );

    const promise = ipcGetBrowserTabs();
    await vi.advanceTimersByTimeAsync(80);

    await expect(promise).resolves.toEqual([
      expect.objectContaining({ title: "Slow Tab 🐢", browser: "Chrome" }),
    ]);
    vi.useRealTimers();
  });

  it("propagates tauri errors like denied AppleScript permissions", async () => {
    mockInvoke.mockRejectedValue(new Error("AppleScript permission denied by user"));
    await expect(ipcGetBrowserTabs()).rejects.toThrow("AppleScript permission denied by user");
  });
});

describe("ipcCloseBrowserTab", () => {
  it("returns true on valid boolean response", async () => {
    mockInvoke.mockResolvedValue(true);
    await expect(ipcCloseBrowserTab("tab-1", "https://example.com", "Chrome")).resolves.toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("close_browser_tab", {
      tabId: "tab-1",
      tabUrl: "https://example.com",
      browser: "Chrome",
    });
  });

  it("rejects non-boolean response", async () => {
    mockInvoke.mockResolvedValue("ok");
    await expect(ipcCloseBrowserTab("tab-1", "https://example.com", "Chrome")).rejects.toThrow(IPCValidationError);
  });

  it("propagates close errors like denied AppleScript permissions", async () => {
    mockInvoke.mockRejectedValue(new Error("User denied Automation permission"));
    await expect(ipcCloseBrowserTab("tab-1", "https://example.com", "Safari")).rejects.toThrow(
      "User denied Automation permission",
    );
  });
});

describe("ipcSaveAiConfig", () => {
  it("calls invoke with correct params", async () => {
    mockInvoke.mockResolvedValue(undefined);
    await ipcSaveAiConfig("openrouter", "gemini-flash", "sk-123");
    expect(mockInvoke).toHaveBeenCalledWith("save_ai_config", {
      provider: "openrouter",
      model: "gemini-flash",
      key: "sk-123",
    });
  });

  it("propagates errors", async () => {
    mockInvoke.mockRejectedValue(new Error("keyring failed"));
    await expect(ipcSaveAiConfig("openrouter", "m", "k")).rejects.toThrow("keyring failed");
  });
});

describe("ipcAnalyzeProcesses", () => {
  function validSuggestion(overrides: Record<string, unknown> = {}) {
    return { pid: 1, name: "Heavy App", reason: "High memory usage", ...overrides };
  }

  it("returns validated suggestions on valid data", async () => {
    mockInvoke.mockResolvedValue([validSuggestion(), validSuggestion({ pid: 2, name: "Other" })]);
    const result = await ipcAnalyzeProcesses("general");
    expect(result).toHaveLength(2);
    expect(result[0].pid).toBe(1);
    expect(result[0].reason).toBe("High memory usage");
    expect(mockInvoke).toHaveBeenCalledWith("analyze_processes", { profile: "general" });
  });

  it("rejects non-array response", async () => {
    mockInvoke.mockResolvedValue({ suggestions: [] });
    await expect(ipcAnalyzeProcesses("general")).rejects.toThrow(IPCValidationError);
  });

  it("rejects suggestion with missing name", async () => {
    mockInvoke.mockResolvedValue([validSuggestion({ name: 42 })]);
    await expect(ipcAnalyzeProcesses("general")).rejects.toThrow(IPCValidationError);
  });

  it("rejects suggestion with non-numeric pid", async () => {
    mockInvoke.mockResolvedValue([validSuggestion({ pid: "abc" })]);
    await expect(ipcAnalyzeProcesses("general")).rejects.toThrow(IPCValidationError);
  });

  it("rejects suggestion that is not an object", async () => {
    mockInvoke.mockResolvedValue([null]);
    await expect(ipcAnalyzeProcesses("general")).rejects.toThrow(IPCValidationError);
  });

  it("propagates API errors", async () => {
    mockInvoke.mockRejectedValue(new Error("No API key configured"));
    await expect(ipcAnalyzeProcesses("developer")).rejects.toThrow("No API key configured");
  });
});

describe("IPCValidationError", () => {
  it("includes field and value", () => {
    const err = new IPCValidationError("test.field", 42);
    expect(err.field).toBe("test.field");
    expect(err.value).toBe(42);
    expect(err.name).toBe("IPCValidationError");
  });
});
