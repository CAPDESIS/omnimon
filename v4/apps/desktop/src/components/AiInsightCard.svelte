<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { securityMap, severityColor } from "../stores/security";
  import { dynamicAlerts } from "../stores/alerts";
  import { t } from "../lib/i18n";
  import { renderMarkdown } from "../lib/markdown";
  import { formatDynamicAlertMessage } from "../lib/localizedUi";
  import type { ProcessThreatLabel, CveMatch } from "../lib/types";

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
          explanation: cve.summary ?? t("insights.noAdditionalDetails"),
          action: t("insights.cveAction", { product: cve.product }),
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
        return t("insights.threatHeadline.suspiciousMemoryRead", { process: threat.process_name });
      case "DllInjection":
        return t("insights.threatHeadline.dllInjection", { process: threat.process_name });
      case "RemoteThreadInjection":
        return t("insights.threatHeadline.remoteThreadInjection", { process: threat.process_name });
      case "UnsignedModuleLoad":
        return t("insights.threatHeadline.unsignedModuleLoad", { process: threat.process_name });
      case "ProcessHollowing":
        return t("insights.threatHeadline.processHollowing", { process: threat.process_name });
      default:
        return t("insights.threatHeadline.default", { process: threat.process_name });
    }
  }

  function explainThreat(threat: ProcessThreatLabel): string {
    const technique = threat.mitre_techniques[0];
    if (!technique) return t("insights.threatExplanation.default");
    return t("insights.threatExplanation.technique", {
      name: technique.name,
      tactic: technique.tactic,
      techniqueId: technique.technique_id,
    });
  }

  function suggestAction(threat: ProcessThreatLabel): string {
    switch (threat.indicator) {
      case "SuspiciousMemoryRead":
        return t("insights.threatAction.suspiciousMemoryRead");
      case "DllInjection":
        return t("insights.threatAction.dllInjection");
      case "RemoteThreadInjection":
        return t("insights.threatAction.remoteThreadInjection");
      case "UnsignedModuleLoad":
        return t("insights.threatAction.unsignedModuleLoad");
      case "ProcessHollowing":
        return t("insights.threatAction.processHollowing");
      default:
        return t("insights.threatAction.default");
    }
  }

  function translateCve(cve: CveMatch): string {
    if (cve.severity === "critical") {
      return t("insights.cveHeadline.critical", { product: cve.product });
    }
    if (cve.severity === "high") {
      return t("insights.cveHeadline.high", { product: cve.product });
    }
    return t("insights.cveHeadline.default", { product: cve.product });
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
    if (c >= 0.9) return t("insights.confidenceLevels.veryHigh");
    if (c >= 0.7) return t("insights.confidenceLevels.high");
    if (c >= 0.5) return t("insights.confidenceLevels.moderate");
    return t("insights.confidenceLevels.low");
  }

  function severityLabel(severity: InsightItem["severity"]): string {
    switch (severity) {
      case "critical":
        return t("securityReport.severityCritical");
      case "high":
        return t("securityReport.severityHigh");
      case "medium":
        return t("securityReport.severityMedium");
      case "low":
        return t("securityReport.severityLow");
      default:
        return severity;
    }
  }

  // --- Dynamic Alerts from Rust Rules Engine ---
  let ruleAlerts = $derived.by((): InsightItem[] => {
    return $dynamicAlerts.map((a) => {
      let naturalExplanation = t("insights.dynamicRuleExplanation.default", {
        rule: a.rule_name,
        destination: `${a.dst_ip}:${a.dst_port}`,
        country: a.country_code ? ` (${a.country_code})` : "",
      });
      if (a.rule_name.toLowerCase().includes("download") || a.dst_port === 443 || a.dst_port === 80) {
        naturalExplanation = t("insights.dynamicRuleExplanation.download");
      } else if (a.rule_name.toLowerCase().includes("memory")) {
        naturalExplanation = t("insights.dynamicRuleExplanation.memory");
      } else if (a.country_code) {
        naturalExplanation = t("insights.dynamicRuleExplanation.country", { country: a.country_code });
      }

      return {
        kind: "rule" as const,
        pid: a.pid,
        processName: a.process_name,
        severity: "high" as const,
        headline: formatDynamicAlertMessage(a),
        explanation: naturalExplanation,
        action: t("insights.dynamicRuleAction", { pid: a.pid }),
        techniqueId: a.mitre_technique_id ?? undefined,
        ruleId: a.rule_id,
        confidence: 0.95,
      };
    });
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
  <div class="insight-section" role="region" aria-label={t("insights.title")}>
    <div class="insight-header">
      <svg class="insight-icon" viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
        <path d="M8 1a5 5 0 013.5 8.6V12a1 1 0 01-1 1h-5a1 1 0 01-1-1V9.6A5 5 0 018 1zm-1.5 13h3a.5.5 0 010 1h-3a.5.5 0 010-1z"/>
      </svg>
      <span class="insight-title">{t("insights.title")}</span>
      <span class="insight-count">{t(allInsights.length === 1 ? "insights.findingSingular" : "insights.findingPlural", { count: allInsights.length })}</span>
    </div>

    <div class="insight-cards">
      {#each allInsights as insight, i (insight.kind + "-" + insight.pid + "-" + (insight.techniqueId ?? insight.cveId ?? insight.ruleId ?? i))}
        {@const key = insight.kind + insight.pid + (insight.techniqueId ?? insight.cveId ?? insight.ruleId ?? i)}
        {@const isExpanded = expanded.has(key)}
        <div
          class="insight-card severity-{insight.severity}"
          role="article"
          aria-label={t("insights.cardAria", { severity: insight.severity, process: insight.processName })}
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
                  <span class="detail-label">{t("insights.detected")}</span>
                  <div class="detail-text prose">{@html renderMarkdown(insight.explanation)}</div>
                </div>
              </div>

              <div class="detail-row">
                <span class="detail-icon">&#128736;</span>
                <div class="detail-content">
                  <span class="detail-label">{t("insights.recommendedAction")}</span>
                  <div class="detail-text action-text prose">{@html renderMarkdown(insight.action)}</div>
                </div>
              </div>

              <div class="insight-meta">
                <span class="meta-chip">
                  {insight.kind === "threat" ? "MITRE" : insight.kind === "cve" ? "CVE" : t("insights.ruleLabel")}:
                  {insight.techniqueId ?? insight.cveId ?? insight.ruleId}
                </span>
                <span class="meta-chip">PID {insight.pid}</span>
                <span class="meta-chip">{t("insights.confidence")}: {confidenceLabel(insight.confidence)}</span>
                <span class="meta-chip severity-chip" style="color: {severityColor(insight.severity)}; border-color: {severityColor(insight.severity)}">
                  {severityLabel(insight.severity)}
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
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
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
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
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

  .detail-text :global(p) { margin: 0 0 4px; }
  .detail-text :global(p:last-child) { margin-bottom: 0; }
  .detail-text :global(strong) { color: var(--fg); font-weight: 700; }
  .detail-text :global(em) { font-style: italic; color: var(--fg-dim); }
  .detail-text :global(ul) { margin: 4px 0; padding-left: 18px; list-style: disc; }
  .detail-text :global(li) { margin: 2px 0; }
  .detail-text :global(pre), .detail-text :global(code) { background: var(--bg-hover); padding: 2px 4px; border-radius: 4px; font-family: monospace; }

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
