<script lang="ts">
  import { onMount } from "svelte";
  import { securityMap, flaggedPids, severityColor, severityRank } from "../stores/security";
  import { processes } from "../stores/processes";
  import { slide } from "svelte/transition";
  import { t } from "../lib/i18n";
  import { renderMarkdown } from "../lib/markdown";
  import type { NistFinding, NistSeverity } from "../lib/types";
  import { focusFirstFocusable, trapFocus } from "../lib/focusTrap";
  import EmptyState from "./EmptyState.svelte";
  import { Shield } from "lucide-svelte";
  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";
  import ModalShell from "./ModalShell.svelte";


  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
  let modalEl: HTMLDivElement | undefined = $state();
  let quickScanning = $state(false);
  let quickScanAt = $state<string | null>(null);

  // Build NIST-style findings from security map
  let findings = $derived.by((): NistFinding[] => {
    const result: NistFinding[] = [];
    for (const [pid, info] of $securityMap) {
      for (const threat of info.threats) {
        result.push({
          id: `T-${pid}-${threat.indicator}`,
          category: "threat",
          severity: threat.confidence >= 0.8 ? "high" : threat.confidence >= 0.5 ? "medium" : "low",
          title: formatIndicator(threat.indicator),
          description: t("securityReport.threatDescription", {
            process: threat.process_name,
            pid,
            techniques: threat.mitre_techniques.map((technique) => technique.name).join(", "),
          }),
          affected_process: threat.process_name,
          pid,
          mitre_id: threat.mitre_techniques[0]?.technique_id,
          recommendation: getRecommendation("threat", threat.indicator),
        });
      }
      for (const cve of info.cves) {
        result.push({
          id: `C-${pid}-${cve.cve_id}`,
          category: "vulnerability",
          severity: (cve.severity as NistSeverity) ?? "medium",
          title: cve.cve_id,
          description: cve.summary ?? t("securityReport.knownVulnerability", { product: cve.product }),
          affected_process: cve.process_name,
          pid,
          cve_id: cve.cve_id,
          recommendation: getRecommendation("cve", cve.product),
        });
      }
    }
    return result.sort((a, b) => severityRank(b.severity) - severityRank(a.severity));
  });

  let riskScore = $derived.by(() => {
    if (findings.length === 0) return 0;
    let score = 0;
    for (const f of findings) {
      score += severityRank(f.severity) * 15;
    }
    return Math.min(100, score);
  });

  let criticalCount = $derived(findings.filter((f) => f.severity === "critical").length);
  let highCount = $derived(findings.filter((f) => f.severity === "high").length);
  let mediumCount = $derived(findings.filter((f) => f.severity === "medium").length);
  let lowCount = $derived(findings.filter((f) => f.severity === "low").length);

  function formatIndicator(indicator: string): string {
    return indicator.replace(/([A-Z])/g, " $1").trim();
  }

  function getRecommendation(type: string, detail: string): string {
    if (type === "threat") {
      switch (detail) {
        case "SuspiciousMemoryRead": return t("securityReport.recommendationThreatSuspiciousMemoryRead");
        case "DllInjection": return t("securityReport.recommendationThreatDllInjection");
        case "RemoteThreadInjection": return t("securityReport.recommendationThreatRemoteThreadInjection");
        case "UnsignedModuleLoad": return t("securityReport.recommendationThreatUnsignedModuleLoad");
        default: return t("securityReport.recommendationThreatDefault");
      }
    }
    return t("securityReport.recommendationCve", { detail });
  }

  function riskLabel(score: number): string {
    if (score >= 75) return t("securityReport.critical");
    if (score >= 50) return t("securityReport.high");
    if (score >= 25) return t("securityReport.moderate");
    if (score > 0) return t("securityReport.low");
    return t("securityReport.none");
  }

  function riskColor(score: number): string {
    if (score >= 75) return "var(--danger)";
    if (score >= 50) return "var(--yellow)";
    if (score >= 25) return "var(--yellow)";
    return "var(--green)";
  }

  function severityLabel(severity: string): string {
    switch (severity) {
      case "critical": return t("securityReport.severityCritical");
      case "high": return t("securityReport.severityHigh");
      case "medium": return t("securityReport.severityMedium");
      case "low": return t("securityReport.severityLow");
      default: return t("securityReport.severityInfo");
    }
  }

  let expandedIds = $state(new Set<string>());
  let generatedAt = $state("");

  function closeWhenBackdropMatches(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }

  function stopMouseEventPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  function toggleExpand(id: string) {
    const next = new Set(expandedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedIds = next;
  }

  let quickScanResults = $state<{ suspicious: number; networkIssues: number; highMem: number; highCpu: number }>({ suspicious: 0, networkIssues: 0, highMem: 0, highCpu: 0 });

  async function runQuickScan() {
    quickScanning = true;
    // Actually analyze processes for issues
    await new Promise((resolve) => setTimeout(resolve, 200));
    const procs = $processes;
    let suspicious = 0;
    let networkIssues = 0;
    let highMem = 0;
    let highCpu = 0;

    const suspiciousPatterns = /nc|netcat|mimikatz|powershell|cmd\.exe|nmap|metasploit|hydra|john|hashcat/i;
    for (const proc of procs) {
      if (suspiciousPatterns.test(proc.name) || suspiciousPatterns.test(proc.exec_name)) suspicious++;
      if (proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec > 10 * 1024 * 1024) networkIssues++;
      if (proc.ram_mb > 2048) highMem++;
      if (proc.cpu_pct > 80) highCpu++;
    }

    quickScanResults = { suspicious, networkIssues, highMem, highCpu };
    quickScanAt = new Date().toLocaleTimeString();
    generatedAt = new Date().toLocaleTimeString();
    quickScanning = false;
  }

  $effect(() => {
    findings;
    if (!generatedAt) {
      generatedAt = new Date().toLocaleTimeString();
    }
  });

  onMount(() => {
    requestAnimationFrame(() => focusFirstFocusable(modalEl));
  });

  function handleDialogKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") return;
    trapFocus(event, modalEl);
  }
