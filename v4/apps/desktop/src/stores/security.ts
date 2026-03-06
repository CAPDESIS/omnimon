/**
 * Security store: manages process threat labels, CVE matches,
 * and network connections.
 *
 * Currently provides a client-side heuristic engine since the
 * backend IPC commands for security/network are not yet exposed.
 * When they are, replace the heuristic functions with real IPC calls.
 */

import { writable, derived, get } from "svelte/store";
import type {
  ProcessEntry,
  ProcessSecurityInfo,
  ProcessThreatLabel,
  CveMatch,
  NetworkConnection,
  MitreTechnique,
  BehaviorIndicator,
} from "../lib/types";
import { processes } from "./processes";

// --- Stores ---

/** Map of PID -> security info (threats + CVEs). Updated each poll cycle. */
export const securityMap = writable<Map<number, ProcessSecurityInfo>>(new Map());

/** Active network connections per process. */
export const networkConnections = writable<NetworkConnection[]>([]);

// --- Derived ---

/** PIDs that have at least one threat or CVE. */
export const flaggedPids = derived(securityMap, ($map) => {
  const set = new Set<number>();
  for (const [pid, info] of $map) {
    if (info.threats.length > 0 || info.cves.length > 0) {
      set.add(pid);
    }
  }
  return set;
});

/** Total count of unique security findings. */
export const totalFindings = derived(securityMap, ($map) => {
  let count = 0;
  for (const info of $map.values()) {
    count += info.threats.length + info.cves.length;
  }
  return count;
});

// --- Heuristic engine ---
// These heuristics run client-side and detect suspicious patterns
// from process metadata. They will be replaced by real backend
// security analysis once IPC commands are exposed.

/** Known suspicious process name patterns mapped to MITRE techniques. */
const SUSPICIOUS_PATTERNS: Array<{
  pattern: RegExp;
  indicator: BehaviorIndicator;
  techniques: MitreTechnique[];
  minConfidence: number;
}> = [
  {
    pattern: /^(nc|ncat|netcat|socat)$/i,
    indicator: "RemoteThreadInjection",
    techniques: [{
      technique_id: "T1059.004",
      tactic: "Execution",
      name: "Unix Shell",
    }],
    minConfidence: 0.7,
  },
  {
    pattern: /^(mimikatz|lazagne|rubeus|sharphound)$/i,
    indicator: "SuspiciousMemoryRead",
    techniques: [{
      technique_id: "T1003",
      tactic: "Credential Access",
      name: "OS Credential Dumping",
    }],
    minConfidence: 0.9,
  },
  {
    pattern: /^(rundll32|regsvr32|mshta|cscript|wscript)$/i,
    indicator: "DllInjection",
    techniques: [{
      technique_id: "T1218",
      tactic: "Defense Evasion",
      name: "System Binary Proxy Execution",
    }],
    minConfidence: 0.7,
  },
  {
    pattern: /^(powershell|pwsh)$/i,
    indicator: "UnsignedModuleLoad",
    techniques: [{
      technique_id: "T1059.001",
      tactic: "Execution",
      name: "PowerShell",
    }],
    minConfidence: 0.5,
  },
];

/** Known CVE-affected products/versions. Static snapshot. */
const KNOWN_CVES: Array<{
  processPattern: RegExp;
  product: string;
  cve_id: string;
  severity: string;
  summary: string;
}> = [
  {
    processPattern: /^(log4j|java)$/i,
    product: "Apache Log4j",
    cve_id: "CVE-2021-44228",
    severity: "critical",
    summary: "Remote code execution via JNDI lookup in log messages (Log4Shell)",
  },
  {
    processPattern: /^(openssl)$/i,
    product: "OpenSSL",
    cve_id: "CVE-2024-5535",
    severity: "high",
    summary: "Buffer overread in SSL_select_next_proto",
  },
  {
    processPattern: /^(httpd|apache2)$/i,
    product: "Apache HTTP Server",
    cve_id: "CVE-2024-38476",
    severity: "high",
    summary: "Server-Side Request Forgery via backend applications",
  },
];

function analyzeProcessSecurity(proc: ProcessEntry): ProcessSecurityInfo {
  const threats: ProcessThreatLabel[] = [];
  const cves: CveMatch[] = [];

  // Check suspicious patterns
  for (const rule of SUSPICIOUS_PATTERNS) {
    if (rule.pattern.test(proc.name) || rule.pattern.test(proc.exec_name)) {
      threats.push({
        pid: proc.pid,
        process_name: proc.name,
        indicator: rule.indicator,
        mitre_techniques: rule.techniques,
        confidence: rule.minConfidence,
      });
    }
  }

  // Check known CVEs
  for (const cve of KNOWN_CVES) {
    if (cve.processPattern.test(proc.name) || cve.processPattern.test(proc.exec_name)) {
      cves.push({
        pid: proc.pid,
        process_name: proc.name,
        product: cve.product,
        detected_version: "unknown",
        cve_id: cve.cve_id,
        severity: cve.severity,
        summary: cve.summary,
      });
    }
  }

  return { pid: proc.pid, threats, cves };
}

/** Run security analysis on all processes. Call on each poll cycle. */
export function refreshSecurityAnalysis(procs: ProcessEntry[]): void {
  const map = new Map<number, ProcessSecurityInfo>();
  for (const proc of procs) {
    const info = analyzeProcessSecurity(proc);
    if (info.threats.length > 0 || info.cves.length > 0) {
      map.set(proc.pid, info);
    }
  }
  securityMap.set(map);
}

// --- Network connection heuristics ---
// Until the backend exposes per-process network data,
// we derive synthetic connection info from browser processes + tabs.

export function refreshNetworkConnections(
  procs: ProcessEntry[],
  tabs: { url: string; browser: string }[],
): void {
  const conns: NetworkConnection[] = [];

  // Derive connections from browser tabs
  for (const tab of tabs) {
    try {
      const url = new URL(tab.url);
      const browserProc = procs.find(
        (p) => p.group === "Browser" && p.name.toLowerCase().includes(tab.browser.toLowerCase()),
      );
      conns.push({
        pid: browserProc?.pid ?? 0,
        process_name: tab.browser,
        remote_addr: url.hostname,
        remote_port: url.port ? parseInt(url.port) : (url.protocol === "https:" ? 443 : 80),
        protocol: "tcp",
        direction: "outbound",
        bytes_sent: 0,
        bytes_recv: 0,
        state: "ESTABLISHED",
      });
    } catch {
      // Invalid URL, skip
    }
  }

  networkConnections.set(conns);
}

/** Get security info for a specific PID. */
export function getSecurityForPid(pid: number): ProcessSecurityInfo | undefined {
  return get(securityMap).get(pid);
}

/** Severity rank for sorting. */
export function severityRank(severity: string | null): number {
  switch (severity) {
    case "critical": return 4;
    case "high": return 3;
    case "medium": return 2;
    case "low": return 1;
    default: return 0;
  }
}

/** Severity CSS color variable name. */
export function severityColor(severity: string | null): string {
  switch (severity) {
    case "critical": return "var(--danger)";
    case "high": return "var(--danger)";
    case "medium": return "var(--yellow)";
    case "low": return "var(--fg-dim)";
    default: return "var(--fg-dim)";
  }
}

export function _resetSecurity(): void {
  securityMap.set(new Map());
  networkConnections.set([]);
}
