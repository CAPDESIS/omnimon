<script lang="ts">
  import { t } from "../lib/i18n";
  import {
    getPerProcessSummary,
  } from "../stores/network.svelte";

  let expandedRow: string | null = $state(null);

  import { formatNetworkRate as formatSpeed } from "../lib/formatting";
  import { donutSegments } from "../lib/networkChart";
  const segments = $derived(donutSegments(getPerProcessSummary(), 70));
</script>

<div class="process-network-container">
  <h2>{t("processNetwork.title")}</h2>
  
  <div class="layout">
    <div class="process-list">
      <table class="process-table">
        <thead>
          <tr>
            <th>{t("processNetwork.process")}</th>
            <th>{t("processNetwork.connections")}</th>
            <th>↑ Total</th>
            <th>↓ Total</th>
            <th>{t("processNetwork.topDestination")}</th>
          </tr>
        </thead>
        <tbody>
          {#each getPerProcessSummary() as proc}
            <tr onclick={() => expandedRow = expandedRow === proc.name ? null : proc.name}>
              <td>{proc.name}</td>
              <td>{proc.connectionsCount}</td>
              <td class="up">{formatSpeed(proc.totalUp)}</td>
              <td class="down">{formatSpeed(proc.totalDown)}</td>
              <td>{proc.topDest}</td>
            </tr>
          {:else}
            <tr>
              <td colspan="5" class="empty-state">{t("processNetwork.emptyState")}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    
    <div class="chart-container">
      {#if segments.length > 0}
        <svg data-testid="process-donut-chart" viewBox="0 0 180 180" role="img" aria-label={t("processNetwork.chartPlaceholder")}>
          <circle cx="90" cy="90" r="70" fill="none" stroke="var(--border-color, #eee)" stroke-width="22" />
          {#each segments as seg}
            <circle cx="90" cy="90" r="70" fill="none" stroke={seg.color} stroke-width="22"
              stroke-dasharray={seg.dash} stroke-dashoffset={seg.offset} transform="rotate(-90 90 90)"><title>{seg.name}: {(seg.share * 100).toFixed(1)}%</title></circle>
          {/each}
        </svg>
        <ul class="donut-legend">
          {#each segments as seg}<li><span class="swatch" style="background:{seg.color}"></span>{seg.name} {(seg.share * 100).toFixed(1)}% · {formatSpeed(seg.total)}</li>{/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .process-network-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
    padding: 1rem;
    background: var(--bg-surface, #fff);
  }

  .layout {
    display: flex;
    gap: 1rem;
  }

  .process-list {
    flex: 2;
    overflow-y: auto;
  }

  .chart-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: var(--bg-surface-2, #f9f9f9);
    border: 1px solid var(--border-color, #eee);
    border-radius: 8px;
    min-height: 200px;
  }

  .process-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
  }

  .process-table th {
    padding: 0.5rem;
    background: var(--bg-header, #e0e0e0);
  }

  .process-table td {
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-color, #ccc);
  }

  .process-table tr:hover td {
    background: var(--bg-hover, #f0f0f0);
    cursor: pointer;
  }

  .empty-state {
    text-align: center;
    color: #999;
    font-style: italic;
  }

  .up { color: #22c55e; }
  .down { color: #3b82f6; }
  .chart-container svg { width: 180px; height: 180px; }
  .donut-legend { list-style: none; margin: 0; padding: 0; font-size: 0.8rem; display: flex; flex-direction: column; gap: 0.25rem; }
  .swatch { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 0.4rem; }
</style>
