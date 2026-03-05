import { invoke } from "@tauri-apps/api/core";
import { ipcGetMetrics, ipcKillProcess, ipcKillProcesses, IPCValidationError } from "../ipc";

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

describe("IPCValidationError", () => {
  it("includes field and value", () => {
    const err = new IPCValidationError("test.field", 42);
    expect(err.field).toBe("test.field");
    expect(err.value).toBe(42);
    expect(err.name).toBe("IPCValidationError");
  });
});
