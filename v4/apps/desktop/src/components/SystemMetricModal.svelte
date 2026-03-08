<script lang="ts">
  import { cpuSeries, ramSeries, netRxSeries, netTxSeries, swapSeries, metricsHistory } from "../stores/metricsHistory";
  import { filtered, stats } from "../stores/processes";
  import NetworkMap from "./NetworkMap.svelte";
  import { t } from "../lib/i18n";
  import type { MetricPoint } from "../stores/metricsHistory";
  import type { ProcessEntry } from "../lib/types";

  type MetricKind = "cpu" | "ram" | "network" | "swap" | "processes";

  interface Props {
    metric: MetricKind;
    onclose: () => void;
  }

  let { metric, onclose }: Props = $props();

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") onclose();
  }

  function metricTitle(kind: MetricKind): string {
    switch (kind) {
      case "cpu": return t("status.cpu");
      case "ram": return t("status.ram");
      case "network": return t("status.net");
      case "swap": return t("status.swap");
      case "processes": return t("status.procs");
    }
  }

  function formatSeries(series: MetricPoint[], suffix = ""): string {
    if (series.length === 0) return "No recent samples";
    const last = series[series.length - 1]?.value ?? 0;
    const max = Math.max(...series.map((point) => point.value), 0);
    const avg = series.reduce((sum, point) => sum + point.value, 0) / Math.max(series.length, 1);
    return `Now ${last.toFixed(1)}${suffix} · Avg ${avg.toFixed(1)}${suffix} · Max ${max.toFixed(1)}${suffix}`;
  }

  function formatRate(bytesPerSec: number): string {
    if (bytesPerSec >= 1024 * 1024) return `${(bytesPerSec / (1024 * 1024)).toFixed(2)} MB/s`;
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${bytesPerSec.toFixed(0)} B/s`;
  }

  function sortProcesses(kind: MetricKind): ProcessEntry[] {
    const list = [...$filtered];
    switch (kind) {
      case "cpu":
        return list.sort((a, b) => b.cpu_pct - a.cpu_pct).slice(0, 20);
      case "ram":
        return list.sort((a, b) => b.ram_mb - a.ram_mb).slice(0, 20);
      case "network":
        return list.sort((a, b) => (b.net_rx_bytes_per_sec + b.net_tx_bytes_per_sec) - (a.net_rx_bytes_per_sec + a.net_tx_bytes_per_sec)).slice(0, 20);
      default:
        return list.sort((a, b) => b.ram_mb - a.ram_mb).slice(0, 20);
    }
  }

  const topProcesses = $derived(sortProcesses(metric));
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="backdrop" onclick={onclose} onkeydown={closeOnEscape} role="presentation">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="metric-modal-title" tabindex="-1">
    <div class="header">
      <div>
        <div class="eyebrow">{t("status.deepDive")}</div>
        <h2 id="metric-modal-title">{metricTitle(metric)}</h2>
      </div>
      <button class="close-btn" onclick={onclose} aria-label={t("common.close")}>×</button>
    </div>

    <div class="body">
      {#if metric === "network"}
        <div class="section">
          <div class="summary-row">
            <div class="summary-card">RX {formatRate($stats?.net_rx_bytes_per_sec ?? 0)}</div>
            <div class="summary-card">TX {formatRate($stats?.net_tx_bytes_per_sec ?? 0)}</div>
            <div class="summary-card">Samples {$metricsHistory.length}</div>
          </div>
          <div class="chart-summary">RX {$netRxSeries.length} pts · TX {$netTxSeries.length} pts</div>
          <NetworkMap />
        </div>
      {:else}
        <div class="summary-row">
          <div class="summary-card">{metric === "cpu" ? formatSeries($cpuSeries, "%") : metric === "ram" ? formatSeries($ramSeries, "%") : metric === "swap" ? formatSeries($swapSeries, " MB") : `${$stats?.total_processes ?? 0} visible`}</div>
          <div class="summary-card">History {$metricsHistory.length}</div>
          <div class="summary-card">Top rows {topProcesses.length}</div>
        </div>

        <div class="section">
          <div class="section-title">Top processes</div>
          <div class="process-grid">
            {#each topProcesses as proc}
              <div class="process-row">
                <div class="proc-name">{proc.name}</div>
                <div class="proc-meta">PID {proc.pid}</div>
                {#if metric === "cpu"}
                  <div class="proc-value">{proc.cpu_pct.toFixed(1)}%</div>
                {:else if metric === "ram"}
                  <div class="proc-value">{proc.ram_mb.toFixed(1)} MB</div>
                {:else if metric === "swap"}
                  <div class="proc-value">{proc.ram_mb.toFixed(1)} MB</div>
                {:else}
                  <div class="proc-value">{formatRate(proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec)}</div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 230;
    background: rgba(0, 0, 0, 0.62);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    width: min(1120px, calc(100vw - 28px));
    max-height: calc(100vh - 36px);
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--bg-surface, var(--bg-alt));
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 18px 18px 10px;
    border-bottom: 1px solid var(--border);
  }

  .eyebrow {
    color: var(--accent);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .close-btn {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--fg-dim);
    cursor: pointer;
    font-size: 18px;
  }

  .body {
    padding: 16px 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .summary-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .summary-card {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
    background: rgba(255,255,255,0.02);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title,
  .chart-summary {
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    color: var(--fg-dim);
  }

  .process-grid {
    display: grid;
    gap: 6px;
  }

  .process-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 10px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    background: rgba(255,255,255,0.02);
  }

  .proc-name {
    font-weight: 600;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .proc-meta,
  .proc-value {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--fg-dim);
  }

  @media (max-width: 800px) {
    .summary-row {
      grid-template-columns: 1fr;
    }

    .process-row {
      grid-template-columns: 1fr;
    }
  }
</style>
