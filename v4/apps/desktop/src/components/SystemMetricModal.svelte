<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";
  import { onMount } from "svelte";
  import { cpuSeries, ramSeries, swapSeries, netRxSeries, netTxSeries, metricsHistory } from "../stores/metricsHistory";
  import { filtered, stats } from "../stores/processes";
  import type { UserMode } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import { formatProcessState, formatProcessUptime } from "../lib/localizedUi";
  import { askAiRequest } from "../stores/uiActions";
  import type { ProcessEntry } from "../lib/types";
  import { focusFirstFocusable, trapFocus } from "../lib/focusTrap";
  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";
  import ModalShell from "./ModalShell.svelte";
  import TvChart from "./TvChart.svelte";
  import {
    activeSeriesForMetric,
    defaultSortKey,
    loadNetworkMap,
    metricSummaryLabel,
  } from "../lib/systemMetricModal";

  type MetricKind = import("../lib/systemMetricModal").MetricKind;
  type SortKey = import("../lib/systemMetricModal").SortKey;

  interface Props {
    metric: MetricKind;
    mode?: UserMode;
    onclose: () => void;
    oninspect?: (process: ProcessEntry) => void;
  }

  let { metric, mode = "pro", onclose, oninspect }: Props = $props();
  let modalEl: HTMLDivElement | undefined = $state();
  let sortKey = $state<SortKey>("ram");
  let sortAsc = $state(false);
  let processLimit = $state(30);
  let networkMapPromise = $state<Promise<any> | null>(null);

  function closeWhenBackdropMatches(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }

  function stopMouseEventPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  $effect(() => {
    sortKey = defaultSortKey(metric);
    sortAsc = false;
    processLimit = 30;
  });

  $effect(() => {
    if (metric === "network") {
      networkMapPromise ??= loadNetworkMap();
    }
  });

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onclose();
      return;
    }
    trapFocus(event, modalEl);
  }

  onMount(() => {
    requestAnimationFrame(() => focusFirstFocusable(modalEl));
  });

  function metricTitle(kind: MetricKind): string {
    switch (kind) {
      case "cpu": return t("status.cpu");
      case "ram": return t("status.ram");
      case "network": return t("status.net");
      case "swap": return t("status.swap");
      case "processes": return t("status.procs");
    }
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
    if (metric === "network") return [];
    const list = [...$filtered];
    list.sort((a, b) => {
      const va = getSortValue(a, sortKey);
      const vb = getSortValue(b, sortKey);
      const cmp = typeof va === "string" ? va.localeCompare(vb as string) : (va as number) - (vb as number);
      return sortAsc ? cmp : -cmp;
    });
    return list.slice(0, processLimit);
  });

  const activeSeries = $derived.by(() => {
    return activeSeriesForMetric(metric, {
      cpuSeries: $cpuSeries,
      ramSeries: $ramSeries,
      swapSeries: $swapSeries,
    });
  });

  /** Dynamic color based on current usage: green (good) → yellow (warning) → red (danger) */
  function colorVarForPct(pct: number): string {
    if (pct >= 80) return "--danger";
    if (pct >= 60) return "--yellow";
    return "--green";
  }

  const currentCpuPct = $derived($stats?.cpu_usage_pct ?? 0);

  const chartColorVar = $derived.by((): string => {
    switch (metric) {
      case "cpu": return colorVarForPct(currentCpuPct);
      case "ram": return colorVarForPct($stats?.ram_used_pct ?? 0);
      case "swap": {
        const mb = $swapSeries.length > 0 ? $swapSeries[$swapSeries.length - 1].value : 0;
        if (mb >= 4096) return "--danger";
        if (mb >= 1024) return "--yellow";
        return "--green";
      }
      case "processes": return colorVarForPct(currentCpuPct);
      default: return "--accent";
    }
  });

  function chartPriceFormat(kind: MetricKind): "percent" | "decimal" | "bytes" | "megabytes" {
    if (kind === "cpu" || kind === "ram") return "percent";
    if (kind === "swap") return "megabytes";
    return "decimal";
  }

  const chartSeries = $derived.by(() => {
    if (metric === "network") {
      return [
        { data: $netRxSeries, color: "--chart-net-rx", label: "RX" },
        { data: $netTxSeries, color: "--chart-net-tx", label: "TX" },
      ];
    }
    if (activeSeries.length < 2) return [];
    return [{ data: activeSeries, color: chartColorVar, label: metricTitle(metric) }];
  });

  /** Dynamic Y-axis scaling — adapts to actual data so the chart always shows visible flow */
  const chartMaxY = $derived.by((): number | undefined => {
    if (metric === "cpu") {
      if ($cpuSeries.length === 0) return 100;
      let max = 0;
      for (const p of $cpuSeries) if (p.value > max) max = p.value;
      // Round up to nearest 10%, minimum 20%, capped at 100%
      return Math.min(100, Math.max(20, Math.ceil(max * 1.3 / 10) * 10));
    }
    if (metric === "ram") {
      if ($ramSeries.length === 0) return 100;
      let max = 0;
      for (const p of $ramSeries) if (p.value > max) max = p.value;
      return Math.min(100, Math.max(30, Math.ceil(max * 1.2 / 10) * 10));
    }
    if (metric === "network") {
      // Use P95 to avoid spike domination — normal traffic fills the chart
      const allValues: number[] = [];
      for (const p of $netRxSeries) allValues.push(p.value);
      for (const p of $netTxSeries) allValues.push(p.value);
      if (allValues.length === 0) return undefined;
      allValues.sort((a, b) => a - b);
      const p95 = allValues[Math.floor(allValues.length * 0.95)] ?? 0;
      if (p95 <= 0) return undefined;
      return Math.ceil(p95 * 2);
    }
    return undefined; // swap, processes: auto-scale
  });

  const summaryLabel = $derived.by(() =>
    metricSummaryLabel(metric, {
      cpuSeries: $cpuSeries,
      ramSeries: $ramSeries,
      swapSeries: $swapSeries,
      totalProcesses: $stats?.total_processes,
    }),
  );
