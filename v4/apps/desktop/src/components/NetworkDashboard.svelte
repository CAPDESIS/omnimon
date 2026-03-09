<script lang="ts">
  import { onMount } from "svelte";
  import {
    getNetworkState,
    getTotalUp,
    getTotalDown,
    initNetworkListener
  } from "../stores/network.svelte";
  import ConnectionsTable from "./ConnectionsTable.svelte";
  import ProcessNetworkView from "./ProcessNetworkView.svelte";

  const networkState = getNetworkState();

  let activeTab = $state<"connections" | "processes">("connections");

  onMount(() => {
    let unlisten: (() => void) | null = null;
    initNetworkListener().then(fn => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  });

  function formatSpeed(bytesPerSec: number) {
    if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
    if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  }
</script>

<div class="network-dashboard">
  <div class="header">
    <div class="metric">
      <h3>Total Upload</h3>
      <p class="up">{formatSpeed(getTotalUp())} ↑</p>
    </div>
    <div class="metric">
      <h3>Total Download</h3>
      <p class="down">{formatSpeed(getTotalDown())} ↓</p>
    </div>
    <div class="metric">
      <h3>Conexiones activas</h3>
      <p>{networkState.snapshot?.active_connections ?? 0}</p>
    </div>
    <div class="metric">
      <h3>Procesos con red</h3>
      <p>{networkState.snapshot?.processes_with_network ?? 0}</p>
    </div>
  </div>

  <div class="history-graph">
    <p class="placeholder-text">Gráfica histórica de red (últimos 5 min)</p>
    <!-- TODO: Implement chart with state.history -->
  </div>

  <div class="tabs">
    <button
      class:active={activeTab === "connections"}
      onclick={() => activeTab = "connections"}
    >
      Tabla de Conexiones
    </button>
    <button
      class:active={activeTab === "processes"}
      onclick={() => activeTab = "processes"}
    >
      Vista por Proceso
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
    height: 150px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-color, #eee);
  }

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
