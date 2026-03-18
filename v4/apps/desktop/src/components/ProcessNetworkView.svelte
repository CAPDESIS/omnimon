<script lang="ts">
  import { t } from "../lib/i18n";
  import {
    getPerProcessSummary,
  } from "../stores/network.svelte";

  let expandedRow: string | null = $state(null);

  import { formatNetworkRate as formatSpeed } from "../lib/formatting";
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
          {/each}
        </tbody>
      </table>
    </div>
    
    <div class="chart-container">
      <p class="placeholder-text">{t("processNetwork.chartPlaceholder")}</p>
      <!-- TODO: Implement actual charting library component -->
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
    align-items: center;
    justify-content: center;
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

  .up { color: #22c55e; }
  .down { color: #3b82f6; }
</style>
