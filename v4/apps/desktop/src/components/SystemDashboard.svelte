<script lang="ts">
  import { stats, processes } from "../stores/processes";
  import { cpuSeries, ramSeries, netRxSeries, netTxSeries } from "../stores/metricsHistory";
  import { t } from "../lib/i18n";
  import TvChart from "./TvChart.svelte";

  interface Props {
    collapsed?: boolean;
    mode?: "basic" | "pro";
    layout?: "compact" | "standard" | "expanded";
    onopenmetric?: (metric: "cpu" | "ram" | "network" | "swap" | "processes") => void;
  }

  let { collapsed = false, mode = "pro", layout = "standard", onopenmetric }: Props = $props();
  let proMode = $derived(mode === "pro");
  let showSparklines = $derived(layout !== "compact");

  function handleOpenMetric(metric: "cpu" | "ram" | "network" | "swap" | "processes") {
    onopenmetric?.(metric);
  }

  let cpuPct = $derived($stats?.cpu_usage_pct ?? 0);

  import { formatNetworkRate } from "../lib/formatting";
  const formatRate = formatNetworkRate;

  function colorVarForPct(pct: number): string {
    if (pct >= 80) return "--danger";
    if (pct >= 60) return "--yellow";
    return "--green";
  }

  function colorForPct(pct: number): string {
    return `var(${colorVarForPct(pct)})`;
  }

  const cpuChartSeries = $derived([{ data: $cpuSeries, color: colorVarForPct(cpuPct) }]);
  const ramChartSeries = $derived([{ data: $ramSeries, color: colorVarForPct($stats?.ram_used_pct ?? 0) }]);
  const netChartSeries = $derived([
    { data: $netRxSeries, color: "--chart-net-rx" },
    { data: $netTxSeries, color: "--chart-net-tx" },
  ]);
</script>

{#if $stats && !collapsed}
  <div class="dashboard">
    <button class="metric-card metric-button" onclick={() => handleOpenMetric("cpu")}>
      <div class="metric-header">
        <span class="metric-label">{t("status.cpu")}</span>
        <span class="metric-value" style="color: {colorForPct(cpuPct)}">{cpuPct.toFixed(1)}%</span>
      </div>
      {#if showSparklines}
        <div class="spark" class:spark-expanded={layout === "expanded"}>
          {#if $cpuSeries.length > 1}
            <TvChart series={cpuChartSeries} sparkline />
          {/if}
        </div>
      {/if}
    </button>

    <button class="metric-card metric-button" onclick={() => handleOpenMetric("ram")}>
      <div class="metric-header">
        <span class="metric-label">{t("status.ram")}</span>
        <span class="metric-value" style="color: {colorForPct($stats.ram_used_pct)}">
          {$stats.ram_used_pct}% <span class="metric-sub">/ {$stats.ram_total_gb.toFixed(0)} GB</span>
        </span>
      </div>
      {#if showSparklines}
        <div class="spark" class:spark-expanded={layout === "expanded"}>
          {#if $ramSeries.length > 1}
            <TvChart series={ramChartSeries} sparkline />
          {/if}
        </div>
      {/if}
    </button>

    {#if proMode}
      <button class="metric-card metric-button" onclick={() => handleOpenMetric("network")}>
        <div class="metric-header">
          <span class="metric-label">{t("status.net")}</span>
          <span class="metric-value net-values">
            <span class="net-rx">{formatRate($stats.net_rx_bytes_per_sec)}</span>
            <span class="net-tx">{formatRate($stats.net_tx_bytes_per_sec)}</span>
          </span>
        </div>
        {#if showSparklines}
          <div class="spark" class:spark-expanded={layout === "expanded"}>
            {#if $netRxSeries.length > 1}
              <TvChart series={netChartSeries} sparkline />
            {/if}
          </div>
        {/if}
      </button>
    {/if}

    <div class="metric-card metric-stats">
      {#if proMode}
        <button class="stat-row stat-button" onclick={() => handleOpenMetric("swap")}>
          <span class="stat-label">{t("status.swap")}</span>
          <span class="stat-value">{$stats.swap_used_mb} MB</span>
        </button>
      {/if}
      <button class="stat-row stat-button" onclick={() => handleOpenMetric("processes")}>
        <span class="stat-label">{t("status.procs")}</span>
        <span class="stat-value">{$stats.total_processes}</span>
      </button>
      {#if proMode}
        <button class="stat-row stat-button" onclick={() => handleOpenMetric("cpu")}>
          <span class="stat-label">{t("status.idle")}</span>
          <span class="stat-value">{$processes.filter((p) => p.idle).length}</span>
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .dashboard {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .metric-card {
    background: var(--bg-card, var(--bg-secondary));
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .metric-button,
  .stat-button {
    cursor: pointer;
  }

  .metric-button {
    text-align: left;
    border: 1px solid var(--border);
  }

  .metric-button:hover,
  .stat-button:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .metric-button:focus-visible,
  .stat-button:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }

  .metric-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
  }

  .metric-label {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .metric-value {
    font-size: calc(var(--base-font-size, 12px) * 1.083);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .metric-sub {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 400;
    color: var(--text-secondary);
  }

  .net-values {
    display: flex;
    gap: 8px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .net-rx { color: var(--chart-net-rx, var(--success)); }
  .net-tx { color: var(--chart-net-tx, var(--warning)); }

  .net-rx::before { content: "\2193 "; }
  .net-tx::before { content: "\2191 "; }

  .spark {
    width: 100%;
    height: 48px;
    border-radius: 4px;
  }

  .spark-expanded {
    height: 72px;
  }

  .metric-stats {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    min-width: 100px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .stat-button {
    border: none;
    background: transparent;
    padding: 0;
    width: 100%;
  }

  .stat-label {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-secondary);
  }

  .stat-value {
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--text-primary);
  }

  @media (max-width: 600px) {
    .dashboard {
      grid-template-columns: 1fr 1fr;
    }
  }

  @media (max-width: 360px) {
    .dashboard {
      grid-template-columns: 1fr;
    }
  }
</style>
