<script lang="ts">
  import { stats, filtered, processes } from "../stores/processes";

  let ramColor = $derived(
    $stats
      ? $stats.ram_used_pct >= 80
        ? "var(--danger)"
        : $stats.ram_used_pct >= 60
          ? "var(--yellow)"
          : "var(--green)"
      : "var(--fg-dim)",
  );

  let avgCpu = $derived(
    $processes.length > 0
      ? $processes.reduce((sum, p) => sum + p.cpu_pct, 0) / $processes.length
      : 0,
  );

  let cpuColor = $derived(
    avgCpu >= 50 ? "var(--danger)" : avgCpu >= 25 ? "var(--yellow)" : "var(--green)",
  );

  let idleCount = $derived(
    $processes.filter((p) => p.idle).length,
  );
</script>

{#if $stats}
  <div class="status-bar">
    <div class="metric">
      <span class="label">RAM</span>
      <div class="bar-track" role="progressbar" aria-label="RAM usage" aria-valuenow={$stats.ram_used_pct} aria-valuemin={0} aria-valuemax={100}>
        <div
          class="bar-fill"
          style="width: {$stats.ram_used_pct}%; background: {ramColor}"
        ></div>
      </div>
      <span class="value" style="color: {ramColor}">
        {$stats.ram_used_pct}% of {$stats.ram_total_gb.toFixed(0)}GB
      </span>
    </div>
    <div class="metric">
      <span class="label">CPU</span>
      <div class="bar-track" role="progressbar" aria-label="Average CPU usage" aria-valuenow={Math.round(avgCpu)} aria-valuemin={0} aria-valuemax={100}>
        <div
          class="bar-fill"
          style="width: {Math.min(avgCpu, 100)}%; background: {cpuColor}"
        ></div>
      </div>
      <span class="value" style="color: {cpuColor}">
        {avgCpu.toFixed(1)}%
      </span>
    </div>
    <div class="metric">
      <span class="label">Swap</span>
      <span class="value">{$stats.swap_used_mb} MB</span>
    </div>
    <div class="metric">
      <span class="label">Procs</span>
      <span class="value">{$filtered.length}</span>
    </div>
    <div class="metric">
      <span class="label">Idle</span>
      <span class="value">{idleCount}</span>
    </div>
  </div>
{/if}

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 3px 10px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    font-size: 10px;
    flex-shrink: 0;
    height: 20px;
  }

  .metric {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .label {
    color: var(--fg-dim);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 9px;
    letter-spacing: 0.3px;
  }

  .value {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 10px;
  }

  .bar-track {
    width: 60px;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    border-radius: 2px;
    transition: width 0.3s ease;
  }
</style>
