<script lang="ts">
  import { stats, processes } from "../stores/processes";
  import { metricsHistory } from "../stores/metricsHistory";
  import { theme } from "../stores/preferences";
  import { t } from "../lib/i18n";

  interface Props {
    collapsed?: boolean;
    mode?: "basic" | "pro";
    onopenmetric?: (metric: "cpu" | "ram" | "network" | "swap" | "processes") => void;
  }

  let { collapsed = false, mode = "pro", onopenmetric }: Props = $props();
  let proMode = $derived(mode === "pro");

  let cpuCanvas: HTMLCanvasElement | undefined = $state();
  let ramCanvas: HTMLCanvasElement | undefined = $state();
  let netCanvas: HTMLCanvasElement | undefined = $state();

  const CHART_H = 48;
  const POINTS = 60; // show last 60 data points

  function getColor(varName: string): string {
    if (typeof document === "undefined") return "#3b82f6";
    return getComputedStyle(document.documentElement).getPropertyValue(varName).trim() || "#3b82f6";
  }

  function drawSparkline(
    canvas: HTMLCanvasElement,
    data: number[],
    colorVar: string,
    maxVal?: number,
    secondData?: number[],
    secondColorVar?: string,
  ): void {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const context = ctx;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    context.scale(dpr, dpr);

    // Clear
    context.clearRect(0, 0, w, h);

    // Grid lines
    const gridColor = getColor("--chart-grid") || "#141418";
    context.strokeStyle = gridColor;
    context.lineWidth = 1;
    for (let i = 1; i <= 3; i++) {
      const y = (h / 4) * i;
      context.beginPath();
      context.moveTo(0, y);
      context.lineTo(w, y);
      context.stroke();
    }

    function drawLine(values: number[], color: string, max: number) {
      if (values.length < 2) return;
      const step = w / (POINTS - 1);
      const startIdx = Math.max(0, values.length - POINTS);
      const slice = values.slice(startIdx);

      // Area fill
      context.beginPath();
      context.moveTo(0, h);
      for (let i = 0; i < slice.length; i++) {
        const x = i * step;
        const y = h - (slice[i] / max) * (h - 2);
        if (i === 0) context.lineTo(x, y);
        else context.lineTo(x, y);
      }
      context.lineTo((slice.length - 1) * step, h);
      context.closePath();
      // Create solid fill by darkening the color (mix ~90% with black)
      const rgbMatch = color.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
      if (rgbMatch) {
        const r = Math.round(parseInt(rgbMatch[1]) * 0.15);
        const g = Math.round(parseInt(rgbMatch[2]) * 0.15);
        const b = Math.round(parseInt(rgbMatch[3]) * 0.15);
        context.fillStyle = `rgb(${r}, ${g}, ${b})`;
      } else {
        context.fillStyle = color;
      }
      context.fill();

      // Line
      context.beginPath();
      for (let i = 0; i < slice.length; i++) {
        const x = i * step;
        const y = h - (slice[i] / max) * (h - 2);
        if (i === 0) context.moveTo(x, y);
        else context.lineTo(x, y);
      }
      context.strokeStyle = color;
      context.lineWidth = 1.5;
      context.lineJoin = "round";
      context.stroke();
    }

    const max = maxVal ?? Math.max(...data, 1);
    drawLine(data, getColor(colorVar), max);

    if (secondData && secondColorVar) {
      const max2 = maxVal ?? Math.max(...secondData, 1);
      drawLine(secondData, getColor(secondColorVar), max2);
    }
  }

  // Reactively redraw charts when history updates
  $effect(() => {
    const _t = $theme; // react to theme changes
    const h = $metricsHistory;
    if (collapsed || h.length < 2) return;

    if (cpuCanvas) {
      const avg = h[h.length - 1]?.cpuAvg || 0;
      drawSparkline(cpuCanvas, h.map((s) => s.cpuAvg), colorVarForPct(avg), 100);
    }
    if (ramCanvas) {
      const pct = h[h.length - 1]?.ramPct || 0;
      drawSparkline(ramCanvas, h.map((s) => s.ramPct), colorVarForPct(pct), 100);
    }
    if (netCanvas) {
      const rx = h.map((s) => s.netRx);
      const tx = h.map((s) => s.netTx);
      const maxNet = Math.max(...rx, ...tx, 1024);
      drawSparkline(netCanvas, rx, "--chart-net-rx", maxNet, tx, "--chart-net-tx");
    }
  });

  function handleOpenMetric(metric: "cpu" | "ram" | "network" | "swap" | "processes") {
    onopenmetric?.(metric);
  }

  let avgCpu = $derived(
    $processes.length > 0
      ? $processes.reduce((sum, p) => sum + p.cpu_pct, 0) / $processes.length
      : 0,
  );

  function formatRate(bytesPerSec: number): string {
    if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
    if (bytesPerSec < 1_048_576) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / 1_048_576).toFixed(1)} MB/s`;
  }

  function colorVarForPct(pct: number): string {
    if (pct >= 80) return "--danger";
    if (pct >= 60) return "--yellow";
    return "--green";
  }

  function colorForPct(pct: number): string {
    return `var(${colorVarForPct(pct)})`;
  }
</script>

{#if $stats && !collapsed}
  <div class="dashboard">
    <button class="metric-card metric-button" onclick={() => handleOpenMetric("cpu")}>
      <div class="metric-header">
        <span class="metric-label">CPU</span>
        <span class="metric-value" style="color: {colorForPct(avgCpu)}">{avgCpu.toFixed(1)}%</span>
      </div>
      <canvas bind:this={cpuCanvas} class="spark" height={CHART_H}></canvas>
    </button>

    <button class="metric-card metric-button" onclick={() => handleOpenMetric("ram")}>
      <div class="metric-header">
        <span class="metric-label">RAM</span>
        <span class="metric-value" style="color: {colorForPct($stats.ram_used_pct)}">
          {$stats.ram_used_pct}% <span class="metric-sub">/ {$stats.ram_total_gb.toFixed(0)}GB</span>
        </span>
      </div>
      <canvas bind:this={ramCanvas} class="spark" height={CHART_H}></canvas>
    </button>

    {#if proMode}
      <button class="metric-card metric-button" onclick={() => handleOpenMetric("network")}>
        <div class="metric-header">
          <span class="metric-label">Network</span>
          <span class="metric-value net-values">
            <span class="net-rx">{formatRate($stats.net_rx_bytes_per_sec)}</span>
            <span class="net-tx">{formatRate($stats.net_tx_bytes_per_sec)}</span>
          </span>
        </div>
        <canvas bind:this={netCanvas} class="spark" height={CHART_H}></canvas>
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