</script>

<svelte:window onkeydown={handleDialogKeydown} />

<ModalShell titleId="report-title" backdropClass="report-backdrop" panelClass="report-modal" onclose={onclose} width="560px" maxHeight="85vh">
  <div bind:this={modalEl}>
    <div class="report-header">
      <div class="report-title-row">
        <h2 id="report-title" class="report-title">{t("securityReport.title")}</h2>
        <span class="report-subtitle">{t("securityReport.subtitle")}</span>
      </div>
      <IconButton class="report-close" onclick={onclose} ariaLabel={t("common.close")} title={t("common.close")}>×</IconButton>
    </div>

    <div class="report-body">
      <div class="quick-scan-bar">
        <Button class="quick-scan-btn" variant="primary" size="sm" onclick={runQuickScan} disabled={quickScanning}>
          {quickScanning ? t("securityReport.scanning") : t("securityReport.quickScan")}
        </Button>
        {#if quickScanAt}
          <span class="quick-scan-meta">{t("securityReport.lastQuickScan", { time: quickScanAt })}</span>
        {/if}
      </div>

      {#if quickScanning}
        <div class="quick-scan-results quick-scan-results-loading" aria-hidden="true">
          {#each Array(4) as _, index}
            <div class="scan-stat scan-stat-skeleton" style={`animation-delay:${index * 70}ms`}>
              <span class="scan-stat-value skeleton-block"></span>
              <span class="scan-stat-label skeleton-line"></span>
            </div>
          {/each}
        </div>
      {:else if quickScanAt}
        <div class="quick-scan-results">
          <div class="scan-stat" style="color: {quickScanResults.suspicious > 0 ? 'var(--danger)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.suspicious}</span>
            <span class="scan-stat-label">{t("securityReport.suspiciousProcesses")}</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.networkIssues > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.networkIssues}</span>
            <span class="scan-stat-label">{t("securityReport.highBandwidth")}</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.highMem > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.highMem}</span>
            <span class="scan-stat-label">{t("securityReport.highMemory")}</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.highCpu > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.highCpu}</span>
            <span class="scan-stat-label">{t("securityReport.highCpu")}</span>
          </div>
        </div>
      {/if}

      <div class="risk-overview">
        <div class="risk-gauge">
          <div class="risk-score" style="color: {riskColor(riskScore)}">{riskScore}</div>
          <div class="risk-label" style="color: {riskColor(riskScore)}">{riskLabel(riskScore)} {t("securityReport.riskSuffix")}</div>
        </div>
        <div class="risk-breakdown">
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--danger)">{criticalCount}</span>
            <span class="risk-stat-label">{t("securityReport.critical")}</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--danger)">{highCount}</span>
            <span class="risk-stat-label">{t("securityReport.high")}</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--yellow)">{mediumCount}</span>
            <span class="risk-stat-label">{t("securityReport.medium")}</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--fg-dim)">{lowCount}</span>
            <span class="risk-stat-label">{t("securityReport.low")}</span>
          </div>
        </div>
        <div class="risk-summary">
          {#if findings.length === 0}
            {t("securityReport.healthy")}
          {:else}
            {t("securityReport.summary", { findings: findings.length, processes: $flaggedPids.size })}
          {/if}
        </div>
      </div>

      {#if findings.length > 0}
        <div class="findings-section">
          <h3 class="section-title">{t("securityReport.findings")}</h3>
          {#each findings as finding (finding.id)}
            <div class="finding-card">
              <button class="finding-header" onclick={() => toggleExpand(finding.id)}>
                <span class="finding-severity" style="color: {severityColor(finding.severity)}; border-color: {severityColor(finding.severity)}">
                  {severityLabel(finding.severity)}
                </span>
                <span class="finding-title">{finding.title}</span>
                {#if finding.mitre_id}
                  <span class="finding-tag mitre">{finding.mitre_id}</span>
                {/if}
                {#if finding.cve_id}
                  <span class="finding-tag cve">{finding.cve_id}</span>
                {/if}
                <span class="finding-process">{finding.affected_process} (PID {finding.pid})</span>
                <span class="finding-chevron" class:open={expandedIds.has(finding.id)}>&#9654;</span>
              </button>

              {#if expandedIds.has(finding.id)}
                <div class="finding-detail" transition:slide={{ duration: 150 }}>
                  <div class="detail-section">
                    <span class="detail-label">{t("securityReport.whatHappened")}</span>
                    <div class="detail-text prose">{@html renderMarkdown(finding.description)}</div>
                  </div>
                  <div class="detail-section">
                    <span class="detail-label">{t("securityReport.whatToDo")}</span>
                    <div class="detail-text recommendation prose">{@html renderMarkdown(finding.recommendation)}</div>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <EmptyState icon={Shield} title={t("securityReport.noFindings")} description={t("securityReport.noFindingsDesc")} />
      {/if}

      <div class="meta-section">
        <span class="meta-label">{t("securityReport.scanned")}</span>
        <span class="meta-value">{t("securityReport.processCount", { count: $processes.length })}</span>
        <span class="meta-sep">·</span>
        <span class="meta-label">{t("securityReport.flagged")}</span>
        <span class="meta-value">{$flaggedPids.size}</span>
        <span class="meta-sep">·</span>
        <span class="meta-label">{t("securityReport.generated")}</span>
        <span class="meta-value">{generatedAt}</span>
      </div>
    </div>
  </div>
</ModalShell>

<style>
  .report-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
  }

  .report-title-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .report-title {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 1.25);
    font-weight: 700;
    color: var(--fg);
  }

  .report-subtitle {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--fg-dim);
  }

  :global(.report-close) {
    font-size: 18px;
  }

  .report-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .quick-scan-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }

  :global(.quick-scan-btn) {
    border: 1px solid var(--accent);
    background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 62%, white 18%));
    color: white;
    border-radius: 8px;
    padding: 8px 12px;
    font-weight: 700;
    cursor: pointer;
    transition: transform 0.18s ease, box-shadow 0.18s ease, filter 0.18s ease;
  }

  :global(.quick-scan-btn:hover:not(:disabled)) {
    transform: translateY(-1px);
    box-shadow: 0 12px 22px rgba(0, 0, 0, 0.18);
    filter: saturate(1.08);
  }

  :global(.quick-scan-btn:disabled) {
    cursor: progress;
    filter: brightness(0.88);
  }

  .quick-scan-meta {
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    color: var(--fg-dim);
  }

  .quick-scan-results {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg) 88%, white 5%), var(--bg));
  }

  .quick-scan-results-loading {
    pointer-events: none;
  }

  .scan-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    min-height: 72px;
    justify-content: center;
  }

  .scan-stat-skeleton {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: color-mix(in srgb, var(--bg-alt) 88%, white 2%);
  }

  .scan-stat-value {
    font-size: calc(var(--base-font-size, 12px) * 1.5);
    font-weight: 700;
    font-family: "SF Mono", "Menlo", monospace;
  }

  .scan-stat-label {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    text-align: center;
    color: var(--fg-dim);
  }

  .skeleton-block,
  .skeleton-line {
    display: block;
    background: var(--border);
    background-size: 220px 100%, 100% 100%;
    animation: shimmer 1.2s linear infinite;
  }

  .skeleton-block {
    width: 48px;
    height: 24px;
    border-radius: 8px;
  }

  .skeleton-line {
    width: 90px;
    height: 10px;
    border-radius: 999px;
  }

  /* Risk Overview */
  .risk-overview {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
  }

  .risk-gauge {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .risk-score {
    font-size: calc(var(--base-font-size, 12px) * 3);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", monospace;
    line-height: 1;
  }

  .risk-label {
    font-size: calc(var(--base-font-size, 12px) * 1.083);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .risk-breakdown {
    display: flex;
    gap: 16px;
  }

  .risk-stat {
    display: flex;
    align-items: baseline;
    gap: 4px;
  }

  .risk-count {
    font-size: calc(var(--base-font-size, 12px) * 1.5);
    font-weight: 700;
    font-family: "SF Mono", "Menlo", monospace;
  }

  .risk-stat-label {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .risk-summary {
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    color: var(--fg-dim);
    line-height: 1.5;
  }

  /* Findings */
  .findings-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-title {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--fg-dim);
  }

  .finding-card {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    overflow: hidden;
    background: color-mix(in srgb, var(--bg) 92%, white 3%);
    transition: transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
  }

  .finding-card:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--accent) 30%, var(--border));
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.12);
  }

  .finding-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: transparent;
    color: var(--fg);
    cursor: pointer;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    text-align: left;
  }
  .finding-header:hover { background: var(--bg-hover); }

  .finding-header:focus-visible,
  :global(.report-close:focus-visible),
  :global(.quick-scan-btn:focus-visible) {
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, var(--bg));
  }

  .finding-severity {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.583);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 1px 5px;
    border: 1px solid;
    border-radius: 3px;
    min-width: 52px;
    text-align: center;
  }

  .finding-title {
    font-weight: 600;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .finding-tag {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-family: "SF Mono", "Menlo", monospace;
    padding: 0 4px;
    border-radius: 2px;
    font-weight: 600;
  }
  .finding-tag.mitre { background: color-mix(in srgb, var(--danger) 28%, var(--bg)); color: var(--danger); }
  .finding-tag.cve { background: color-mix(in srgb, var(--yellow) 28%, var(--bg)); color: var(--yellow); }

  .finding-process {
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    font-family: "SF Mono", "Menlo", monospace;
  }

  .finding-chevron {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
    flex-shrink: 0;
  }
  .finding-chevron.open { transform: rotate(90deg); }

  .finding-detail {
    padding: 0 10px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--border-subtle, #2a2a3a);
  }

  .detail-section {
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
    padding-top: 6px;
  }

  .detail-text {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
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

  .recommendation {
    color: var(--green);
  }

  /* Meta */
  .meta-section {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
  }

  .meta-label {
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    font-weight: 600;
  }

  .meta-value {
    color: var(--fg);
    font-family: "SF Mono", "Menlo", monospace;
  }

  .meta-sep {
    color: var(--fg-dim);
  }

  @keyframes report-enter {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes shimmer {
    from {
      background-position: -220px 0, 0 0;
    }
    to {
      background-position: 220px 0, 0 0;
    }
  }

  @media (max-width: 720px) {
    :global(.report-modal) {
      width: min(92vw, 560px);
    }

    .quick-scan-results {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .risk-breakdown,
    .meta-section {
      flex-wrap: wrap;
    }

    .finding-header {
      flex-wrap: wrap;
    }
  }
</style>
