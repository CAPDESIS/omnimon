<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { securityMap, severityColor } from "../stores/security";
  import { processes } from "../stores/processes";
  import { dynamicAlerts } from "../stores/alerts";
  import type { ProcessSecurityInfo, ProcessThreatLabel, CveMatch, DynamicAlert } from "../lib/types";

  let insights = $derived.by((): InsightItem[] => {
    const items: InsightItem[] = [];
    for (const [pid, info] of $securityMap) {
      for (const threat of info.threats) {
        items.push({
          kind: "threat",
          pid,
          processName: threat.process_name,
          severity: threat.confidence >= 0.8 ? "high" : "medium",
          headline: translateThreat(threat),
          explanation: explainThreat(threat),
          action: suggestAction(threat),
          techniqueId: threat.mitre_techniques[0]?.technique_id,
          confidence: threat.confidence,
        });
      }
      for (const cve of info.cves) {
        items.push({
          kind: "cve",
          pid,
          processName: cve.process_name,
          severity: cve.severity as "critical" | "high" | "medium" | "low",
          headline: translateCve(cve),
          explanation: cve.summary ?? "No additional details available.",
          action: `Update ${cve.product} to the latest patched version.`,
          cveId: cve.cve_id,
          confidence: 1.0,
        });
      }
    }
    return items.sort((a, b) => severityWeight(b.severity) - severityWeight(a.severity));
  });

  interface InsightItem {
    kind: "threat" | "cve" | "rule";
    pid: number;
    processName: string;
    severity: "critical" | "high" | "medium" | "low";
    headline: string;
    explanation: string;
    action: string;
    techniqueId?: string;
    cveId?: string;
    ruleId?: string;
    confidence: number;
  }

  function severityWeight(s: string): number {
    switch (s) {
      case "critical": return 4;
      case "high": return 3;
      case "medium": return 2;
      case "low": return 1;
      default: return 0;
    }
  }

  function translateThreat(threat: ProcessThreatLabel): string {
    switch (threat.indicator) {
      case "SuspiciousMemoryRead":
        return `"${threat.process_name}" is trying to read memory from other apps. This is how attackers steal passwords.`;
      case "DllInjection":
        return `"${threat.process_name}" uses a system tool that attackers often abuse to hide malicious code.`;
      case "RemoteThreadInjection":
        return `"${threat.process_name}" can create remote connections. Attackers use this to control your computer from afar.`;
      case "UnsignedModuleLoad":
        return `"${threat.process_name}" can run arbitrary scripts. Without restrictions, it could execute harmful commands.`;
      case "ProcessHollowing":
        return `"${threat.process_name}" may be replacing a legitimate app's code in memory with something malicious.`;
      default:
        return `"${threat.process_name}" shows unusual behavior that could indicate a security threat.`;
    }
  }

  function explainThreat(threat: ProcessThreatLabel): string {
    const technique = threat.mitre_techniques[0];
    if (!technique) return "Anomalous behavior detected by the security engine.";
    return `Detected pattern: ${technique.name} (${technique.tactic}). This maps to MITRE ATT&CK technique ${technique.technique_id}.`;
  }

  function suggestAction(threat: ProcessThreatLabel): string {
    switch (threat.indicator) {
      case "SuspiciousMemoryRead":
        return "Terminate this process immediately and run a malware scan. Do not enter passwords until resolved.";
      case "DllInjection":
        return "Verify this was launched intentionally. If not, terminate it and check for recently installed software.";
      case "RemoteThreadInjection":
        return "Check your network connections. If you didn't start this, block it in your firewall and terminate.";
      case "UnsignedModuleLoad":
        return "Review recent script executions. Enable script execution policies if on Windows.";
      case "ProcessHollowing":
        return "This process should be terminated immediately. Run a full system antivirus scan.";
      default:
        return "Investigate this process and verify it's operating within expected parameters.";
    }
  }

  function translateCve(cve: CveMatch): string {
    if (cve.severity === "critical") {
      return `Critical vulnerability found in ${cve.product}! Attackers could take full control of your system.`;
    }
    if (cve.severity === "high") {
      return `Serious vulnerability found in ${cve.product}. Your system may be exposed to attacks.`;
    }
    return `A known vulnerability was found in ${cve.product}. Consider updating when possible.`;
  }

  function severityIcon(s: string): string {
    switch (s) {
      case "critical": return "\u26D4"; // no entry
      case "high": return "\u26A0";    // warning
      case "medium": return "\u2139";  // info
      default: return "\u2022";        // bullet
    }
  }

  function confidenceLabel(c: number): string {
    if (c >= 0.9) return "Very High";
    if (c >= 0.7) return "High";
    if (c >= 0.5) return "Moderate";
    return "Low";
  }

  // --- Dynamic Alerts from Rust Rules Engine ---
  let ruleAlerts = $derived.by((): InsightItem[] => {
    return $dynamicAlerts.map((a) => ({
      kind: "rule" as const,
      pid: a.pid,
      processName: a.process_name,
      severity: "high" as const,
      headline: a.message || `"${a.process_name}" triggered rule "${a.rule_name}"`,
      explanation: `Security rule "${a.rule_name}" fired. Connection to ${a.dst_ip}:${a.dst_port}${a.country_code ? ` (${a.country_code})` : ""}.`,
      action: `Review network activity for PID ${a.pid}. Consider blocking this connection or disabling the process.`,
      techniqueId: a.mitre_technique_id ?? undefined,
      ruleId: a.rule_id,
      confidence: 0.95,
    }));
  });

  let allInsights = $derived([...insights, ...ruleAlerts]);

  let expanded = $state<Set<string>>(new Set());

  function toggleExpand(key: string) {
    const next = new Set(expanded);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expanded = next;
  }
