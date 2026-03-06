<script lang="ts">
  import { onMount } from "svelte";
  import type { ProcessEntry, BrowserTab } from "../lib/types";
  import { ipcCloseBrowserTab, ipcFocusBrowserTab, ipcAnalyzeContext } from "../lib/ipc";
  import { browserTabs } from "../stores/processes";
  import { aiProviderConfig } from "../stores/preferences";
  import { t } from "../lib/i18n";

  interface Props {
    process: ProcessEntry;
    onclose: () => void;
  }

  let { process, onclose }: Props = $props();
  let modalEl: HTMLDivElement | undefined = $state();

  // Tab management state
  let selectedTabIds = $state<Set<string>>(new Set());
  let closingTabs = $state<Set<string>>(new Set());
  let tabFilter = $state("");

  // AI analysis state
  let aiResponse = $state("");
  let aiAnalyzing = $state(false);
  let aiError = $state<string | null>(null);

  let detectedBrowser = $derived.by((): string | null => {
    if (process.group !== "Browser") return null;
    if (process.exec_name.includes("Google Chrome") || process.name.includes("Chrome")) return "Chrome";
    if (process.name === "com.apple.WebKit.WebContent" || process.exec_name.includes("Safari") || process.name.includes("Safari")) return "Safari";
    if (process.exec_name.includes("Brave Browser") || process.name.includes("Brave")) return "Brave";
    if (process.exec_name.includes("Microsoft Edge") || process.name.includes("Edge")) return "Edge";
    if (process.exec_name.includes("Arc") || process.name.includes("Arc")) return "Arc";
    if (process.exec_name.includes("firefox") || process.name.includes("firefox")) return "Firefox";
    return null;
  });

  let allBrowserTabs = $derived(
    detectedBrowser ? $browserTabs.filter((t) => t.browser === detectedBrowser) : [],
  );

  let filteredTabs = $derived.by(() => {
    const q = tabFilter.trim().toLowerCase();
    if (!q) return allBrowserTabs;
    return allBrowserTabs.filter(
      (t) =>
        t.title.toLowerCase().includes(q) ||
        t.url.toLowerCase().includes(q) ||
        getDomain(t.url).toLowerCase().includes(q),
    );
  });

  let selectedCount = $derived(selectedTabIds.size);

  function ramColor(mb: number): string {
    if (mb >= 1024) return "var(--danger)";
    if (mb >= 256) return "var(--yellow)";
    return "var(--fg)";
  }

  function cpuColor(pct: number): string {
    if (pct >= 50) return "var(--danger)";
    if (pct >= 10) return "var(--yellow)";
    return "var(--fg)";
  }

  function getDomain(url: string): string {
    try { return new URL(url).hostname; } catch { return ""; }
  }

  function toggleTab(tabId: string) {
    const next = new Set(selectedTabIds);
    if (next.has(tabId)) next.delete(tabId);
    else next.add(tabId);
    selectedTabIds = next;
  }

  function selectAllTabs() {
    selectedTabIds = new Set(filteredTabs.map((t) => t.id));
  }

  function selectNoneTabs() {
    selectedTabIds = new Set();
  }

  async function closeTab(tab: BrowserTab) {
    closingTabs = new Set([...closingTabs, tab.id]);
    try {
      await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
      browserTabs.update(($tabs) => $tabs.filter((t) => t.id !== tab.id));
      const next = new Set(selectedTabIds);
      next.delete(tab.id);
      selectedTabIds = next;
    } catch (e) {
      console.error("Failed to close tab:", e);
    }
    const after = new Set(closingTabs);
    after.delete(tab.id);
    closingTabs = after;
  }

  async function closeSelectedTabs() {
    const toClose = allBrowserTabs.filter((t) => selectedTabIds.has(t.id));
    for (const tab of toClose) {
      closingTabs = new Set([...closingTabs, tab.id]);
      try {
        await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
        browserTabs.update(($tabs) => $tabs.filter((t) => t.id !== tab.id));
        const next = new Set(selectedTabIds);
        next.delete(tab.id);
        selectedTabIds = next;
      } catch {
        // continue closing others
      }
      const after = new Set(closingTabs);
      after.delete(tab.id);
      closingTabs = after;
    }
  }

  async function focusTab(tab: BrowserTab) {
    try {
      await ipcFocusBrowserTab(tab.id, tab.url, tab.browser);
    } catch (e) {
      console.error("Failed to focus tab:", e);
    }
  }

  function buildAiContext(): string {
    const lines = [
      `Process: ${process.name}`,
      `Executable: ${process.exec_name}`,
      `PID: ${process.pid}`,
      `RAM: ${process.ram_mb.toFixed(1)} MB`,
      `CPU: ${process.cpu_pct.toFixed(1)}%`,
      `Uptime: ${process.uptime}`,
      `Group: ${process.group || "none"}`,
      `State: ${process.state}`,
      `System process: ${process.is_system ? "Yes" : "No"}`,
    ];
    if (allBrowserTabs.length > 0) {
      lines.push(`\nBrowser: ${detectedBrowser}`);
      lines.push(`Open tabs (${allBrowserTabs.length}):`);
      for (const tab of allBrowserTabs) {
        lines.push(`  - ${tab.title || "(Untitled)"} | ${getDomain(tab.url)} | ${tab.url}`);
      }
    }
    lines.push("\nPlease analyze this process: What is it doing? Is the memory/CPU usage normal? Are any tabs particularly heavy or suspicious? Any recommendations?");
    return lines.join("\n");
  }

  async function askAi() {
    aiAnalyzing = true;
    aiError = null;
    aiResponse = "";
    try {
      const context = buildAiContext();
      const config = $aiProviderConfig;
      aiResponse = await ipcAnalyzeContext(context, config.provider, config.model);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes("No matching entry") || msg.includes("not found in secure storage") || msg.includes("keyring")) {
        aiError = t("processes.noApiKey");
      } else {
        aiError = msg;
      }
    } finally {
      aiAnalyzing = false;
    }
  }

  onMount(() => {
    modalEl?.focus();
  });

  function handleBackdropKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    if (e.key === "Tab" && modalEl) {
      const focusable = modalEl.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="backdrop"
  onclick={onclose}
  onkeydown={handleBackdropKeydown}
  role="presentation"
>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="modal"
    bind:this={modalEl}
    onclick={(e: MouseEvent) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
    tabindex="-1"
  >
    <div class="header">
      <h2 class="title" id="modal-title">{process.name}</h2>
      <span class="pid">{t("process.pidLabel", { pid: process.pid })}</span>
      <button class="close-btn" onclick={onclose} aria-label={t("common.close")}>&times;</button>
    </div>
    <div class="body">
      <div class="section-label">{t("process.section")}</div>
      <div class="row">
        <span class="label">{t("process.name")}</span>
        <span class="value mono">{process.name}</span>
      </div>
      <div class="row">
        <span class="label">{t("process.executable")}</span>
        <span class="value mono">{process.exec_name}</span>
      </div>
      <div class="row">
        <span class="label">{t("process.pid")}</span>
        <span class="value mono">{process.pid}</span>
      </div>
      <div class="row">
        <span class="label">{t("process.group")}</span>
        <span class="value">{process.group || "\u2014"}</span>
      </div>
      <div class="row">
        <span class="label">{t("process.state")}</span>
        <span class="value mono">{process.state}</span>
      </div>
      <div class="row">
        <span class="label">{t("process.system")}</span>
        <span class="value">{process.is_system ? t("common.yes") : t("common.no")}</span>
      </div>

      <div class="section-divider"></div>
      <div class="section-label">{t("process.resources")}</div>
      <div class="row">
        <span class="label">{t("process.ram")}</span>
        <span class="value mono" style="color: {ramColor(process.ram_mb)}">{process.ram_mb.toFixed(1)} MB</span>
      </div>
      <div class="row">
        <span class="label">{t("process.cpu")}</span>
        <span class="value mono" style="color: {cpuColor(process.cpu_pct)}">{process.cpu_pct.toFixed(1)}%</span>
      </div>
      <div class="row">
        <span class="label">{t("process.uptime")}</span>
        <span class="value mono">{process.uptime || "\u2014"}</span>
      </div>

      {#if allBrowserTabs.length > 0}
        <div class="section-divider"></div>
        <div class="tabs-header">
          <div class="section-label">{t("process.browserTabs", { count: allBrowserTabs.length })}</div>
          <div class="tabs-actions">
            <button class="btn-tab-action" onclick={selectAllTabs} title={t("process.selectAllTabs")}>{t("common.all")}</button>
            <button class="btn-tab-action" onclick={selectNoneTabs} title={t("process.deselectAll")}>{t("common.none")}</button>
            {#if selectedCount > 0}
              <button class="btn-tab-close-selected" onclick={closeSelectedTabs} title={t("process.closeCountTitle", { count: selectedCount })}>
                {t("process.closeCount", { count: selectedCount })}
              </button>
            {/if}
          </div>
        </div>
        <div class="tab-filter-row">
          <input
            class="tab-filter"
            type="text"
            placeholder={t("process.filterTabs")}
            value={tabFilter}
            oninput={(e) => tabFilter = (e.target as HTMLInputElement).value}
            aria-label={t("process.filterTabsLabel")}
          />
        </div>
        {#if tabFilter && filteredTabs.length < allBrowserTabs.length}
          <div class="tab-filter-info">{t("process.tabsFiltered", { count: filteredTabs.length, total: allBrowserTabs.length })}</div>
        {/if}
        <div class="tab-list">
          {#each filteredTabs as tab (tab.id)}
            <div
              class="tab-item"
              class:closing={closingTabs.has(tab.id)}
              class:selected={selectedTabIds.has(tab.id)}
            >
              <input
                type="checkbox"
                checked={selectedTabIds.has(tab.id)}
                aria-label={t("process.selectTab", { title: tab.title })}
                onclick={() => toggleTab(tab.id)}
              />
              <button
                class="tab-title-btn"
                onclick={() => focusTab(tab)}
                title={t("process.goToTab", { title: tab.title, url: tab.url })}
              >
                {tab.title || t("common.untitled")}
              </button>
              <span class="tab-domain" title={tab.url}>{getDomain(tab.url)}</span>
              <button
                class="btn-tab-kill"
                onclick={() => closeTab(tab)}
                disabled={closingTabs.has(tab.id)}
                title={t("process.closeThisTab")}
              >
                &#10005;
              </button>
            </div>
          {/each}
          {#if filteredTabs.length === 0}
            <div class="tab-empty">{t("common.noMatchingTabs")}</div>
          {/if}
        </div>
      {/if}

      <div class="section-divider"></div>
      <div class="ai-section">
        <div class="ai-header-row">
          <div class="section-label">{t("process.aiAnalysis")}</div>
          <button
            class="btn-ask-ai"
            onclick={askAi}
            disabled={aiAnalyzing}
          >
            {aiAnalyzing ? t("process.analyzing") : t("process.askAi")}
          </button>
        </div>
        {#if aiError}
          <div class="ai-error">{aiError}</div>
        {/if}
        {#if aiResponse}
          <div class="ai-response">{aiResponse}</div>
        {:else if !aiAnalyzing && !aiError}
          <div class="ai-hint">{allBrowserTabs.length > 0 ? t("process.aiHintWithTabs") : t("process.aiHint")}</div>
        {/if}
      </div>
    </div>
    <div class="footer">
      <span class="hint">{t("process.escToClose")}</span>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-alt);
    border: 1px solid var(--border);
    border-radius: 6px;
    width: 480px;
    max-height: 85vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    font-weight: 700;
    font-size: var(--base-font-size);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
  }

  .pid {
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    flex-shrink: 0;
  }

  .close-btn {
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 1.333);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    line-height: 1;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .body {
    padding: 6px 0;
  }

  .section-label {
    padding: 4px 10px 2px;
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
  }

  .row {
    display: flex;
    align-items: baseline;
    padding: 3px 10px;
    font-size: calc(var(--base-font-size) * 0.917);
    gap: 8px;
  }
  .row:hover {
    background: var(--bg-hover);
  }

  .label {
    width: 72px;
    flex-shrink: 0;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.833);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .value {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    word-break: break-all;
  }

  .mono {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.917);
    font-variant-numeric: tabular-nums;
  }

  .section-divider {
    height: 1px;
    background: var(--border);
    margin: 4px 10px;
  }

  /* --- Tab section --- */
  .tabs-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 10px;
  }

  .tabs-actions {
    display: flex;
    gap: 3px;
  }

  .btn-tab-action {
    padding: 1px 5px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 600;
    cursor: pointer;
  }
  .btn-tab-action:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .btn-tab-close-selected {
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
  .btn-tab-close-selected:hover {
    background: #b71c1c;
  }

  .tab-filter-row {
    padding: 2px 10px;
  }

  .tab-filter {
    width: 100%;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    outline: none;
    height: calc(var(--base-font-size) * 1.667);
  }
  .tab-filter:focus {
    border-color: var(--accent);
  }

  .tab-filter-info {
    padding: 0 10px;
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
  }

  .tab-list {
    max-height: 200px;
    overflow-y: auto;
    margin: 2px 0;
  }

  .tab-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
  }
  .tab-item:hover {
    background: var(--bg-hover);
  }
  .tab-item.selected {
    background: var(--bg-selected);
  }
  .tab-item.closing {
    opacity: 0.4;
    pointer-events: none;
  }

  .tab-item input[type="checkbox"] {
    margin: 0;
    cursor: pointer;
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .tab-title-btn {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: none;
    border: none;
    padding: 0;
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    cursor: pointer;
    text-align: left;
  }
  .tab-title-btn:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .tab-domain {
    flex-shrink: 0;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--fg-dim);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.75);
  }

  .btn-tab-kill {
    width: 18px;
    height: 18px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.833);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .btn-tab-kill:hover {
    background: rgba(211, 47, 47, 0.15);
    color: var(--danger);
    border-color: var(--danger);
  }

  .tab-empty {
    padding: 4px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    font-style: italic;
  }

  /* --- AI section --- */
  .ai-section {
    padding: 0 0 4px;
  }

  .ai-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 10px;
  }

  .btn-ask-ai {
    padding: 2px 8px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    background: var(--accent);
    color: white;
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 600;
    cursor: pointer;
  }
  .btn-ask-ai:hover:not(:disabled) {
    background: #005fa3;
  }
  .btn-ask-ai:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .ai-error {
    padding: 4px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--danger);
  }

  .ai-response {
    padding: 6px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
    line-height: 1.5;
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
    background: var(--bg);
    margin: 4px 10px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  .ai-hint {
    padding: 4px 10px;
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
    font-style: italic;
  }

  .footer {
    padding: 4px 10px;
    border-top: 1px solid var(--border);
    text-align: right;
  }

  .hint {
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
  }
</style>
