<script lang="ts">
  import { cpuSeries, ramSeries, netRxSeries, netTxSeries, swapSeries, metricsHistory } from "../stores/metricsHistory";
  import { filtered, stats } from "../stores/processes";
  import NetworkMap from "./NetworkMap.svelte";
  import { t } from "../lib/i18n";
  import type { MetricPoint } from "../stores/metricsHistory";
  import type { ProcessEntry } from "../lib/types";

  type MetricKind = "cpu" | "ram" | "network" | "swap" | "processes";
  type SortKey = "name" | "pid" | "cpu" | "ram" | "net" | "state" | "uptime";

  interface Props {
    metric: MetricKind;
    onclose: () => void;
  }

  let { metric, onclose }: Props = $props();
  function defaultSortKey(kind: MetricKind): SortKey {
    if (kind === "cpu") return "cpu";
    if (kind === "ram" || kind === "swap") return "ram";
    if (kind === "network") return "net";
    return "ram";
  }
  let sortKey = $state<SortKey>("ram");
  let sortAsc = $state(false);
  let processLimit = $state(30);

  $effect(() => {
    sortKey = defaultSortKey(metric);
    sortAsc = false;
    processLimit = 30;
  });

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
    if (series.length === 0) return "—";
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

  function toggleSort(key: SortKey) {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = false;
    }
  }

  function sortIndicator(key: SortKey): string {
    if (sortKey !== key) return "";
    return sortAsc ? " ▲" : " ▼";
  }

  function getSortValue(proc: ProcessEntry, key: SortKey): number | string {
    switch (key) {
      case "name": return proc.name.toLowerCase();
      case "pid": return proc.pid;
      case "cpu": return proc.cpu_pct;
      case "ram": return proc.ram_mb;
      case "net": return proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec;
      case "state": return proc.state ?? "";
      case "uptime": return proc.uptime ?? "";
    }
  }

  const topProcesses = $derived.by(() => {
    const list = [...$filtered];
    list.sort((a, b) => {
      const va = getSortValue(a, sortKey);
      const vb = getSortValue(b, sortKey);
      const cmp = typeof va === "string" ? va.localeCompare(vb as string) : (va as number) - (vb as number);
      return sortAsc ? cmp : -cmp;
    });
    return list.slice(0, processLimit);
  });

  /** Build a mini sparkline SVG path from a series of points */
  function sparklinePath(series: MetricPoint[], width = 200, height = 32): string {
    if (series.length < 2) return "";
    const max = Math.max(...series.map((p) => p.value), 1);
    const step = width / (series.length - 1);
    return series.map((p, i) => {
      const x = i * step;
      const y = height - (p.value / max) * height;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
  }

  const activeSeries = $derived.by(() => {
    switch (metric) {
      case "cpu": return $cpuSeries;
      case "ram": return $ramSeries;
      case "swap": return $swapSeries;
      default: return [];
    }
  });
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
            <div class="summary-card"><span class="card-label">RX</span><span class="card-value">{formatRate($stats?.net_rx_bytes_per_sec ?? 0)}</span></div>
            <div class="summary-card"><span class="card-label">TX</span><span class="card-value">{formatRate($stats?.net_tx_bytes_per_sec ?? 0)}</span></div>
            <div class="summary-card"><span class="card-label">Samples</span><span class="card-value">{$metricsHistory.length}</span></div>
            <div class="summary-card"><span class="card-label">Processes</span><span class="card-value">{$stats?.total_processes ?? 0}</span></div>
          </div>
          <NetworkMap />
        </div>
      {:else}
        <!-- Summary cards -->
        <div class="summary-row">
          <div class="summary-card wide">
            <span class="card-label">{metricTitle(metric)}</span>
            <span class="card-value">
              {metric === "cpu" ? formatSeries($cpuSeries, "%") : metric === "ram" ? formatSeries($ramSeries, "%") : metric === "swap" ? formatSeries($swapSeries, " MB") : `${$stats?.total_processes ?? 0} visible`}
            </span>
          </div>
          <div class="summary-card"><span class="card-label">History</span><span class="card-value">{$metricsHistory.length} samples</span></div>
          <div class="summary-card"><span class="card-label">Showing</span><span class="card-value">{topProcesses.length} / {$filtered.length}</span></div>
        </div>

        <!-- Sparkline chart -->
        {#if activeSeries.length > 1}
          <div class="sparkline-container">
            <svg viewBox="0 0 200 32" preserveAspectRatio="none" class="sparkline-svg">
              <path d={sparklinePath(activeSeries)} fill="none" stroke="var(--accent)" stroke-width="1.5" />
            </svg>
          </div>
        {/if}

        <!-- Interactive process table -->
        <div class="section">
          <div class="section-title">Top processes · Click headers to sort</div>
          <div class="process-table-wrapper">
            <table class="process-table">
              <thead>
                <tr>
                  <th class="th-name" onclick={() => toggleSort("name")}>Name{sortIndicator("name")}</th>
                  <th class="th-pid" onclick={() => toggleSort("pid")}>PID{sortIndicator("pid")}</th>
                  <th class="th-num" onclick={() => toggleSort("cpu")}>CPU%{sortIndicator("cpu")}</th>
                  <th class="th-num" onclick={() => toggleSort("ram")}>RAM MB{sortIndicator("ram")}</th>
                  <th class="th-num" onclick={() => toggleSort("net")}>Net{sortIndicator("net")}</th>
                  <th class="th-state" onclick={() => toggleSort("state")}>State{sortIndicator("state")}</th>
                  <th class="th-uptime" onclick={() => toggleSort("uptime")}>Uptime{sortIndicator("uptime")}</th>
                </tr>
              </thead>
              <tbody>
                {#each topProcesses as proc (proc.pid)}
                  <tr>
                    <td class="td-name" title={proc.exec_name}>{proc.name}</td>
                    <td class="td-mono">{proc.pid}</td>
                    <td class="td-mono">{proc.cpu_pct.toFixed(1)}</td>
                    <td class="td-mono">{proc.ram_mb.toFixed(1)}</td>
                    <td class="td-mono">{formatRate(proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec)}</td>
                    <td class="td-state">{proc.state ?? "—"}</td>
                    <td class="td-mono">{proc.uptime ?? "—"}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          {#if $filtered.length > processLimit}
            <button class="show-more-btn" onclick={() => processLimit += 20}>
              Show more ({$filtered.length - processLimit} remaining)
            </button>
          {/if}
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
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 10px;
  }

  .summary-card {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
    background: rgba(255,255,255,0.02);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .summary-card.wide {
    grid-column: span 2;
  }

  .card-label {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
  }

  .card-value {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
  }

  .sparkline-container {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    background: rgba(255,255,255,0.02);
    height: 48px;
  }

  .sparkline-svg {
    width: 100%;
    height: 100%;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title {
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    color: var(--fg-dim);
  }

  .process-table-wrapper {
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .process-table {
    width: 100%;
    border-collapse: collapse;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
  }

  .process-table thead {
    position: sticky;
    top: 0;
    background: var(--bg-alt);
    z-index: 1;
  }

  .process-table th {
    padding: 8px 10px;
    text-align: left;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--fg-dim);
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .process-table th:hover {
    color: var(--accent);
  }

  .th-num, .th-pid { text-align: right; }
  .th-uptime { text-align: right; }

  .process-table td {
    padding: 6px 10px;
    border-bottom: 1px solid rgba(128,128,128,0.08);
  }

  .process-table tbody tr:hover {
    background: var(--bg-hover);
  }

  .td-name {
    font-weight: 600;
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .td-mono {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    text-align: right;
    color: var(--fg-dim);
    white-space: nowrap;
  }

  .td-state {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
  }

  .show-more-btn {
    align-self: center;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--accent);
    padding: 6px 16px;
    cursor: pointer;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .show-more-btn:hover {
    background: var(--bg-hover);
  }

  @media (max-width: 800px) {
    .summary-row {
      grid-template-columns: 1fr;
    }

    .summary-card.wide {
      grid-column: span 1;
    }
  }
</style>
