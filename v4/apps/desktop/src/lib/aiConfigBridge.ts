/**
 * AI-Driven Config Bridge
 * Receives natural-language instructions, sends them + current config to the AI,
 * and validates the returned JSON patch before applying it.
 * Also generates AI Rules v1 payloads for the Rust rules engine.
 */

import type { AiProviderConfig } from "../stores/preferences";
import type { AiRuleV1, AiRulesPayload, AiRuleKind } from "./types";

/** Allowed config keys that the AI is permitted to modify. */
const MUTABLE_KEYS = new Set([
  "idleThreshold",
  "aiProfile",
  "fontSize",
  "theme",
  "locale",
  "columns",
  "columnOrder",
]);

/** Config keys that must NEVER be modified via AI (security boundary). */
const IMMUTABLE_KEYS = new Set([
  "apiKey",
  "provider",
  "model",
  "telemetry",
  "crabNebulaKey",
]);

export interface ConfigPatch {
  [key: string]: unknown;
}

export interface AlertRule {
  metric: "cpu" | "ram" | "net_rx" | "net_tx" | "swap";
  operator: ">" | "<" | ">=" | "<=";
  threshold: number;
  processName?: string;
  action: "toast" | "sound" | "highlight";
}

/**
 * Validates that a config patch from the AI doesn't touch immutable keys
 * and all values are of expected types.
 * Returns sanitized patch or throws on violation.
 */
export function validateConfigPatch(patch: unknown): ConfigPatch {
  if (!patch || typeof patch !== "object" || Array.isArray(patch)) {
    throw new Error("AI returned invalid config patch: expected a plain object");
  }

  const raw = patch as Record<string, unknown>;
  const sanitized: ConfigPatch = {};

  for (const [key, value] of Object.entries(raw)) {
    // Block immutable keys
    if (IMMUTABLE_KEYS.has(key)) {
      throw new Error(`Security violation: AI attempted to modify protected key "${key}"`);
    }
    // Only allow known mutable keys
    if (!MUTABLE_KEYS.has(key)) {
      continue; // silently skip unknown keys
    }
    // Type validation per key
    if (!validateValue(key, value)) {
      throw new Error(`Invalid value for "${key}": ${JSON.stringify(value)}`);
    }
    sanitized[key] = value;
  }

  return sanitized;
}

function validateValue(key: string, value: unknown): boolean {
  switch (key) {
    case "idleThreshold":
      return typeof value === "number" && value >= 0.1 && value <= 10.0;
    case "fontSize":
      return typeof value === "number" && value >= 8 && value <= 48 && Number.isInteger(value);
    case "theme":
      return typeof value === "string" && ["auto", "light", "dark", "cyberpunk"].includes(value);
    case "locale":
      return typeof value === "string" && ["auto", "en", "es"].includes(value);
    case "aiProfile":
      return typeof value === "string" && ["general", "developer", "gaming", "battery"].includes(value);
    case "columns":
      return typeof value === "object" && value !== null && !Array.isArray(value);
    case "columnOrder":
      return Array.isArray(value) && value.every((v) => typeof v === "string");
    default:
      return false;
  }
}

/**
 * Validates an alert rule from AI.
 * Returns sanitized rule or throws.
 */
export function validateAlertRule(rule: unknown): AlertRule {
  if (!rule || typeof rule !== "object") {
    throw new Error("Invalid alert rule: expected an object");
  }

  const r = rule as Record<string, unknown>;

  const validMetrics = ["cpu", "ram", "net_rx", "net_tx", "swap"];
  const validOperators = [">", "<", ">=", "<="];
  const validActions = ["toast", "sound", "highlight"];

  if (typeof r.metric !== "string" || !validMetrics.includes(r.metric)) {
    throw new Error(`Invalid alert metric: ${r.metric}`);
  }
  if (typeof r.operator !== "string" || !validOperators.includes(r.operator)) {
    throw new Error(`Invalid alert operator: ${r.operator}`);
  }
  if (typeof r.threshold !== "number" || !isFinite(r.threshold) || r.threshold < 0) {
    throw new Error(`Invalid alert threshold: ${r.threshold}`);
  }
  if (r.processName !== undefined && typeof r.processName !== "string") {
    throw new Error("Invalid processName");
  }
  if (typeof r.action !== "string" || !validActions.includes(r.action)) {
    throw new Error(`Invalid alert action: ${r.action}`);
  }

  return {
    metric: r.metric as AlertRule["metric"],
    operator: r.operator as AlertRule["operator"],
    threshold: r.threshold,
    processName: typeof r.processName === "string" ? r.processName : undefined,
    action: r.action as AlertRule["action"],
  };
}

/**
 * Detects potential prompt injection in user input.
 * Returns true if the input looks suspicious.
 */
export function detectPromptInjection(input: string): boolean {
  const lower = input.toLowerCase();
  const patterns = [
    /ignore\s+(all\s+)?previous\s+instructions/i,
    /you\s+are\s+now\s+a/i,
    /system\s*:\s*/i,
    /\boverride\b.*\b(config|settings|security|api.?key)\b/i,
    /\b(delete|drop|rm\s+-rf|exec|eval)\b.*\b(database|table|file|system)\b/i,
    /\bset\s+(api.?key|token|secret|password)\b/i,
  ];
  return patterns.some((p) => p.test(lower));
}

// --- AI Rules Engine v1 ---