</script>

{#if allInsights.length > 0}
  <div class="insight-section" role="region" aria-label="AI Security Insights">
    <div class="insight-header">
      <svg class="insight-icon" viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
        <path d="M8 1a5 5 0 013.5 8.6V12a1 1 0 01-1 1h-5a1 1 0 01-1-1V9.6A5 5 0 018 1zm-1.5 13h3a.5.5 0 010 1h-3a.5.5 0 010-1z"/>
      </svg>
      <span class="insight-title">AI Security Insights</span>
      <span class="insight-count">{allInsights.length} finding{allInsights.length !== 1 ? "s" : ""}</span>
    </div>

    <div class="insight-cards">
      {#each allInsights as insight, i (insight.kind + "-" + insight.pid + "-" + (insight.techniqueId ?? insight.cveId ?? insight.ruleId ?? i))}
        {@const key = insight.kind + insight.pid + (insight.techniqueId ?? insight.cveId ?? insight.ruleId ?? i)}
        {@const isExpanded = expanded.has(key)}
        <div
          class="insight-card severity-{insight.severity}"
          role="article"
          aria-label="{insight.severity} security insight for {insight.processName}"
          in:fly={{ y: -20, duration: 250 }}
          out:fade={{ duration: 150 }}
        >
          <button class="insight-card-header" onclick={() => toggleExpand(key)}>
            <span class="severity-indicator" style="color: {severityColor(insight.severity)}">
              {severityIcon(insight.severity)}
            </span>
            <span class="insight-headline">{insight.headline}</span>
            <span class="insight-chevron" class:open={isExpanded}>&#9654;</span>
          </button>

          {#if isExpanded}
            <div class="insight-detail">
              <div class="detail-row">
                <span class="detail-icon">&#128269;</span>
                <div class="detail-content">
                  <span class="detail-label">What was detected</span>
                  <p class="detail-text">{insight.explanation}</p>
                </div>
              </div>

              <div class="detail-row">
                <span class="detail-icon">&#128736;</span>
                <div class="detail-content">
                  <span class="detail-label">Recommended action</span>
                  <p class="detail-text action-text">{insight.action}</p>
                </div>
              </div>

              <div class="insight-meta">
                <span class="meta-chip">
                  {insight.kind === "threat" ? "MITRE" : insight.kind === "cve" ? "CVE" : "RULE"}:
                  {insight.techniqueId ?? insight.cveId ?? insight.ruleId}
                </span>
                <span class="meta-chip">PID {insight.pid}</span>
                <span class="meta-chip">Confidence: {confidenceLabel(insight.confidence)}</span>
                <span class="meta-chip severity-chip" style="color: {severityColor(insight.severity)}; border-color: {severityColor(insight.severity)}">
                  {insight.severity.toUpperCase()}
                </span>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .insight-section {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-alt);
  }

  .insight-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
  }

  .insight-icon {
    color: var(--yellow);
  }

  .insight-title {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--yellow);
  }

  .insight-count {
    margin-left: auto;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .insight-cards {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .insight-card {
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.08));
    will-change: transform, opacity;
  }

  .insight-card.severity-critical {
    border-left: 3px solid var(--danger);
  }
  .insight-card.severity-high {
    border-left: 3px solid var(--danger);
  }
  .insight-card.severity-medium {
    border-left: 3px solid var(--yellow);
  }
  .insight-card.severity-low {
    border-left: 3px solid var(--fg-dim);
  }

  .insight-card-header {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    text-align: left;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    line-height: 1.4;
  }
  .insight-card-header:hover {
    background: var(--bg-hover);
  }

  .severity-indicator {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 1.083);
    line-height: 1;
    padding-top: 1px;
  }

  .insight-headline {
    flex: 1;
  }

  .insight-chevron {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
    padding-top: 3px;
  }
  .insight-chevron.open {
    transform: rotate(90deg);
  }

  .insight-detail {
    padding: 0 10px 10px 28px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .detail-row {
    display: flex;
    gap: 8px;
  }

  .detail-icon {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    padding-top: 2px;
  }

  .detail-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .detail-label {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
  }

  .detail-text {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    color: var(--fg);
    line-height: 1.5;
  }

  .action-text {
    color: var(--green);
  }

  .insight-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding-top: 4px;
  }

  .meta-chip {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 3px;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-weight: 600;
    background: var(--bg);
    color: var(--fg-dim);
    border: 1px solid var(--border);
  }

  .severity-chip {
    background: transparent;
    font-weight: 800;
    letter-spacing: 0.3px;
  }
</style>
