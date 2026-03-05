<script lang="ts">
  import type { BrowserTab } from "../lib/types";
  import { ipcCloseBrowserTab } from "../lib/ipc";
  import { browserTabs, chromeProcesses } from "../stores/processes";

  let expanded = $state(true);
  let closing = $state<Set<string>>(new Set());
  let selectedTabIds = $state<Set<string>>(new Set());

  interface BrowserSection {
    name: string;
    color: string;
    tabs: BrowserTab[];
  }

  const BROWSER_COLORS: Record<string, string> = {
    Chrome: "#4285f4",
    Safari: "#007aff",
    Brave: "#fb542b",
    Edge: "#0078d4",
    Arc: "#a855f7",
    Firefox: "#ff7139",
  };

  let sections = $derived.by((): BrowserSection[] => {
    const map = new Map<string, BrowserTab[]>();
    for (const tab of $browserTabs) {
      const arr = map.get(tab.browser);
      if (arr) arr.push(tab);
      else map.set(tab.browser, [tab]);
    }
    return [...map.entries()].map(([name, tabs]) => ({
      name,
      color: BROWSER_COLORS[name] ?? "var(--fg-dim)",
      tabs,
    }));
  });

  let totalRam = $derived(
    $chromeProcesses.reduce((sum, p) => sum + p.ram_mb, 0),
  );

  let selectedCount = $derived(selectedTabIds.size);

  function extractDomain(url: string): string {
    try {
      return new URL(url).hostname;
    } catch {
      return url;
    }
  }

  function ramColor(mb: number): string {
    if (mb >= 1024) return "var(--danger)";
    if (mb >= 256) return "var(--yellow)";
    return "var(--fg)";
  }

  function toggleTab(tabId: string) {
    const next = new Set(selectedTabIds);
    if (next.has(tabId)) next.delete(tabId);
    else next.add(tabId);
    selectedTabIds = next;
  }

  function selectAllTabs(tabs: BrowserTab[]) {
    selectedTabIds = new Set(tabs.map((t) => t.id));
  }

  function selectNoneTabs() {
    selectedTabIds = new Set();
  }

  function removeTabFromStore(tabId: string) {
    browserTabs.update(($tabs) => $tabs.filter((t) => t.id !== tabId));
    selectedTabIds.delete(tabId);
    selectedTabIds = new Set(selectedTabIds);
  }

  async function closeTab(tab: BrowserTab) {
    const next = new Set(closing);
    next.add(tab.id);
    closing = next;
    try {
      await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
      removeTabFromStore(tab.id);
    } catch (e) {
      console.error("Failed to close tab:", e);
    }
    const after = new Set(closing);
    after.delete(tab.id);
    closing = after;
  }

  async function closeSelected() {
    const allTabs = $browserTabs;
    const toClose = allTabs.filter((t) => selectedTabIds.has(t.id));
    for (const tab of toClose) {
      const next = new Set(closing);
      next.add(tab.id);
      closing = next;
      try {
        await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
        removeTabFromStore(tab.id);
      } catch {
        // continue closing others
      }
      const after = new Set(closing);
      after.delete(tab.id);
      closing = after;
    }
  }

  async function closeAllTabs(tabs: BrowserTab[]) {
    for (const tab of tabs) {
      const next = new Set(closing);
      next.add(tab.id);
      closing = next;
      try {
        await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
        removeTabFromStore(tab.id);
      } catch {
        // continue
      }
      const after = new Set(closing);
      after.delete(tab.id);
      closing = after;
    }
  }
</script>

