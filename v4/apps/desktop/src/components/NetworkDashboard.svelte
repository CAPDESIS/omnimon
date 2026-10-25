<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import {
    getNetworkState,
    getTotalUp,
    getTotalDown,
    initNetworkListener
  } from "../stores/network.svelte";
  import ConnectionsTable from "./ConnectionsTable.svelte";
  import ProcessNetworkView from "./ProcessNetworkView.svelte";
  import { areaPath, linePath, maxRate } from "../lib/networkChart";

  const networkState = getNetworkState();
  const W = 600, H = 150, PAD = 8;
  const history = $derived(networkState.history);
  const chartMax = $derived(maxRate(history));
  const upLine = $derived(linePath(history.map((s) => s.total_bytes_per_sec_up), W, H, PAD, chartMax));
  const downLine = $derived(linePath(history.map((s) => s.total_bytes_per_sec_down), W, H, PAD, chartMax));
  const upArea = $derived(areaPath(history.map((s) => s.total_bytes_per_sec_up), W, H, PAD, chartMax));
  const downArea = $derived(areaPath(history.map((s) => s.total_bytes_per_sec_down), W, H, PAD, chartMax));

  let activeTab = $state<"connections" | "processes">("connections");

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;
    initNetworkListener().then((fn) => {
      if (disposed) {
        fn();
        return;
      }
      unlisten = fn;
    });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  });

  import { formatNetworkRate as formatSpeed } from "../lib/formatting";
</script>

<div class="network-dashboard">
  <div class="header">
    <div class="metric">
      <h3>{t("networkDashboard.totalUpload")}</h3>
      <p class="up">{formatSpeed(getTotalUp())} ↑</p>
    </div>
    <div class="metric">
      <h3>{t("networkDashboard.totalDownload")}</h3>
      <p class="down">{formatSpeed(getTotalDown())} ↓</p>
    </div>
    <div class="metric">
      <h3>{t("networkDashboard.activeConnections")}</h3>
      <p>{networkState.snapshot?.active_connections ?? 0}</p>
    </div>
    <div class="metric">
      <h3>{t("networkDashboard.processesWithNetwork")}</h3>
      <p>{networkState.snapshot?.processes_with_network ?? 0}</p>
    </div>
  </div>

  <div class="history-graph">
    {#if history.length >= 2}
      <svg data-testid="network-history-chart" viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label={t("networkDashboard.historyPlaceholder")}>
        <path d={downArea} fill="#3b82f6" opacity="0.18" />
        <path d={upArea} fill="#22c55e" opacity="0.18" />
        <path d={downLine} fill="none" stroke="#3b82f6" stroke-width="2" />
        <path d={upLine} fill="none" stroke="#22c55e" stroke-width="2" />
      </svg>
      <div class="legend"><span class="up">↑ {formatSpeed(getTotalUp())}</span><span class="down">↓ {formatSpeed(getTotalDown())}</span><span>{t("networkDashboard.historyPlaceholder")}</span></div>
    {:else}
      <p class="placeholder-text">{t("networkDashboard.historyPlaceholder")}</p>
    {/if}
  </div>

  <div class="tabs">
    <button
      class:active={activeTab === "connections"}
      onclick={() => activeTab = "connections"}
    >
      {t("networkDashboard.connectionsTable")}
    </button>
    <button
      class:active={activeTab === "processes"}
      onclick={() => activeTab = "processes"}
    >
      {t("networkDashboard.processView")}
    </button>
  </div>

  <div class="tab-content">
    {#if activeTab === "connections"}
      <ConnectionsTable />
    {:else}
      <ProcessNetworkView />
    {/if}
  </div>
</div>

<style>
  .network-dashboard {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 1rem;
    padding: 1rem;
    background: var(--bg-main, #f0f0f0);
  }

  .header {
    display: flex;
    gap: 1rem;
    justify-content: space-between;
  }

  .metric {
    background: var(--bg-surface, #fff);
    padding: 1rem;
    border-radius: 8px;
    flex: 1;
    text-align: center;
    box-shadow: 0 2px 4px rgba(0,0,0,0.05);
  }

  .metric h3 {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-secondary, #666);
  }

  .metric p {
    margin: 0.5rem 0 0 0;
    font-size: 1.5rem;
    font-weight: bold;
  }

  .up { color: #22c55e; }
  .down { color: #3b82f6; }

  .history-graph {
    background: var(--bg-surface, #fff);
    border-radius: 8px;
    min-height: 150px;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    border: 1px solid var(--border-color, #eee);
    padding: 0.5rem;
    gap: 0.25rem;
  }
  .history-graph svg { width: 100%; height: 120px; display: block; }
  .legend { display: flex; gap: 1rem; font-size: 0.8rem; color: var(--text-secondary, #666); }

  .tabs {
    display: flex;
    gap: 0.5rem;
  }

  .tabs button {
    padding: 0.5rem 1rem;
    border: none;
    background: var(--bg-surface, #fff);
    cursor: pointer;
    border-radius: 4px;
    font-weight: bold;
  }

  .tabs button.active {
    background: var(--primary-color, #3b82f6);
    color: white;
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    background: var(--bg-surface, #fff);
    border-radius: 8px;
    padding: 1rem;
  }

  .placeholder-text {
    color: #999;
    font-style: italic;
  }
</style>
