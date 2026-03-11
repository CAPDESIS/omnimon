<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";
  import { onMount } from "svelte";
  import { cpuSeries, ramSeries, swapSeries, metricsHistory } from "../stores/metricsHistory";
  import { filtered, stats } from "../stores/processes";
  import type { UserMode } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import { askAiRequest } from "../stores/uiActions";
  import type { ProcessEntry } from "../lib/types";
  import { focusFirstFocusable, trapFocus } from "../lib/focusTrap";
  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";
  import ModalShell from "./ModalShell.svelte";
  import {
    activeSeriesForMetric,
    defaultSortKey,
    getSparklineColor,
    loadNetworkMap,
    metricSummaryLabel,
    sparklinePath,
  } from "../lib/systemMetricModal";

  type MetricKind = import("../lib/systemMetricModal").MetricKind;
  type SortKey = import("../lib/systemMetricModal").SortKey;

  interface Props {
    metric: MetricKind;
    mode?: UserMode;
    onclose: () => void;
  }

  let { metric, mode = "pro", onclose }: Props = $props();
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
          askAiRequest.set(`¿Qué está pasando con ${metric}?`);
          onclose();
        }}>
          ✨ ¿Qué está pasando con {metric}?
        </Button>
        <IconButton class="close-btn" onclick={onclose} ariaLabel={t("common.close")} title={t("common.close")}>×</IconButton>
      </div>
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
          {#if networkMapPromise}
            {#await networkMapPromise then NetworkMapModule}
              <NetworkMapModule.default mode={mode} />
            {:catch}
              <div class="network-map-error">Failed to load network map.</div>
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
          <div class="summary-card"><span class="card-label">History</span><span class="card-value">{$metricsHistory.length} samples</span></div>
          <div class="summary-card"><span class="card-label">Showing</span><span class="card-value">{topProcesses.length} / {$filtered.length}</span></div>
        </div>

        <!-- Sparkline chart -->
        {#if activeSeries.length > 1}
          <div class="sparkline-container">
            <svg viewBox="0 0 200 32" preserveAspectRatio="none" class="sparkline-svg" role="img" aria-label={summaryLabel}>
              <path d={sparklinePath(activeSeries)} fill="none" stroke={getSparklineColor(metric, activeSeries)} stroke-width="1.5" />
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
                  <th class="th-name" scope="col"><button type="button" class="sort-button" onclick={() => toggleSort("name")}>Name<span aria-hidden="true">{sortIndicator("name")}</span></button></th>
                  <th class="th-pid" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("pid")}>PID<span aria-hidden="true">{sortIndicator("pid")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("cpu")}>CPU%<span aria-hidden="true">{sortIndicator("cpu")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("ram")}>RAM MB<span aria-hidden="true">{sortIndicator("ram")}</span></button></th>
                  <th class="th-num" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("net")}>Net<span aria-hidden="true">{sortIndicator("net")}</span></button></th>
                  <th class="th-state" scope="col"><button type="button" class="sort-button" onclick={() => toggleSort("state")}>State<span aria-hidden="true">{sortIndicator("state")}</span></button></th>
                  <th class="th-uptime" scope="col"><button type="button" class="sort-button sort-button-num" onclick={() => toggleSort("uptime")}>Uptime<span aria-hidden="true">{sortIndicator("uptime")}</span></button></th>
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
            <Button class="show-more-btn" variant="secondary" size="sm" onclick={() => processLimit += 20}>
              Show more ({$filtered.length - processLimit} remaining)
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

  .sparkline-container {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    background: var(--bg-alt);
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
