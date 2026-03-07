import { invoke } from "@tauri-apps/api/core";
import type { ProcessEntry, SystemStats, Metrics, BrowserTab, BrowserName, ProcessSuggestion, KillProcessesResult, ChatResponse } from "./types";

const VALID_BROWSERS = new Set<string>(["Chrome", "Safari", "Brave", "Edge", "Arc", "Firefox"]);

/** Error thrown when a Tauri IPC response fails runtime type validation. */
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
  if (r.exe_path !== null && r.exe_path !== undefined) assertString(`processes[${index}].exe_path`, r.exe_path);
  if (r.bundle_id !== null && r.bundle_id !== undefined) assertString(`processes[${index}].bundle_id`, r.bundle_id);
  if (r.icon_data_url !== null && r.icon_data_url !== undefined) assertString(`processes[${index}].icon_data_url`, r.icon_data_url);
  assertFiniteNumber(`processes[${index}].ram_mb`, r.ram_mb);
  assertFiniteNumber(`processes[${index}].cpu_pct`, r.cpu_pct);
  assertFiniteNumber(`processes[${index}].disk_read_mb`, r.disk_read_mb);
  assertFiniteNumber(`processes[${index}].disk_write_mb`, r.disk_write_mb);
  assertFiniteNumber(`processes[${index}].net_rx_bytes_per_sec`, r.net_rx_bytes_per_sec);
  assertFiniteNumber(`processes[${index}].net_tx_bytes_per_sec`, r.net_tx_bytes_per_sec);
  if (r.energy_impact_score !== null && r.energy_impact_score !== undefined) assertFiniteNumber(`processes[${index}].energy_impact_score`, r.energy_impact_score);
  assertString(`processes[${index}].uptime`, r.uptime);
  assertString(`processes[${index}].group`, r.group);
  assertString(`processes[${index}].group_key`, r.group_key);
  assertString(`processes[${index}].group_identity_type`, r.group_identity_type);
  assertString(`processes[${index}].grouped_name`, r.grouped_name);
  assertFiniteNumber(`processes[${index}].process_count`, r.process_count);
  assertBoolean(`processes[${index}].is_system`, r.is_system);
  assertBoolean(`processes[${index}].idle`, r.idle);
  assertString(`processes[${index}].state`, r.state);

  return {
    pid: r.pid as number,
    name: r.name as string,
    exec_name: r.exec_name as string,
    exe_path: (r.exe_path as string | null | undefined) ?? null,
    bundle_id: (r.bundle_id as string | null | undefined) ?? null,
    icon_data_url: (r.icon_data_url as string | null | undefined) ?? null,
    ram_mb: r.ram_mb as number,
    cpu_pct: r.cpu_pct as number,
    disk_read_mb: r.disk_read_mb as number,
    disk_write_mb: r.disk_write_mb as number,
    net_rx_bytes_per_sec: r.net_rx_bytes_per_sec as number,
    net_tx_bytes_per_sec: r.net_tx_bytes_per_sec as number,
    energy_impact_score: (r.energy_impact_score as number | null | undefined) ?? null,
    uptime: r.uptime as string,
    group: r.group as string,
    group_key: r.group_key as string,
    group_identity_type: r.group_identity_type as string,
    grouped_name: r.grouped_name as string,
    process_count: r.process_count as number,
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
  assertFiniteNumber("stats.net_rx_bytes_per_sec", r.net_rx_bytes_per_sec);
  assertFiniteNumber("stats.net_tx_bytes_per_sec", r.net_tx_bytes_per_sec);

  return {
    ram_total_gb: r.ram_total_gb as number,
    ram_used_pct: r.ram_used_pct as number,
    swap_used_mb: r.swap_used_mb as number,
    total_processes: r.total_processes as number,
    net_rx_bytes_per_sec: r.net_rx_bytes_per_sec as number,
    net_tx_bytes_per_sec: r.net_tx_bytes_per_sec as number,
  };
}