</script>

<div transition:fade={fadeConfig}>
  <ModalShell titleId="metric-modal-title" backdropClass="backdrop" panelClass="modal" onclose={onclose}>
  <div bind:this={modalEl} onkeydown={closeOnEscape} transition:scale={scaleConfig}>
    <div class="header">
      <div>
        <div class="eyebrow">{t("status.deepDive")}</div>
        <h2 id="metric-modal-title">{metricTitle(metric)}</h2>
      </div>
      <div class="header-actions">
        <Button class="ask-ai-btn" variant="secondary" size="sm" onclick={() => {
          askAiRequest.set(t("systemMetrics.askAi", { metric: metricTitle(metric) }));
          onclose();
        }}>
          ✨ {t("systemMetrics.askAi", { metric: metricTitle(metric) })}
        </Button>
        <IconButton class="close-btn" onclick={onclose} ariaLabel={t("common.close")} title={t("common.close")}>×</IconButton>
      </div>
    </div>

    <div class="body">
      {#if metric === "network"}
        <div class="section">
          <div class="summary-row">
            <div class="summary-card"><span class="card-label">{t("systemMetrics.rx")}</span><span class="card-value">{formatRate($stats?.net_rx_bytes_per_sec ?? 0)}</span></div>
            <div class="summary-card"><span class="card-label">{t("systemMetrics.tx")}</span><span class="card-value">{formatRate($stats?.net_tx_bytes_per_sec ?? 0)}</span></div>
            <div class="summary-card"><span class="card-label">{t("systemMetrics.samples")}</span><span class="card-value">{$metricsHistory.length}</span></div>
            <div class="summary-card"><span class="card-label">{t("systemMetrics.processes")}</span><span class="card-value">{$stats?.total_processes ?? 0}</span></div>
          </div>
          {#if chartSeries.length > 0}
            <div class="chart-container">
              <TvChart
                series={chartSeries}
                maxY={chartMaxY}
                height="160px"
                priceFormat="bytes"
              />
            </div>
          {/if}
          {#if networkMapPromise}
            {#await networkMapPromise then NetworkMapModule}
              <NetworkMapModule.default mode={mode} />
            {:catch}
              <div class="network-map-error">{t("systemMetrics.networkMapLoadError")}</div>
            {/await}
          {/if}
        </div>
      {:else}
        <!-- Summary cards -->
        <div class="summary-row">
          <div class="summary-card wide">
              <span class="card-label">{metricTitle(metric)}</span>
              <span class="card-value">
                {summaryLabel}
              </span>
            </div>
          <div class="summary-card"><span class="card-label">{t("systemMetrics.history")}</span><span class="card-value">{t("systemMetrics.samplesCount", { count: $metricsHistory.length })}</span></div>
          <div class="summary-card"><span class="card-label">{t("systemMetrics.showing")}</span><span class="card-value">{t("systemMetrics.visibleCount", { shown: topProcesses.length, total: $filtered.length })}</span></div>
        </div>

        <!-- TradingView chart -->
        {#if chartSeries.length > 0}
          <div class="chart-container">
            <TvChart
              series={chartSeries}
              maxY={chartMaxY}
              height="180px"
              priceFormat={chartPriceFormat(metric)}
            />
          </div>
        {/if}

        <!-- Interactive process table -->
        <div class="section">
          <div class="section-title">{t("systemMetrics.topProcesses")} · {t("systemMetrics.sortHint")}</div>
          <div class="process-table-wrapper">
            <table class="process-table">
              <thead>
                <tr>
                  <th class="th-name" scope="col"><button type="button" class="sort-button" onclick={() => toggleSort("name")}>{t("table.name")}<span aria-hidden="true">{sortIndicator("name")}</span></button></th>
                  <th class="th-pid" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("pid")}>{t("table.pid")}<span aria-hidden="true">{sortIndicator("pid")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("cpu")}>{t("table.cpu")}<span aria-hidden="true">{sortIndicator("cpu")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("ram")}>{t("table.ram")} MB<span aria-hidden="true">{sortIndicator("ram")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("net")}>{t("table.network")}<span aria-hidden="true">{sortIndicator("net")}</span></button></th>
                  <th class="th-state" scope="col"><button type="button" class="sort-button" onclick={() => toggleSort("state")}>{t("process.state")}<span aria-hidden="true">{sortIndicator("state")}</span></button></th>
                  <th class="th-uptime" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("uptime")}>{t("process.uptime")}<span aria-hidden="true">{sortIndicator("uptime")}</span></button></th>
                </tr>
              </thead>
              <tbody>
                {#each topProcesses as proc (proc.pid)}
                  <tr class="process-row" class:clickable={!!oninspect} onclick={() => oninspect?.(proc)} title={oninspect ? t("systemMetrics.inspectProcess", { name: proc.name }) : undefined}>
                    <td class="td-name" title={proc.exec_name}>{proc.name}</td>
                    <td class="td-mono">{proc.pid}</td>
                    <td class="td-mono">{proc.cpu_pct.toFixed(1)}</td>
                    <td class="td-mono">{proc.ram_mb.toFixed(1)}</td>
                    <td class="td-mono">{formatRate(proc.net_rx_bytes_per_sec + proc.net_tx_bytes_per_sec)}</td>
                    <td class="td-state">{formatProcessState(proc.state)}</td>
                    <td class="td-mono">{formatProcessUptime(proc.uptime)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          {#if $filtered.length > processLimit}
            <Button class="show-more-btn" variant="secondary" size="sm" onclick={() => processLimit += 20}>
              {t("systemMetrics.showMore", { count: $filtered.length - processLimit })}
            </Button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
  </ModalShell>
</div>

<style>
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

  
  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .ask-ai-btn {
    font-size: calc(var(--base-font-size, 12px) * 0.9);
  }

  .close-btn {
    font-size: 18px;
  }

  .body {
    padding: 16px 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
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
    background: var(--bg-alt);
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

  .chart-container {
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--chart-bg, var(--bg-primary));
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    flex: 1;
    min-height: 0;
  }

  .section-title {
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    color: var(--fg-dim);
  }

  .process-table-wrapper {
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    flex: 1;
    min-height: 0;
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
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--fg-dim);
    border-bottom: 1px solid var(--border);
    user-select: none;
    white-space: nowrap;
  }

  .sort-button {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }

  .sort-button-num {
    text-align: right;
  }

  .sort-button:hover,
  .sort-button:focus-visible {
    color: var(--accent);
  }

  .th-num, .th-pid { text-align: right; }
  .th-uptime { text-align: right; }

  .process-table td {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
  }

  .process-table tbody tr:hover {
    background: var(--bg-hover);
  }

  .process-row.clickable {
    cursor: pointer;
  }

  .process-row.clickable:hover {
    background: var(--bg-selected, var(--bg-hover));
  }

  .process-row.clickable:active {
    background: color-mix(in srgb, var(--accent) 15%, var(--bg-secondary));
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
