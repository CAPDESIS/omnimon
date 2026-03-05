<script lang="ts">
  import { onMount } from "svelte";
  import ProcessTable from "./components/ProcessTable.svelte";
  import ChromeTabManager from "./components/ChromeTabManager.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import {
    processes,
    filtered,
    loading,
    search,
    selectedPids,
    selectedCount,
    selectedRamMB,
    startPolling,
    stopPolling,
    killSelected,
    selectAllVisible,
    selectNone,
  } from "./stores/processes";

  onMount(() => {
    startPolling(2000);
    return stopPolling;
  });
</script>

<main>
  <header class="toolbar">
    <input
      class="search"
      type="text"
      placeholder="Filter by name, PID, group..."
      bind:value={$search}
    />
    <div class="actions">
      <button class="btn btn-sm" onclick={selectAllVisible}>All</button>
      <button class="btn btn-sm" onclick={selectNone}>None</button>
      <button
        class="btn btn-kill"
        onclick={killSelected}
        disabled={$selectedCount === 0}
      >
        Close{#if $selectedCount > 0}
          &nbsp;({$selectedCount} &middot; {$selectedRamMB.toFixed(0)} MB){/if}
      </button>
    </div>
  </header>

  <StatusBar />
  <ChromeTabManager />

  {#if $loading}
    <div class="loading">Loading...</div>
  {:else}
    <ProcessTable processes={$filtered} />
  {/if}

  <footer class="statusline">
    {$filtered.length} processes{#if $filtered.length !== $processes.length}
      &nbsp;(filtered from {$processes.length}){/if}
    {#if $selectedCount > 0}
      &nbsp;&middot;&nbsp;{$selectedCount} selected ({$selectedRamMB.toFixed(0)} MB)
    {/if}
  </footer>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI",
      Roboto, "Helvetica Neue", sans-serif;
    font-size: 12px;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }

  :global(:root) {
    --bg: #1a1a1a;
    --bg-alt: #222;
    --bg-hover: #2a2a2a;
    --bg-selected: #0a3d6e;
    --fg: #ccc;
    --fg-dim: #777;
    --border: #333;
    --accent: #0078d4;
    --danger: #d32f2f;
    --green: #4caf50;
    --yellow: #ffc107;
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #f8f8f8;
      --bg-alt: #eee;
      --bg-hover: #e0e0e0;
      --bg-selected: #cce5ff;
      --fg: #1a1a1a;
      --fg-dim: #666;
      --border: #d0d0d0;
    }
  }

  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    height: 28px;
  }

  .search {
    flex: 1;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: 11px;
    outline: none;
    height: 20px;
  }
  .search:focus {
    border-color: var(--accent);
  }

  .actions {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }

  .btn {
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: 10px;
    cursor: pointer;
    white-space: nowrap;
    height: 20px;
    line-height: 14px;
  }
  .btn:hover {
    background: var(--bg-hover);
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .btn-sm {
    padding: 2px 6px;
  }
  .btn-kill {
    background: var(--danger);
    color: white;
    border-color: var(--danger);
    font-weight: 600;
  }
  .btn-kill:hover:not(:disabled) {
    background: #b71c1c;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    font-size: 11px;
  }

  .statusline {
    padding: 2px 8px;
    font-size: 10px;
    color: var(--fg-dim);
    background: var(--bg-alt);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    height: 18px;
    line-height: 14px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }
</style>