const VALID_RULE_KINDS: AiRuleKind[] = ["process_country", "process_ip", "process_cidr", "process_port", "process_memory"];
const VALID_PROTOCOLS = ["any", "tcp", "udp"];

/**
 * Validates a single AI rule from parsed JSON.
 * Returns sanitized rule or throws on invalid data.
 */
export function validateAiRule(raw: unknown): AiRuleV1 {
  if (!raw || typeof raw !== "object") {
    throw new Error("Invalid AI rule: expected an object");
  }
  const r = raw as Record<string, unknown>;

  if (typeof r.id !== "string" || r.id.length === 0) {
    throw new Error("AI rule must have a non-empty string 'id'");
  }
  if (typeof r.name !== "string" || r.name.length === 0) {
    throw new Error("AI rule must have a non-empty string 'name'");
  }
  if (typeof r.enabled !== "boolean") {
    throw new Error("AI rule 'enabled' must be a boolean");
  }
  if (typeof r.kind !== "string" || !VALID_RULE_KINDS.includes(r.kind as AiRuleKind)) {
    throw new Error(`AI rule 'kind' must be one of: ${VALID_RULE_KINDS.join(", ")}`);
  }
  const protocol = typeof r.protocol === "string" ? r.protocol : "any";
  if (!VALID_PROTOCOLS.includes(protocol)) {
    throw new Error(`AI rule 'protocol' must be one of: ${VALID_PROTOCOLS.join(", ")}`);
  }

  return {
    id: r.id as string,
    name: r.name as string,
    enabled: r.enabled as boolean,
    kind: r.kind as AiRuleKind,
    process_contains: typeof r.process_contains === "string" ? r.process_contains : null,
    country_code: typeof r.country_code === "string" ? r.country_code : null,
    destination_ip: typeof r.destination_ip === "string" ? r.destination_ip : null,
    destination_cidr: typeof r.destination_cidr === "string" ? r.destination_cidr : null,
    destination_port: typeof r.destination_port === "number" ? r.destination_port : null,
    protocol: protocol as "any" | "tcp" | "udp",
    process_memory_mb_gt: typeof r.process_memory_mb_gt === "number" ? r.process_memory_mb_gt : null,
    mitre_technique_id: typeof r.mitre_technique_id === "string" ? r.mitre_technique_id : null,
  };
}

/**
 * Validates and builds a complete AI Rules v1 payload from parsed JSON.
 * Returns the payload string ready to send via invoke('apply_ai_rules').
 */
export function buildRulesPayload(rules: AiRuleV1[]): string {
  const payload: AiRulesPayload = {
    schema_version: 1,
    rules,
  };
  return JSON.stringify(payload);
}

/**
 * Builds the prompt to send to the AI for config changes.
 * Includes current config (sanitized, no API keys) and the user instruction.
 */
export function buildConfigPrompt(
  userInstruction: string,
  currentConfig: Record<string, unknown>,
): string {
  // Strip sensitive keys before sending to AI
  const safe: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(currentConfig)) {
    if (!IMMUTABLE_KEYS.has(k)) {
      safe[k] = v;
    }
  }

  return `You are a system configuration assistant for OmniMon, a process monitor.
The user wants to change a setting or create a security rule using natural language.

Current configuration (JSON):
${JSON.stringify(safe, null, 2)}

Available settings you can modify:
- idleThreshold (number 0.1-10.0): CPU% below which processes are considered idle
- fontSize (integer 8-48): UI font size in pixels
- theme ("auto"|"light"|"dark"|"cyberpunk"): UI theme
- locale ("auto"|"en"|"es"): UI language
- aiProfile ("general"|"developer"|"gaming"|"battery"): AI analysis profile
- columns (object with boolean values): Which columns are visible
- columnOrder (array of strings): Order of columns

You can also create alert rules with this structure:
{ "alerts": [{ "metric": "cpu"|"ram"|"net_rx"|"net_tx"|"swap", "operator": ">"|"<"|">="|"<=", "threshold": number, "processName": "optional", "action": "toast"|"sound"|"highlight" }] }

You can also create SECURITY RULES for the rules engine (schema v1). These block or alert on network activity:
{ "ai_rules": [{ "id": "unique-id", "name": "Rule name", "enabled": true, "kind": "process_country"|"process_ip"|"process_cidr"|"process_port"|"process_memory", "process_contains": "process name substring or null", "country_code": "CN|RU|... or null", "destination_ip": "1.2.3.4 or null", "destination_cidr": "10.0.0.0/8 or null", "destination_port": 8080 or null, "protocol": "any"|"tcp"|"udp", "process_memory_mb_gt": 1024 or null, "mitre_technique_id": "T1071 or null" }] }
Examples:
- Block connections to China: {"ai_rules":[{"id":"block-cn-001","name":"Block China traffic","enabled":true,"kind":"process_country","process_contains":null,"country_code":"CN","destination_ip":null,"destination_cidr":null,"destination_port":null,"protocol":"any","process_memory_mb_gt":null,"mitre_technique_id":"T1071"}]}
- Alert if node uses more than 1GB: {"ai_rules":[{"id":"node-mem-001","name":"Node memory alert","enabled":true,"kind":"process_memory","process_contains":"node","country_code":null,"destination_ip":null,"destination_cidr":null,"destination_port":null,"protocol":"any","process_memory_mb_gt":1024,"mitre_technique_id":"T1499"}]}

User instruction: "${userInstruction}"

Respond with ONLY a JSON object containing the changes. No explanation, no markdown.`;
}
