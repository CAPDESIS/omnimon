import { invoke } from "@tauri-apps/api/core";
import type { ProcessEntry, SystemStats, Metrics } from "./types";

export class IPCValidationError extends Error {
  constructor(
    public readonly field: string,
    public readonly value: unknown,
    message?: string,
  ) {
    super(message ?? `IPC validation failed for "${field}": ${JSON.stringify(value)}`);
    this.name = "IPCValidationError";
  }
}

function assertFiniteNumber(field: string, value: unknown): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new IPCValidationError(field, value, `Expected finite number for "${field}", got ${typeof value}: ${value}`);
  }
}

function assertString(field: string, value: unknown): asserts value is string {
  if (typeof value !== "string") {
    throw new IPCValidationError(field, value, `Expected string for "${field}", got ${typeof value}`);
  }
}

function assertBoolean(field: string, value: unknown): asserts value is boolean {
  if (typeof value !== "boolean") {
    throw new IPCValidationError(field, value, `Expected boolean for "${field}", got ${typeof value}`);
  }
}

function validateProcessEntry(raw: unknown, index: number): ProcessEntry {
  if (raw == null || typeof raw !== "object") {
    throw new IPCValidationError(`processes[${index}]`, raw, `Expected object at index ${index}`);
  }
  const r = raw as Record<string, unknown>;

  assertFiniteNumber(`processes[${index}].pid`, r.pid);
  assertString(`processes[${index}].name`, r.name);
  assertString(`processes[${index}].exec_name`, r.exec_name);
  assertFiniteNumber(`processes[${index}].ram_mb`, r.ram_mb);
  assertFiniteNumber(`processes[${index}].cpu_pct`, r.cpu_pct);
  assertString(`processes[${index}].uptime`, r.uptime);
  assertString(`processes[${index}].group`, r.group);
  assertBoolean(`processes[${index}].is_system`, r.is_system);
  assertBoolean(`processes[${index}].idle`, r.idle);
  assertString(`processes[${index}].state`, r.state);

  return {
    pid: r.pid as number,
    name: r.name as string,
    exec_name: r.exec_name as string,
    ram_mb: r.ram_mb as number,
    cpu_pct: r.cpu_pct as number,
    uptime: r.uptime as string,
    group: r.group as string,
    is_system: r.is_system as boolean,
    idle: r.idle as boolean,
    state: r.state as string,
  };
}

function validateSystemStats(raw: unknown): SystemStats {
  if (raw == null || typeof raw !== "object") {
    throw new IPCValidationError("stats", raw, "Expected object for stats");
  }
  const r = raw as Record<string, unknown>;

  assertFiniteNumber("stats.ram_total_gb", r.ram_total_gb);
  assertFiniteNumber("stats.ram_used_pct", r.ram_used_pct);
  assertFiniteNumber("stats.swap_used_mb", r.swap_used_mb);
  assertFiniteNumber("stats.total_processes", r.total_processes);

  return {
    ram_total_gb: r.ram_total_gb as number,
    ram_used_pct: r.ram_used_pct as number,
    swap_used_mb: r.swap_used_mb as number,
    total_processes: r.total_processes as number,
  };
}

export async function ipcGetMetrics(): Promise<Metrics> {
  const data: unknown = await invoke("get_metrics");

  if (data == null || typeof data !== "object") {
    throw new IPCValidationError("metrics", data, "Expected object from get_metrics");
  }
  const d = data as Record<string, unknown>;

  if (!Array.isArray(d.processes)) {
    throw new IPCValidationError("metrics.processes", d.processes, "Expected array for processes");
  }

  const processes = d.processes.map((raw, i) => validateProcessEntry(raw, i));
  const stats = validateSystemStats(d.stats);

  return { processes, stats };
}

export async function ipcKillProcess(pid: number): Promise<boolean> {
  const result: unknown = await invoke("kill_process", { pid });
  assertBoolean("kill_process result", result);
  return result;
}

export async function ipcKillProcesses(pids: number[]): Promise<number[]> {
  const result: unknown = await invoke("kill_processes", { pids });

  if (!Array.isArray(result)) {
    throw new IPCValidationError("kill_processes result", result, "Expected array from kill_processes");
  }

  for (let i = 0; i < result.length; i++) {
    assertFiniteNumber(`kill_processes result[${i}]`, result[i]);
  }

  return result as number[];
}
