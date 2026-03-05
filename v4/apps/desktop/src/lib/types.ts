export interface ProcessEntry {
  pid: number;
  name: string;
  exec_name: string;
  ram_mb: number;
  cpu_pct: number;
  uptime: string;
  group: string;
  is_system: boolean;
  idle: boolean;
  state: string;
}

export interface SystemStats {
  ram_total_gb: number;
  ram_used_pct: number;
  swap_used_mb: number;
  total_processes: number;
}

export interface Metrics {
  processes: ProcessEntry[];
  stats: SystemStats;
}

export type BrowserName = "Chrome" | "Safari" | "Brave" | "Edge" | "Arc" | "Firefox";

export interface BrowserTab {
  id: string;
  title: string;
  url: string;
  browser: BrowserName;
}

export interface ProcessSuggestion {
  pid: number;
  name: string;
  reason: string;
}

export type AiProviderKind = "openrouter" | "openai" | "gemini" | "anthropic";

export interface AiProviderDef {
  id: AiProviderKind;
  label: string;
  models: string[];
}

export const AI_PROVIDERS: AiProviderDef[] = [
  {
    id: "openrouter",
    label: "OpenRouter",
    models: [
      "google/gemini-flash-1.5-8b",
      "google/gemini-2.0-flash-001",
      "meta-llama/llama-3.1-8b-instruct",
      "anthropic/claude-sonnet-4",
    ],
  },
  {
    id: "openai",
    label: "OpenAI",
    models: ["gpt-4o-mini", "gpt-4o", "gpt-4.1-nano", "gpt-4.1-mini"],
  },
  {
    id: "gemini",
    label: "Gemini",
    models: [
      "gemini-2.0-flash",
      "gemini-1.5-flash-8b",
      "gemini-2.5-flash-preview-05-20",
    ],
  },
  {
    id: "anthropic",
    label: "Anthropic",
    models: ["claude-sonnet-4-20250514", "claude-haiku-4-5-20251001", "claude-3-5-haiku-20241022"],
  },
];