/** Fetches system metrics (process list + stats) from the Rust backend via IPC. */
export async function ipcGetMetrics(idleThreshold?: number): Promise<Metrics> {
  const data: unknown = await invoke("get_metrics", { idleThreshold: idleThreshold ?? 1.0 });

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

/** Sends a kill signal to a single process by PID. Returns true if successful. */
export async function ipcKillProcess(pid: number): Promise<boolean> {
  const result: unknown = await invoke("kill_process", { pid });
  assertBoolean("kill_process result", result);
  return result;
}

/** Kills multiple processes by PID in batch. Returns an object with killed PIDs and failed PIDs with error messages. */
export async function ipcKillProcesses(pids: number[]): Promise<KillProcessesResult> {
  const result: unknown = await invoke("kill_processes", { pids });

  if (result == null || typeof result !== "object" || Array.isArray(result)) {
    throw new IPCValidationError("kill_processes result", result, "Expected object with killed/failed from kill_processes");
  }

  const r = result as Record<string, unknown>;

  if (!Array.isArray(r.killed)) {
    throw new IPCValidationError("kill_processes result.killed", r.killed, "Expected array for killed");
  }

  for (let i = 0; i < r.killed.length; i++) {
    assertFiniteNumber(`kill_processes result.killed[${i}]`, r.killed[i]);
  }

  if (!Array.isArray(r.failed)) {
    throw new IPCValidationError("kill_processes result.failed", r.failed, "Expected array for failed");
  }

  for (let i = 0; i < r.failed.length; i++) {
    const entry = r.failed[i];
    if (!Array.isArray(entry) || entry.length !== 2) {
      throw new IPCValidationError(`kill_processes result.failed[${i}]`, entry, "Expected [number, string] tuple");
    }
    assertFiniteNumber(`kill_processes result.failed[${i}][0]`, entry[0]);
    assertString(`kill_processes result.failed[${i}][1]`, entry[1]);
  }

  return { killed: r.killed as number[], failed: r.failed as Array<[number, string]> };
}

function validateBrowserTab(raw: unknown, index: number): BrowserTab {
  if (raw == null || typeof raw !== "object") {
    throw new IPCValidationError(`tabs[${index}]`, raw, `Expected object at index ${index}`);
  }
  const r = raw as Record<string, unknown>;

  assertString(`tabs[${index}].id`, r.id);
  assertString(`tabs[${index}].title`, r.title);
  assertString(`tabs[${index}].url`, r.url);
  assertString(`tabs[${index}].browser`, r.browser);

  if (!VALID_BROWSERS.has(r.browser as string)) {
    throw new IPCValidationError(`tabs[${index}].browser`, r.browser, `Unknown browser "${r.browser}" at index ${index}`);
  }

  return {
    id: r.id as string,
    title: r.title as string,
    url: r.url as string,
    browser: r.browser as BrowserName,
  };
}

/** Retrieves all open browser tabs across supported browsers via the Rust backend. */
export async function ipcGetBrowserTabs(): Promise<BrowserTab[]> {
  const data: unknown = await invoke("get_browser_tabs");

  if (!Array.isArray(data)) {
    throw new IPCValidationError("get_browser_tabs", data, "Expected array from get_browser_tabs");
  }

  return data.map((raw, i) => validateBrowserTab(raw, i));
}

/** Closes a specific browser tab identified by its tab ID, URL, and browser name. */
export async function ipcCloseBrowserTab(tabId: string, tabUrl: string, browser: string): Promise<boolean> {
  const result: unknown = await invoke("close_browser_tab", { tabId, tabUrl, browser });
  assertBoolean("close_browser_tab result", result);
  return result;
}

/** Brings a specific browser tab to the foreground by its tab ID, URL, and browser name. */
export async function ipcFocusBrowserTab(tabId: string, tabUrl: string, browser: string): Promise<boolean> {
  const result: unknown = await invoke("focus_browser_tab", { tabId, tabUrl, browser });
  assertBoolean("focus_browser_tab result", result);
  return result;
}

/** Sends a free-form context string to the AI backend for analysis. Returns the AI response text. */
export async function ipcAnalyzeContext(context: string, provider: string, model: string): Promise<string> {
  const result: unknown = await invoke("analyze_context", { context, provider, model });
  assertString("analyze_context result", result);
  return result;
}

function validateProcessSuggestion(raw: unknown, index: number): ProcessSuggestion {
  if (raw == null || typeof raw !== "object") {
    throw new IPCValidationError(`suggestions[${index}]`, raw, `Expected object at index ${index}`);
  }
  const r = raw as Record<string, unknown>;

  assertFiniteNumber(`suggestions[${index}].pid`, r.pid);
  assertString(`suggestions[${index}].name`, r.name);
  assertString(`suggestions[${index}].reason`, r.reason);

  return {
    pid: r.pid as number,
    name: r.name as string,
    reason: r.reason as string,
  };
}

/** Persists AI provider configuration (provider, model, API key) to secure storage via the backend. */
export async function ipcSaveAiConfig(provider: string, model: string, key: string): Promise<void> {
  await invoke("save_ai_config", { provider, model, key });
}

/** Checks whether an API key exists in secure storage for the given AI provider. */
export async function ipcCheckApiKey(provider: string): Promise<boolean> {
  const result: unknown = await invoke("check_api_key", { provider });
  assertBoolean("check_api_key result", result);
  return result;
}

/** Validates an API key against the provider's API. Returns true if the key is valid. */
export async function ipcValidateApiKey(provider: string, key: string): Promise<boolean> {
  const result: unknown = await invoke("validate_api_key", { provider, key });
  assertBoolean("validate_api_key result", result);
  return result;
}

/** Sends the current process list to the AI backend for optimization suggestions based on a usage profile. */
export async function ipcAnalyzeProcesses(profile: string, provider: string, model: string): Promise<ProcessSuggestion[]> {
  const data: unknown = await invoke("analyze_processes", { profile, provider, model });

  if (!Array.isArray(data)) {
    throw new IPCValidationError("analyze_processes", data, "Expected array from analyze_processes");
  }

  return data.map((raw, i) => validateProcessSuggestion(raw, i));
}

/** Returns whether the main application window is currently visible. */
export async function ipcGetWindowVisible(): Promise<boolean> {
  const result: unknown = await invoke("get_window_visible");
  assertBoolean("get_window_visible result", result);
  return result;
}

/** Sends AI-generated rules payload to the Rust rules engine. Returns number of rules applied. */
export async function ipcApplyAiRules(payload: string): Promise<number> {
  const result: unknown = await invoke("apply_ai_rules", { payload });
  assertFiniteNumber("apply_ai_rules result", result);
  return result;
}

/** Sends a chat message to the AI backend with tool calling support. */
export async function ipcAiChat(message: string, provider: string, model: string): Promise<ChatResponse> {
  const result: unknown = await invoke("ai_chat", { message, provider, model });

  if (result == null || typeof result !== "object") {
    throw new IPCValidationError("ai_chat result", result, "Expected object from ai_chat");
  }
  const r = result as Record<string, unknown>;
  assertString("ai_chat result.reply", r.reply);

  return {
    reply: r.reply as string,
    tool_call: r.tool_call as ChatResponse["tool_call"],
  };
}

/** Retrieves the JSON schema contract for AI rules from the Rust backend. */
export async function ipcGetAiRulesSchema(): Promise<string> {
  const result: unknown = await invoke("get_ai_rules_schema");
  assertString("get_ai_rules_schema result", result);
  return result;
}
