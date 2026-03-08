<script lang="ts">
  import { securityMap, totalFindings, flaggedPids, severityColor, severityRank } from "../stores/security";
  import { processes } from "../stores/processes";
  import { slide } from "svelte/transition";
  import type { ProcessSecurityInfo, NistFinding, NistSeverity } from "../lib/types";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
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
          description: `Process "${threat.process_name}" (PID ${pid}) exhibits behavior consistent with ${threat.mitre_techniques.map((t) => t.name).join(", ")}.`,
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
          description: cve.summary ?? `Known vulnerability in ${cve.product}.`,
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
        case "SuspiciousMemoryRead": return "Investigate this process immediately. It may be attempting to dump credentials from memory. Consider terminating it and scanning for malware.";
        case "DllInjection": return "This process uses a system binary commonly abused for code execution. Verify it was launched intentionally and check its parent process.";
        case "RemoteThreadInjection": return "This tool can establish reverse shells. Verify network connections and ensure it's being used for authorized purposes.";
        case "UnsignedModuleLoad": return "PowerShell can execute arbitrary scripts. Review recent command history and ensure execution policies are configured properly.";
        default: return "Review this process and verify it's operating within expected parameters.";
      }
    }
    return `Update ${detail} to the latest version. Check vendor advisories for patches addressing this vulnerability.`;
  }

  function riskLabel(score: number): string {
    if (score >= 75) return "Critical";
    if (score >= 50) return "High";
    if (score >= 25) return "Moderate";
    if (score > 0) return "Low";
    return "None";
  }

  function riskColor(score: number): string {
    if (score >= 75) return "var(--danger)";
    if (score >= 50) return "var(--yellow)";
    if (score >= 25) return "var(--yellow)";
    return "var(--green)";
  }

  function severityLabel(severity: string): string {
    switch (severity) {
      case "critical": return "CRITICAL";
      case "high": return "HIGH";
      case "medium": return "MEDIUM";
      case "low": return "LOW";
      default: return "INFO";
    }
  }

  let expandedIds = $state(new Set<string>());

  function toggleExpand(id: string) {
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
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
    quickScanning = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="report-backdrop" onclick={onclose} onkeydown={(e) => { if (e.key === "Escape") onclose(); }} role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="report-modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="report-title">
    <div class="report-header">
      <div class="report-title-row">
        <h2 id="report-title" class="report-title">Security Report</h2>
        <span class="report-subtitle">NIST Framework Assessment</span>
      </div>
      <button class="report-close" onclick={onclose} aria-label="Close">&times;</button>
    </div>

    <div class="report-body">
      <div class="quick-scan-bar">
        <button class="quick-scan-btn" onclick={runQuickScan} disabled={quickScanning}>
          {quickScanning ? "Scanning..." : "Quick scan"}
        </button>
        {#if quickScanAt}
          <span class="quick-scan-meta">Last quick scan: {quickScanAt}</span>
        {/if}
      </div>

      {#if quickScanAt}
        <div class="quick-scan-results">
          <div class="scan-stat" style="color: {quickScanResults.suspicious > 0 ? 'var(--danger)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.suspicious}</span>
            <span class="scan-stat-label">Suspicious processes</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.networkIssues > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.networkIssues}</span>
            <span class="scan-stat-label">High bandwidth (&gt;10 MB/s)</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.highMem > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.highMem}</span>
            <span class="scan-stat-label">High memory (&gt;2 GB)</span>
          </div>
          <div class="scan-stat" style="color: {quickScanResults.highCpu > 0 ? 'var(--yellow)' : 'var(--green)'}">
            <span class="scan-stat-value">{quickScanResults.highCpu}</span>
            <span class="scan-stat-label">High CPU (&gt;80%)</span>
          </div>
        </div>
      {/if}

      <!-- Risk Score Overview -->
      <div class="risk-overview">
        <div class="risk-gauge">
          <div class="risk-score" style="color: {riskColor(riskScore)}">{riskScore}</div>
          <div class="risk-label" style="color: {riskColor(riskScore)}">{riskLabel(riskScore)} Risk</div>
        </div>
        <div class="risk-breakdown">
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--danger)">{criticalCount}</span>
            <span class="risk-stat-label">Critical</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--danger)">{highCount}</span>
            <span class="risk-stat-label">High</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--yellow)">{mediumCount}</span>
            <span class="risk-stat-label">Medium</span>
          </div>
          <div class="risk-stat">
            <span class="risk-count" style="color: var(--fg-dim)">{lowCount}</span>
            <span class="risk-stat-label">Low</span>
          </div>
        </div>
        <div class="risk-summary">
          {#if findings.length === 0}
            No security issues detected. Your system appears healthy.
          {:else}
            Found {findings.length} security finding{findings.length !== 1 ? "s" : ""} across
            {$flaggedPids.size} process{$flaggedPids.size !== 1 ? "es" : ""}. Review the details below.
          {/if}
        </div>
      </div>

      <!-- Findings List -->
      {#if findings.length > 0}
        <div class="findings-section">
          <h3 class="section-title">Findings</h3>
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
                    <span class="detail-label">What happened</span>
                    <p class="detail-text">{finding.description}</p>
                  </div>
                  <div class="detail-section">
                    <span class="detail-label">What to do</span>
                    <p class="detail-text recommendation">{finding.recommendation}</p>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Scanned Processes -->
      <div class="meta-section">
        <span class="meta-label">Scanned</span>
        <span class="meta-value">{$processes.length} processes</span>
        <span class="meta-sep">·</span>
        <span class="meta-label">Flagged</span>
        <span class="meta-value">{$flaggedPids.size}</span>
        <span class="meta-sep">·</span>
        <span class="meta-label">Generated</span>
        <span class="meta-value">{new Date().toLocaleTimeString()}</span>
      </div>
    </div>
  </div>
</div>

<style>
  .report-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 300;
  }

  .report-modal {
    background: var(--bg-surface, var(--bg-alt));
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    width: 560px;
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0,0,0,0.6));
  }

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

  .report-close {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--radius-sm, 4px);
    background: transparent;
    color: var(--fg-dim);
    font-size: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .report-close:hover { background: var(--bg-hover); color: var(--fg); }

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

  .quick-scan-btn {
    border: 1px solid var(--accent);
    background: var(--accent);
    color: white;
    border-radius: 8px;
    padding: 8px 12px;
    font-weight: 700;
    cursor: pointer;
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
    background: var(--bg);
  }

  .scan-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
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

  /* Risk Overview */
  .risk-overview {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--bg);
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
  .finding-tag.mitre { background: rgba(239, 68, 68, 0.12); color: var(--danger); }
  .finding-tag.cve { background: rgba(234, 179, 8, 0.12); color: var(--yellow); }

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
    border-top: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
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
</style>
