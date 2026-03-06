<script lang="ts">
  import type { BrowserTab } from "../lib/types";
  import { ipcCloseBrowserTab, ipcFocusBrowserTab } from "../lib/ipc";
  import { browserTabs, chromeProcesses } from "../stores/processes";
  import { confirmAction } from "../lib/confirm";
  import { t } from "../lib/i18n";
  import { detectBrowser } from "../lib/browser";

  interface Props {
    filter?: string;
  }

  let { filter = "" }: Props = $props();

  let expandedBrowsers = $state<Set<string>>(new Set(["Chrome", "Safari", "Brave", "Edge", "Arc", "Firefox"]));
  let closing = $state<Set<string>>(new Set());
  let selectedTabIds = $state<Set<string>>(new Set());

  interface BrowserSection {
    name: string;
    color: string;
    tabs: BrowserTab[];
    totalTabs: number;
    totalRam: number;
  }

  const BROWSER_COLORS: Record<string, string> = {
    Chrome: "#4285f4",
    Safari: "#007aff",
    Brave: "#fb542b",
    Edge: "#0078d4",
    Arc: "#a855f7",
    Firefox: "#ff7139",
  };

  function extractDomain(url: string): string {
    try {
      return new URL(url).hostname;
    } catch {
      return url;
    }
  }

  /** Per-browser RAM from process list */
  let ramByBrowser = $derived.by((): Map<string, number> => {
    const map = new Map<string, number>();
    for (const p of $chromeProcesses) {
      const browser = detectBrowser(p);
      if (browser) {
        map.set(browser, (map.get(browser) ?? 0) + p.ram_mb);
      }
    }
    return map;
  });

  let sections = $derived.by((): BrowserSection[] => {
    const q = filter.trim().toLowerCase();
    const map = new Map<string, { filtered: BrowserTab[]; total: number }>();
    for (const tab of $browserTabs) {
      const entry = map.get(tab.browser) ?? { filtered: [], total: 0 };
      entry.total++;
      if (!q || tab.title.toLowerCase().includes(q) || tab.url.toLowerCase().includes(q) || extractDomain(tab.url).toLowerCase().includes(q)) {
        entry.filtered.push(tab);
      }
      map.set(tab.browser, entry);
    }
    return [...map.entries()].map(([name, { filtered: tabs, total }]) => ({
      name,
      color: BROWSER_COLORS[name] ?? "var(--fg-dim)",
      tabs,
      totalTabs: total,
      totalRam: ramByBrowser.get(name) ?? 0,
    }));
  });

  let selectedCount = $derived(selectedTabIds.size);

  function toggleBrowserExpanded(name: string) {
    const next = new Set(expandedBrowsers);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    expandedBrowsers = next;
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

  async function focusTab(tab: BrowserTab) {
    try {
      await ipcFocusBrowserTab(tab.id, tab.url, tab.browser);
    } catch (e) {
      console.error("Failed to focus tab:", e);
    }
  }

  async function closeTab(tab: BrowserTab) {
    if (!confirmAction(t("tabs.confirmCloseTab", { title: tab.title }))) return;
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
    if (toClose.length === 0) return;
    if (!confirmAction(t("tabs.confirmCloseSelected", { count: toClose.length }))) return;
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
    if (tabs.length === 0) return;
    if (!confirmAction(t("tabs.confirmCloseAll", { count: tabs.length }))) return;
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

{#snippet tabSection(section: BrowserSection)}
  {@const isExpanded = expandedBrowsers.has(section.name)}
  <div class="browser-section">
    <div
      class="browser-header"
      onclick={() => toggleBrowserExpanded(section.name)}
      onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleBrowserExpanded(section.name); } }}
      role="button"
      tabindex="0"
      aria-expanded={isExpanded}
      aria-label={t("tabs.browserTabs", { browser: section.name })}
    >
      <span class="chevron" class:open={isExpanded} aria-hidden="true">&#9654;</span>
      <span class="browser-icon" style="color: {section.color}" aria-hidden="true">&#9679;</span>
      <span class="browser-title">{section.name}</span>
      <span class="browser-meta">
        {#if section.tabs.length < section.totalTabs}
          {section.totalTabs !== 1 ? t("tabs.tabCountFilteredPlural", { count: section.tabs.length, total: section.totalTabs }) : t("tabs.tabCountFiltered", { count: section.tabs.length, total: section.totalTabs })}
        {:else}
          {section.tabs.length !== 1 ? t("tabs.tabCountPlural", { count: section.tabs.length }) : t("tabs.tabCount", { count: section.tabs.length })}
        {/if}
        &middot; <span style="color: {ramColor(section.totalRam)}">{t("tabs.mb", { value: section.totalRam.toFixed(0) })}</span>
      </span>
      <div class="header-actions">
        <button
          class="btn-header"
          onclick={(e: MouseEvent) => { e.stopPropagation(); selectAllTabs(section.tabs); }}
          title={t("tabs.selectAllBrowser", { browser: section.name })}
        >{t("common.all")}</button>
        <button
          class="btn-header"
          onclick={(e: MouseEvent) => { e.stopPropagation(); selectNoneTabs(); }}
          title={t("tabs.deselectAll")}
        >{t("common.none")}</button>
        {#if selectedCount > 0}
          <button
            class="btn-close-selected"
            onclick={(e: MouseEvent) => { e.stopPropagation(); closeSelected(); }}
            title={t("tabs.closeCountTitle", { count: selectedCount })}
          >
            {t("tabs.closeCount", { count: selectedCount })}
          </button>
        {/if}
        <button
          class="btn-close-all"
          onclick={(e: MouseEvent) => { e.stopPropagation(); closeAllTabs(section.tabs); }}
          title={t("tabs.closeAllTitle", { browser: section.name })}
        >
          {t("tabs.closeAll")}
        </button>
      </div>
    </div>

    {#if isExpanded && section.tabs.length > 0}
      <div class="tab-list">
        <div class="tab-list-header sticky-header">
          <span class="th-check"></span>
          <span class="th-name">{t("tabs.title")}</span>
          <span class="th-domain">{t("tabs.domain")}</span>
          <span class="th-action"></span>
        </div>
        {#each section.tabs as tab (tab.id)}
          <div
            class="tab-row"
            class:closing={closing.has(tab.id)}
            class:selected={selectedTabIds.has(tab.id)}
            onclick={() => toggleTab(tab.id)}
            onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleTab(tab.id); } }}
            tabindex="0"
            role="row"
          >
            <input
              type="checkbox"
              checked={selectedTabIds.has(tab.id)}
              aria-label={t("tabs.selectTab", { title: tab.title })}
              onclick={(e: MouseEvent) => { e.stopPropagation(); toggleTab(tab.id); }}
            />
            <button
              class="tab-title-btn"
              title={t("tabs.focusTab", { browser: tab.browser })}
              onclick={(e: MouseEvent) => { e.stopPropagation(); focusTab(tab); }}
            >{tab.title || t("common.untitled")}</button>
            <span class="tab-domain mono" title={tab.url}>{extractDomain(tab.url)}</span>
            <button
              class="btn-kill"
              onclick={(e: MouseEvent) => { e.stopPropagation(); closeTab(tab); }}
              disabled={closing.has(tab.id)}
              title={t("tabs.closeThisTab")}
            >
              &#10005;
            </button>
          </div>
        {/each}
      </div>
    {:else if isExpanded && section.tabs.length === 0}
      <div class="tab-empty">{t("common.noMatchingTabs")}</div>
    {/if}
  </div>
{/snippet}

{#if sections.length > 0}
  <div class="chrome-manager">
    {#each sections as section (section.name)}
      {@render tabSection(section)}
    {/each}
  </div>
{/if}

<style>
  .chrome-manager {
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
    min-width: 0;
    flex: 1;
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
    font-size: calc(var(--base-font-size) * 0.917);
    font-weight: 600;
    cursor: pointer;
    text-align: left;
    min-height: calc(var(--base-font-size) * 2);
  }
  .browser-header:hover {
    background: var(--bg-hover);
  }

  .chevron {
    font-size: calc(var(--base-font-size) * 0.667);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .chevron.open {
    transform: rotate(90deg);
  }

  .browser-icon {
    font-size: calc(var(--base-font-size) * 0.833);
  }

  .browser-title {
    flex-shrink: 0;
  }

  .browser-meta {
    flex: 1;
    color: var(--fg-dim);
    font-weight: 400;
    font-size: calc(var(--base-font-size) * 0.833);
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
    font-size: calc(var(--base-font-size) * 0.75);
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
    font-size: calc(var(--base-font-size) * 0.75);
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
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .btn-close-all:hover {
    background: rgba(211, 47, 47, 0.1);
  }

  .tab-list {
    overflow-y: auto;
  }

  .tab-list-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 12px;
    min-height: calc(var(--base-font-size) * 1.5);
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--fg-dim);
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.1));
  }

  .sticky-header {
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .th-check {
    width: 16px;
    flex-shrink: 0;
  }
  .th-name {
    flex: 3;
    min-width: 120px;
  }
  .th-domain {
    flex: 2;
    min-width: 100px;
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
    min-height: calc(var(--base-font-size) * 1.833);
    font-size: calc(var(--base-font-size) * 0.917);
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

  .tab-title-btn {
    flex: 3;
    min-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: none;
    border: none;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    padding: 0;
    font: inherit;
  }
  .tab-title-btn:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .tab-domain {
    flex: 2;
    min-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-dim);
  }

  .tab-empty {
    padding: 6px 12px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    font-style: italic;
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.833);
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
    font-size: var(--base-font-size);
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
