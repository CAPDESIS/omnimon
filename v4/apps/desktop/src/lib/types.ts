export interface ProcessEntry {
  pid: number;
  name: string;
  exec_name: string;
  exe_path: string | null;
  bundle_id: string | null;
  icon_data_url: string | null;
  ram_mb: number;
  cpu_pct: number;
  disk_read_mb: number;
  disk_write_mb: number;
  net_rx_bytes_per_sec: number;
  net_tx_bytes_per_sec: number;
  energy_impact_score: number | null;
  uptime: string;
  group: string;
  group_key: string;
  group_identity_type: string;
  grouped_name: string;
  process_count: number;
  is_system: boolean;
  idle: boolean;
  state: string;
}

export interface SystemStats {
  cpu_usage_pct: number;
  ram_total_gb: number;
  ram_used_pct: number;
  swap_used_mb: number;
  total_processes: number;
  net_rx_bytes_per_sec: number;
  net_tx_bytes_per_sec: number;
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

export interface KillProcessesResult {
  killed: number[];
  failed: Array<[number, string]>;
}

export interface PluginMetric {
  name: string;
  label: string;
  kind: "gauge" | "counter" | string;
  value: number;
  unit: string | null;
  tags: Record<string, string>;
}

export interface PluginDescriptor {
  id: string;
  name: string;
  file_name: string;
  enabled: boolean;
  description: string | null;
  version: string | null;
  status: string;
  last_error: string | null;
  last_run_ms: number | null;
  last_duration_ms: number | null;
  metrics: PluginMetric[];
}

// --- Security & Telemetry Types ---
// Mirror Rust structs from crates/core/src/security.rs and audit.rs
// Ready to consume once IPC commands are exposed.

export type BehaviorIndicator =
  | "DllInjection"
  | "RemoteThreadInjection"
  | "ProcessHollowing"
  | "SuspiciousMemoryRead"
  | "UnsignedModuleLoad"
  | "SuspiciousNetworkConnection";

export interface MitreTechnique {
  technique_id: string;   // e.g. "T1055.001"
  tactic: string;         // e.g. "Defense Evasion / Privilege Escalation"
  name: string;           // Human-readable
}

export interface ProcessThreatLabel {
  pid: number;
  process_name: string;
  indicator: BehaviorIndicator;
  mitre_techniques: MitreTechnique[];
  confidence: number;     // 0.0 - 1.0
  context?: string | null;
}

export interface CveMatch {
  pid: number;
  process_name: string;
  product: string;
  detected_version: string;
  cve_id: string;         // e.g. "CVE-2024-12345"
  severity: string | null; // "low" | "medium" | "high" | "critical"
  summary: string | null;
}

/** Aggregated security status per-process for the UI. */
export interface ProcessSecurityInfo {
  pid: number;
  threats: ProcessThreatLabel[];
  cves: CveMatch[];
}

// --- Network Telemetry Types ---
// Mirror Rust structs from crates/core/src/network.rs and metrics.rs

export interface NetworkConnection {
  pid: number;
  process_name: string;
  remote_addr: string;      // IP or hostname
  remote_port: number;
  protocol: "tcp" | "udp";
  direction: "outbound" | "inbound";
  bytes_sent: number;
  bytes_recv: number;
  state: string;            // ESTABLISHED, LISTEN, etc.
}

// --- Backend Network Telemetry (from Rust watcher) ---

export interface ProcessNetworkThroughput {
  pid: number;
  process_name: string | null;
  rx_bytes_per_sec: number;
  tx_bytes_per_sec: number;
  tcp_packets_per_sec: number;
  udp_packets_per_sec: number;
}

export interface ProcessConnectionEvent {
  pid: number;
  protocol: "Tcp" | "Udp";
  direction: "Inbound" | "Outbound";
  src_ip: string;
  dst_ip: string;
  src_port: number;
  dst_port: number;
  bytes: number;
}

export interface NetworkData {
  top_processes: ProcessNetworkThroughput[];
  recent_connections: ProcessConnectionEvent[];
  net_rx_bytes_per_sec: number;
  net_tx_bytes_per_sec: number;
  capture_backend: string;
  dpi_active: boolean;
}

export interface SuperProcess {
  binary_key: string;
  display_name: string;
  process_count: number;
  pids: number[];
  total_memory_bytes: number;
  total_cpu_usage_percent: number;
  connections: NetworkConnection[];
}

// --- NIST Security Snapshot Types ---
// Mirror Rust struct NistSecuritySnapshot from crates/core/src/audit.rs

export type NistSeverity = "critical" | "high" | "medium" | "low" | "info";

export interface NistFinding {
  id: string;
  category: "threat" | "vulnerability" | "network" | "compliance";
  severity: NistSeverity;
  title: string;
  description: string;
  affected_process: string;
  pid: number;
  mitre_id?: string;
  cve_id?: string;
  recommendation: string;
}

export interface NistSecuritySnapshot {
  timestamp: number;
  hostname: string;
  total_processes: number;
  findings: NistFinding[];
  risk_score: number; // 0-100
  summary: string;
}

// --- Dynamic Alert (from Rust rules engine) ---
// Mirror Rust struct DynamicAlert from crates/core/src/rules_engine.rs

export interface DynamicAlert {
  rule_id: string;
  rule_name: string;
  pid: number;
  process_name: string;
  dst_ip: string;
  dst_port: number;
  country_code: string | null;
  mitre_technique_id: string;
  message: string;
}

export type NetworkAlertSeverity = "info" | "warning" | "critical";
export type NetworkAlertDirection = "upload" | "download" | "both";

export type NetworkAlertCondition =
  | {
      kind: "high_bandwidth";
      threshold_mbps: number;
      direction: NetworkAlertDirection;
      process: string | null;
    }
  | {
      kind: "new_external_connection";
      exclude_known: boolean;
    }
  | {
      kind: "unusual_port";
      suspicious_ports: number[];
    }
  | {
      kind: "process_network_spike";
      process_name: string;
      multiplier: number;
    }
  | {
      kind: "connection_count_exceeded";
      max_connections: number;
      process: string | null;
    }
  | {
      kind: "suspicious_destination";
      patterns: string[];
    };

export interface NetworkAlertRule {
  id: string;
  name: string;
  enabled: boolean;
  condition: NetworkAlertCondition;
  severity: NetworkAlertSeverity;
  cooldown_seconds: number;
  notify_ai: boolean;
}

export interface NetworkAlert {
  id: string;
  rule_id: string;
  rule_name: string;
  severity: NetworkAlertSeverity;
  condition_kind: string;
  message: string;
  triggered_at_unix_ms: number;
  notify_ai: boolean;
  process_name: string | null;
  pid: number | null;
  destination: string | null;
  bandwidth_mbps: number | null;
  connection_count: number | null;
  details: string[];
}

// --- AI Rules Engine Schema v1 ---

export type AiRuleKind = "process_country" | "process_ip" | "process_cidr" | "process_port" | "process_memory";

export interface TemporalCorrelation {
  rule_id: string;
  within_seconds: number;
}

export interface AiRuleV1 {
  id: string;
  name: string;
  enabled: boolean;
  kind: AiRuleKind;
  process_contains: string | null;
  country_code: string | null;
  destination_ip: string | null;
  destination_cidr: string | null;
  destination_port: number | null;
  protocol: "any" | "tcp" | "udp";
  process_memory_mb_gt: number | null;
  mitre_technique_id: string | null;
  temporal_correlation: TemporalCorrelation | null;
}

export interface AiRulesPayload {
  schema_version: 1;
  rules: AiRuleV1[];
}

// --- AI Chat Types ---

export interface ToolResult {
  tool: string;
  success: boolean;
  details: string;
  payload?: Record<string, unknown> | null;
}

export interface ChatResponse {
  reply: string;
  tool_call: ToolResult | null;
}

export interface ProfilePreset {
  id: string;
  label: string;
  idleThreshold: number;
  pollIntervalMs: number;
  automationIntervalSecs: number;
  aiProfile: "general" | "developer" | "gaming" | "battery";
}

export type AiProviderKind = "openrouter" | "openai" | "gemini" | "anthropic" | "ollama";

export interface AiProviderDef {
  id: AiProviderKind;
  label: string;
  models: string[];
}

export const AI_PROVIDERS: AiProviderDef[] = [
  {
    id: "ollama",
    label: "settings.providers.ollama",
    models: ["llama3.2", "llama3.1", "mistral", "gemma2", "qwen2.5"],
  },
  {
    id: "openrouter",
    label: "settings.providers.openrouter",
    models: [
      "meta-llama/llama-3.2-3b-instruct:free",
      "google/gemini-2.0-flash-001",
      "meta-llama/llama-3.1-8b-instruct",
      "anthropic/claude-sonnet-4",
    ],
  },
  {
    id: "openai",
    label: "settings.providers.openai",
    models: ["gpt-4o-mini", "gpt-4o", "gpt-4.1-nano", "gpt-4.1-mini"],
  },
  {
    id: "gemini",
    label: "settings.providers.gemini",
    models: [
      "gemini-2.0-flash",
      "gemini-1.5-flash-8b",
      "gemini-2.5-flash-preview-05-20",
    ],
  },
  {
    id: "anthropic",
    label: "settings.providers.anthropic",
    models: ["claude-sonnet-4-20250514", "claude-haiku-4-5-20251001", "claude-3-5-haiku-20241022"],
  },
];
