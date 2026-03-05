<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import ProcessTable from "./components/ProcessTable.svelte";
  import ChromeTabManager from "./components/ChromeTabManager.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import ProcessDetailsModal from "./components/ProcessDetailsModal.svelte";
  import type { ProcessEntry } from "./lib/types";
  import {
    processes,
    filtered,
    loading,
    search,
    selectedPids,
    selectedCount,
    selectedRamMB,
    focusedPid,
    grouping,
    startPolling,
    stopPolling,
    killSelected,
    selectAllVisible,
    selectNone,
  } from "./stores/processes";

  let detailProcess: ProcessEntry | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let searchValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  function onSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    searchValue = val;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      $search = val;
    }, 150);
  }

  onMount(() => {
    startPolling(2000);
    return () => {
      stopPolling();
      clearTimeout(debounceTimer);
    };
  });

  function openDetailForFocused() {
    const pid = get(focusedPid);
    if (pid == null) return;
    const proc = get(processes).find((p) => p.pid === pid);
    if (proc) detailProcess = proc;
  }

  function handleKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    const inInput =
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement;

    // Cmd/Ctrl+F → focus search
    if (mod && e.key === "f") {
      e.preventDefault();
      searchInput?.focus();
      return;
    }

    // Cmd/Ctrl+I → inspect focused process
    if (mod && e.key === "i") {
      e.preventDefault();
      openDetailForFocused();
      return;
    }

    // Escape → close modal or blur search
    if (e.key === "Escape") {
      if (detailProcess) {
        detailProcess = null;
        return;
      }
      if (inInput) {
        (e.target as HTMLElement).blur();
        return;
      }
    }

    // Delete/Backspace → kill selected (only when not typing)
    if ((e.key === "Delete" || e.key === "Backspace") && !inInput) {
      e.preventDefault();
      killSelected();
      return;
    }
  }

  function inspectProcess(proc: ProcessEntry) {
    detailProcess = proc;
  }

  function closeDetail() {
    detailProcess = null;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main>
  <header class="toolbar">
    <input
      class="search"
      type="text"
      placeholder="Filter by name, PID, group... (Cmd+F)"
      aria-label="Search processes"
      value={searchValue}
      oninput={onSearchInput}
      bind:this={searchInput}
    />
    <div class="actions">
      <button
        class="btn btn-sm"
        class:active={$grouping}
        onclick={() => $grouping = !$grouping}
        title="Toggle grouping"
      >
        Groups
      </button>
      <button class="btn btn-sm" onclick={selectAllVisible} aria-label="Select all processes">All</button>
      <button class="btn btn-sm" onclick={selectNone} aria-label="Deselect all processes">None</button>
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
    <ProcessTable
      processes={$filtered}
      grouping={$grouping}
      oninspect={inspectProcess}
    />
  {/if}

  <footer class="statusline" aria-live="polite" aria-atomic="true">
    {$filtered.length} processes{#if $filtered.length !== $processes.length}
      &nbsp;(filtered from {$processes.length}){/if}
    {#if $selectedCount > 0}
      <span aria-hidden="true">&nbsp;&middot;&nbsp;</span>{$selectedCount} selected ({$selectedRamMB.toFixed(0)} MB)
    {/if}
    <span class="shortcuts" aria-hidden="true"><kbd>Cmd+I</kbd> detail <kbd>Cmd+F</kbd> search <kbd>Del</kbd> close</span>
  </footer>
</main>

{#if detailProcess}
  <ProcessDetailsModal process={detailProcess} onclose={closeDetail} />
{/if}

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
    --fg-dim: #888;
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
  .btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
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
    display: flex;
    justify-content: space-between;
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

  .shortcuts {
    opacity: 0.5;
  }
  .shortcuts :global(kbd) {
    font-family: inherit;
    font-size: inherit;
  }
</style>
