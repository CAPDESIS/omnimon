<script lang="ts">
  import type { ProcessEntry } from "../lib/types";
  import { chromeProcesses, killSingle, killSelected, selectedPids, toggleSelect } from "../stores/processes";

  let expanded = $state(true);
  let killing = $state<number | null>(null);

  let totalRam = $derived(
    $chromeProcesses.reduce((sum, p) => sum + p.ram_mb, 0),
  );

  let tabCount = $derived(
    $chromeProcesses.filter((p) => p.name.startsWith("Chrome Tab")).length,
  );

  let selectedTabs = $derived(
    $chromeProcesses.filter((p) => $selectedPids.has(p.pid)),
  );

  async function killTab(pid: number) {
    killing = pid;
    await killSingle(pid);
    killing = null;
  }

  async function killAllTabs() {
    const tabPids = $chromeProcesses
      .filter((p) => p.name.startsWith("Chrome Tab"))
      .map((p) => p.pid);
    if (tabPids.length === 0) return;

    // Select all tabs, then kill
    for (const pid of tabPids) {
      if (!$selectedPids.has(pid)) toggleSelect(pid);
    }
    await killSelected();
  }

  function ramColor(mb: number): string {
    if (mb >= 1024) return "var(--danger)";
    if (mb >= 256) return "var(--yellow)";
    return "var(--fg)";
  }
</script>

{#if $chromeProcesses.length > 0}
  <div class="chrome-manager">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="chrome-header" onclick={() => expanded = !expanded} role="button" tabindex="0">
      <span class="chevron" class:open={expanded}>&#9654;</span>
      <span class="chrome-icon">&#9679;</span>
      <span class="chrome-title">Chrome</span>
      <span class="chrome-meta">
        {tabCount} tab{tabCount !== 1 ? "s" : ""} &middot;
        <span style="color: {ramColor(totalRam)}">{totalRam.toFixed(0)} MB</span>
      </span>
      {#if tabCount > 0}
        <button
          class="btn-close-all"
          onclick={(e: MouseEvent) => { e.stopPropagation(); killAllTabs(); }}
          title="Close all Chrome tabs"
        >
          Close Tabs
        </button>
      {/if}
    </div>

    {#if expanded}
      <div class="chrome-list">
        {#each $chromeProcesses as proc (proc.pid)}
          <div
            class="chrome-row"
            class:selected={$selectedPids.has(proc.pid)}
            class:killing={killing === proc.pid}
          >
            <input
              type="checkbox"
              checked={$selectedPids.has(proc.pid)}
              onclick={(e: MouseEvent) => { e.stopPropagation(); toggleSelect(proc.pid); }}
            />
            <span class="tab-name" title={proc.exec_name}>{proc.name}</span>
            <span class="tab-ram mono" style="color: {ramColor(proc.ram_mb)}">
              {proc.ram_mb.toFixed(0)}
            </span>
            <span class="tab-cpu mono">
              {proc.cpu_pct.toFixed(1)}%
            </span>
            <button
              class="btn-kill"
              onclick={() => killTab(proc.pid)}
              disabled={killing === proc.pid}
              title="Close this process"
            >
              ✕
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .chrome-manager {
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .chrome-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 8px;
    background: var(--bg-alt);
    border: none;
    color: var(--fg);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
    height: 24px;
  }
  .chrome-header:hover {
    background: var(--bg-hover);
  }

  .chevron {
    font-size: 8px;
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .chevron.open {
    transform: rotate(90deg);
  }

  .chrome-icon {
    color: #4285f4;
    font-size: 10px;
  }

  .chrome-title {
    flex-shrink: 0;
  }

  .chrome-meta {
    flex: 1;
    color: var(--fg-dim);
    font-weight: 400;
    font-size: 10px;
  }

  .btn-close-all {
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--danger);
    font-size: 9px;
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .btn-close-all:hover {
    background: rgba(211, 47, 47, 0.1);
  }

  .chrome-list {
    max-height: 200px;
    overflow-y: auto;
  }

  .chrome-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 24px;
    height: 20px;
    font-size: 11px;
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.1));
    cursor: default;
  }
  .chrome-row:hover {
    background: var(--bg-hover);
  }
  .chrome-row.selected {
    background: var(--bg-selected);
  }
  .chrome-row.killing {
    opacity: 0.4;
  }

  .tab-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 10px;
  }

  .tab-ram {
    width: 45px;
    text-align: right;
    flex-shrink: 0;
  }

  .tab-cpu {
    width: 45px;
    text-align: right;
    flex-shrink: 0;
    color: var(--fg-dim);
  }

  input[type="checkbox"] {
    margin: 0;
    cursor: pointer;
    width: 12px;
    height: 12px;
  }

  .btn-kill {
    width: 16px;
    height: 16px;
    padding: 0;
    border: none;
    border-radius: 2px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 10px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .btn-kill:hover {
    background: rgba(211, 47, 47, 0.15);
    color: var(--danger);
  }
</style>
