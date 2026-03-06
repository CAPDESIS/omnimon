<script lang="ts">
  import { onMount } from "svelte";
  import { stats, processes } from "../stores/processes";
  import { metricsHistory, type MetricsSnapshot } from "../stores/metricsHistory";
  import { t } from "../lib/i18n";

  interface Props {
    collapsed?: boolean;
  }

  let { collapsed = false }: Props = $props();

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

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);

    // Clear
    ctx.clearRect(0, 0, w, h);

    // Grid lines
    const gridColor = getColor("--chart-grid") || "rgba(255,255,255,0.04)";
    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;
    for (let i = 1; i <= 3; i++) {
      const y = (h / 4) * i;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    function drawLine(values: number[], color: string, max: number) {
      if (values.length < 2) return;
      const step = w / (POINTS - 1);
      const startIdx = Math.max(0, values.length - POINTS);
      const slice = values.slice(startIdx);

      // Area fill
      ctx.beginPath();
      ctx.moveTo(0, h);
      for (let i = 0; i < slice.length; i++) {
        const x = i * step;
        const y = h - (slice[i] / max) * (h - 2);
        if (i === 0) ctx.lineTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.lineTo((slice.length - 1) * step, h);
      ctx.closePath();
      ctx.fillStyle = color.replace(")", ",0.1)").replace("rgb", "rgba");
      ctx.fill();

      // Line
      ctx.beginPath();
      for (let i = 0; i < slice.length; i++) {
        const x = i * step;
        const y = h - (slice[i] / max) * (h - 2);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.5;
      ctx.lineJoin = "round";
      ctx.stroke();
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
    const h = $metricsHistory;
    if (collapsed || h.length < 2) return;

    if (cpuCanvas) {
      drawSparkline(cpuCanvas, h.map((s) => s.cpuAvg), "--chart-cpu", 100);
    }
    if (ramCanvas) {
      drawSparkline(ramCanvas, h.map((s) => s.ramPct), "--chart-ram", 100);
    }
    if (netCanvas) {
      const rx = h.map((s) => s.netRx);
      const tx = h.map((s) => s.netTx);
      const maxNet = Math.max(...rx, ...tx, 1024);
      drawSparkline(netCanvas, rx, "--chart-net-rx", maxNet, tx, "--chart-net-tx");
    }
  });

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

  function colorForPct(pct: number): string {
    if (pct >= 80) return "var(--danger)";
    if (pct >= 60) return "var(--yellow)";
    return "var(--green)";
  }
</script>

{#if $stats && !collapsed}
  <div class="dashboard">
    <div class="metric-card">
      <div class="metric-header">
        <span class="metric-label">CPU</span>
        <span class="metric-value" style="color: {colorForPct(avgCpu)}">{avgCpu.toFixed(1)}%</span>
      </div>
      <canvas bind:this={cpuCanvas} class="spark" height={CHART_H}></canvas>
    </div>

    <div class="metric-card">
      <div class="metric-header">
        <span class="metric-label">RAM</span>
        <span class="metric-value" style="color: {colorForPct($stats.ram_used_pct)}">
          {$stats.ram_used_pct}% <span class="metric-sub">/ {$stats.ram_total_gb.toFixed(0)}GB</span>
        </span>
      </div>
      <canvas bind:this={ramCanvas} class="spark" height={CHART_H}></canvas>
    </div>

    <div class="metric-card">
      <div class="metric-header">
        <span class="metric-label">Network</span>
        <span class="metric-value net-values">
          <span class="net-rx">{formatRate($stats.net_rx_bytes_per_sec)}</span>
          <span class="net-tx">{formatRate($stats.net_tx_bytes_per_sec)}</span>
        </span>
      </div>
      <canvas bind:this={netCanvas} class="spark" height={CHART_H}></canvas>
    </div>

    <div class="metric-card metric-stats">
      <div class="stat-row">
        <span class="stat-label">{t("status.swap")}</span>
        <span class="stat-value">{$stats.swap_used_mb} MB</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">{t("status.procs")}</span>
        <span class="stat-value">{$stats.total_processes}</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">{t("status.idle")}</span>
        <span class="stat-value">{$processes.filter((p) => p.idle).length}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .dashboard {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr auto;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .metric-card {
    background: var(--bg-surface, var(--bg-alt));
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
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
    color: var(--fg-dim);
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
    color: var(--fg-dim);
  }

  .net-values {
    display: flex;
    gap: 8px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .net-rx { color: var(--chart-net-rx, var(--green)); }
  .net-tx { color: var(--chart-net-tx, var(--yellow)); }

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

  .stat-label {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--fg-dim);
  }

  .stat-value {
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--fg);
  }

  @media (max-width: 700px) {
    .dashboard {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