{#snippet tabSection(label: string, tabs: BrowserTab[], iconColor: string)}
  {#if tabs.length > 0}
    <div class="browser-section">
      <div
        class="browser-header"
        onclick={() => expanded = !expanded}
        onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); expanded = !expanded; } }}
        role="button"
        tabindex="0"
        aria-expanded={expanded}
        aria-label="{label} tabs"
      >
        <span class="chevron" class:open={expanded} aria-hidden="true">&#9654;</span>
        <span class="browser-icon" style="color: {iconColor}" aria-hidden="true">&#9679;</span>
        <span class="browser-title">{label}</span>
        <span class="browser-meta">
          {tabs.length} tab{tabs.length !== 1 ? "s" : ""}
          &middot; <span style="color: {ramColor(totalRam)}">{totalRam.toFixed(0)} MB</span>
        </span>
        <div class="header-actions">
          <button
            class="btn-header"
            onclick={(e: MouseEvent) => { e.stopPropagation(); selectAllTabs(tabs); }}
            title="Select all {label} tabs"
          >All</button>
          <button
            class="btn-header"
            onclick={(e: MouseEvent) => { e.stopPropagation(); selectNoneTabs(); }}
            title="Deselect all"
          >None</button>
          {#if selectedCount > 0}
            <button
              class="btn-close-selected"
              onclick={(e: MouseEvent) => { e.stopPropagation(); closeSelected(); }}
              title="Close {selectedCount} selected tab(s)"
            >
              Close {selectedCount}
            </button>
          {/if}
          <button
            class="btn-close-all"
            onclick={(e: MouseEvent) => { e.stopPropagation(); closeAllTabs(tabs); }}
            title="Close all {label} tabs"
          >
            Close All
          </button>
        </div>
      </div>

      {#if expanded}
        <div class="tab-list">
          <div class="tab-list-header">
            <span class="th-check"></span>
            <span class="th-name">Title</span>
            <span class="th-domain">Domain</span>
            <span class="th-url">URL</span>
            <span class="th-action"></span>
          </div>
          {#each tabs as tab (tab.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="tab-row"
              class:closing={closing.has(tab.id)}
              class:selected={selectedTabIds.has(tab.id)}
              onclick={() => toggleTab(tab.id)}
            >
              <input
                type="checkbox"
                checked={selectedTabIds.has(tab.id)}
                aria-label="Select {tab.title}"
                onclick={(e: MouseEvent) => { e.stopPropagation(); toggleTab(tab.id); }}
              />
              <span class="tab-title" title={tab.title}>{tab.title || "(Untitled)"}</span>
              <span class="tab-domain mono" title={extractDomain(tab.url)}>{extractDomain(tab.url)}</span>
              <span class="tab-url mono" title={tab.url}>{tab.url}</span>
              <button
                class="btn-kill"
                onclick={(e: MouseEvent) => { e.stopPropagation(); closeTab(tab); }}
                disabled={closing.has(tab.id)}
                title="Close this tab"
              >
                &#10005;
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

{#if sections.length > 0}
  <div class="chrome-manager">
    {#each sections as section (section.name)}
      {@render tabSection(section.name, section.tabs, section.color)}
    {/each}
  </div>
{/if}

<style>
  .chrome-manager {
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .browser-section {
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.1));
  }
  .browser-section:last-child {
    border-bottom: none;
  }

  .browser-header {
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
  .browser-header:hover {
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

  .browser-icon {
    font-size: 10px;
  }

  .browser-title {
    flex-shrink: 0;
  }

  .browser-meta {
    flex: 1;
    color: var(--fg-dim);
    font-weight: 400;
    font-size: 10px;
  }

  .header-actions {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }

  .btn-header {
    padding: 1px 5px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 9px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn-header:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .btn-close-selected {
    padding: 1px 6px;
    border: 1px solid var(--danger);
    border-radius: 3px;
    background: var(--danger);
    color: white;
    font-size: 9px;
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .btn-close-selected:hover {
    background: #b71c1c;
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

  .tab-list {
    max-height: 240px;
    overflow-y: auto;
  }

  .tab-list-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 12px;
    height: 18px;
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--fg-dim);
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.1));
  }

  .th-check {
    width: 16px;
    flex-shrink: 0;
  }
  .th-name {
    flex: 2;
    min-width: 120px;
  }
  .th-domain {
    flex: 1;
    min-width: 100px;
  }
  .th-url {
    flex: 2;
    min-width: 120px;
  }
  .th-action {
    width: 22px;
    flex-shrink: 0;
  }

  .tab-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 12px;
    height: 22px;
    font-size: 11px;
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.1));
    cursor: pointer;
  }
  .tab-row:hover {
    background: var(--bg-hover);
  }
  .tab-row.selected {
    background: var(--bg-selected);
  }
  .tab-row.closing {
    opacity: 0.4;
    pointer-events: none;
  }

  .tab-title {
    flex: 2;
    min-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-domain {
    flex: 1;
    min-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-dim);
  }

  .tab-url {
    flex: 2;
    min-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-dim);
    font-size: 10px;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 10px;
  }

  input[type="checkbox"] {
    margin: 0;
    cursor: pointer;
    width: 13px;
    height: 13px;
    flex-shrink: 0;
  }

  .btn-kill {
    width: 22px;
    height: 22px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .btn-kill:hover {
    background: rgba(211, 47, 47, 0.15);
    color: var(--danger);
    border-color: var(--danger);
  }
</style>
