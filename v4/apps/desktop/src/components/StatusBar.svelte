<script lang="ts">
  import { stats, filtered, processes } from "../stores/processes";
  import { networkTelemetryStatus } from "../stores/security";
  import { t } from "../lib/i18n";
  import { Radar } from "lucide-svelte";

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

  import { formatNetworkRate } from "../lib/formatting";
  const formatRate = formatNetworkRate;
</script>

{#if $stats}
  <div class="status-bar">
    <div class="metric">
      <span class="label">{t("status.ram")}</span>
      <div class="bar-track" role="progressbar" aria-label={t("status.ramUsage")} aria-valuenow={$stats.ram_used_pct} aria-valuemin={0} aria-valuemax={100}>
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
      <span class="label">{t("status.cpu")}</span>
      <div class="bar-track" role="progressbar" aria-label={t("status.cpuUsage")} aria-valuenow={Math.round(avgCpu)} aria-valuemin={0} aria-valuemax={100}>
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
      <span class="label">{t("status.swap")}</span>
      <span class="value">{$stats.swap_used_mb} MB</span>
    </div>
    <div class="metric">
      <span class="label">{t("status.net")}</span>
      <span class="value">↓ {formatRate($stats.net_rx_bytes_per_sec)}</span>
      <span class="value">↑ {formatRate($stats.net_tx_bytes_per_sec)}</span>
    </div>
    <div class="metric">
      <span class="label">{t("status.procs")}</span>
      <span class="value">{$filtered.length}</span>
    </div>
    <div class="metric">
      <span class="label">{t("status.idle")}</span>
      <span class="value">{idleCount}</span>
    </div>
    {#if $networkTelemetryStatus.dpiActive}
      <!--
        DPI disclosure. Surfaces the fact that Deep Packet Inspection is
        active so users on regulated networks (GDPR, workplace policy) are
        never monitored without seeing it in the UI.
      -->
      <div
        class="metric dpi-badge"
        role="status"
        aria-live="polite"
        title={t("status.dpiTooltip")}
      >
        <Radar size={12} aria-hidden="true" />
        <span class="label dpi-label">{t("status.dpi")}</span>
        <span class="value dpi-value">{t("status.dpiActive")}</span>
      </div>
    {/if}
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
    font-size: calc(var(--base-font-size) * 0.833);
    flex-shrink: 0;
    min-height: calc(var(--base-font-size) * 1.667);
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
    font-size: calc(var(--base-font-size) * 0.75);
    letter-spacing: 0.3px;
  }

  .value {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.833);
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

  .dpi-badge {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--warning, var(--accent));
    color: var(--warning, var(--accent));
    gap: 4px;
  }

  .dpi-label {
    color: var(--warning, var(--accent));
  }

  .dpi-value {
    color: var(--warning, var(--accent));
    font-weight: 600;
  }
</style>
