<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";
  import Skeleton from "./Skeleton.svelte";
  import type { ProcessEntry, BrowserTab } from "../lib/types";
  import { ipcCloseBrowserTab, ipcFocusBrowserTab } from "../lib/ipc";
  import { browserTabs } from "../stores/processes";
  import { t } from "../lib/i18n";
  import { formatProcessState, formatProcessUptime } from "../lib/localizedUi";
  import { detectBrowser } from "../lib/browser";
  import { ipcAnalyzeContext } from "../lib/ipc";
  import { aiProviderConfig, userMode } from "../stores/preferences";
  import { focusFirstFocusable, trapFocus } from "../lib/focusTrap";
  import Button from "./Button.svelte";
  import ModalShell from "./ModalShell.svelte";

  interface Props {
    process: ProcessEntry;
    onclose: () => void;
  }

  let { process, onclose }: Props = $props();
  let modalEl: HTMLDivElement | undefined = $state();

  function closeWhenBackdropMatches(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }

  function stopMouseEventPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  // Tab management state
  let selectedTabIds = $state<Set<string>>(new Set());
  let closingTabs = $state<Set<string>>(new Set());
  let tabFilter = $state("");
  let aiResponse = $state("");
  let aiAnalyzing = $state(false);
  let aiError = $state<string | null>(null);
  let proMode = $derived($userMode === "pro");

  let detectedBrowser = $derived(detectBrowser(process));

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

  function buildAiContext(question: string): string {
    const context: Record<string, unknown> = {
      process: {
        name: process.name,
        executable: process.exec_name,
        pid: process.pid,
        ram_mb: parseFloat(process.ram_mb.toFixed(1)),
        cpu_pct: parseFloat(process.cpu_pct.toFixed(1)),
        uptime: process.uptime,
        group: process.group || null,
        state: process.state,
        is_system: process.is_system,
      },
      prompt: question,
    };
    if (allBrowserTabs.length > 0) {
      context.browser = {
        name: detectedBrowser,
        tab_count: allBrowserTabs.length,
        tabs: allBrowserTabs.map((tab) => ({
          title: tab.title || t("common.untitled"),
          domain: getDomain(tab.url),
          url: tab.url,
        })),
      };
    }
    return JSON.stringify(context);
  }

  async function askAi() {
    aiAnalyzing = true;
    aiError = null;
    aiResponse = "";
    try {
      const context = buildAiContext(
        "Analyze this process: What is it doing? Is the memory/CPU usage normal? Are any tabs particularly heavy or suspicious? Any recommendations?"
      );
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
    requestAnimationFrame(() => {
      const focusables = modalEl ? Array.from(modalEl.querySelectorAll<HTMLElement>('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')) : [];
      const preferredCloseButton = focusables.find((element) => element.getAttribute("aria-label") === t("common.close"));
      if (preferredCloseButton) {
        preferredCloseButton.focus();
        return;
      }
      focusFirstFocusable(modalEl);
    });
  });

  function handleBackdropKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onclose();
      return;
    }
    trapFocus(e, modalEl);
  }
</script>

<div transition:fade={fadeConfig}>
  <ModalShell titleId="modal-title" backdropClass="backdrop" panelClass="modal" onclose={onclose} width="500px" maxHeight="85vh">
  <div
    bind:this={modalEl}
    onkeydown={handleBackdropKeydown}
    role="document"
    transition:scale={scaleConfig}
  >
    <div class="header">
      <h2 class="title" id="modal-title">{process.name}</h2>
      <span class="pid">{t("process.pidLabel", { pid: process.pid })}</span>
      <Button class="close-btn" variant="ghost" size="icon" onclick={onclose} aria-label={t("common.close")} title={t("common.close")} tabindex="-1">×</Button>
    </div>
    <div class="body">
      <div class="mode-banner" transition:fade={{ duration: 180 }}>
        {$userMode === "basic" ? t("processView.basicHint") : t("processView.advancedHint")}
      </div>
      <div class="section-label">{t("process.section")}</div>
      <div class="row">
        <span class="label">{t("process.name")}</span>
        <span class="value mono">{process.name}</span>
      </div>
      {#if proMode}
        <div class="row" transition:fly={{ x: -8, duration: 160 }}>
          <span class="label">{t("process.executable")}</span>
          <span class="value mono">{process.exec_name}</span>
        </div>
      {/if}
      <div class="row">
        <span class="label">{t("process.pid")}</span>
        <span class="value mono">{process.pid}</span>
      </div>
      {#if proMode}
        <div class="row" transition:fly={{ x: -8, duration: 160 }}>
          <span class="label">{t("process.group")}</span>
          <span class="value">{process.group || "\u2014"}</span>
        </div>
        <div class="row" transition:fly={{ x: -8, duration: 160 }}>
          <span class="label">{t("process.state")}</span>
          <span class="value mono">{formatProcessState(process.state)}</span>
        </div>
      {/if}
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
          <span class="value mono">{formatProcessUptime(process.uptime)}</span>
      </div>

      {#if allBrowserTabs.length > 0}
        <div class="section-divider"></div>
        <div class="tabs-header">
          <div class="section-label">{t("process.browserTabs", { count: allBrowserTabs.length })}</div>
          <div class="tabs-actions">
            <Button variant="secondary" size="sm" class="btn-tab-action" onclick={selectAllTabs} title={t("process.selectAllTabs")}>{t("common.all")}</Button>
            <Button variant="ghost" size="sm" class="btn-tab-action" onclick={selectNoneTabs} title={t("process.deselectAll")}>{t("common.none")}</Button>
            {#if selectedCount > 0}
              <Button variant="danger" size="sm" class="btn-tab-close-selected" onclick={closeSelectedTabs} title={t("process.closeCountTitle", { count: selectedCount })}>
                {t("process.closeCount", { count: selectedCount })}
              </Button>
            {/if}
          </div>
        </div>
        <div class="tab-filter-row">
          <input
            class="tab-filter"
            type="text"
            placeholder={t("process.filterTabs")}
            value={tabFilter}
            oninput={(e: Event) => tabFilter = (e.target as HTMLInputElement).value}
            aria-label={t("process.filterTabsLabel")}
          />
        </div>
        {#if tabFilter && filteredTabs.length < allBrowserTabs.length}
          <div class="tab-filter-info">{t("process.tabsFiltered", { count: filteredTabs.length, total: allBrowserTabs.length })}</div>
        {/if}
        {#each filteredTabs as tab, i}
          <div style="display:flex;align-items:center;gap:6px;padding:4px 14px;min-height:28px;font-size:calc(var(--base-font-size) * 0.833);color:var(--fg);border-radius:6px;margin:0 4px;{i % 2 === 0 ? 'background:var(--bg-alt);' : ''}{selectedTabIds.has(tab.id) ? 'background:var(--bg-selected);' : ''}{closingTabs.has(tab.id) ? 'color:var(--fg-muted);pointer-events:none;' : ''}">
            <input
              type="checkbox"
              checked={selectedTabIds.has(tab.id)}
              aria-label={t("process.selectTab", { title: tab.title })}
              onclick={() => toggleTab(tab.id)}
              style="margin:0;cursor:pointer;width:13px;height:13px;flex-shrink:0;accent-color:var(--accent);"
            />
            <span
              onclick={() => focusTab(tab)}
              role="button"
              tabindex="0"
              title={t("process.goToTab", { title: tab.title, url: tab.url })}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); focusTab(tab); } }}
              style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;cursor:pointer;color:var(--fg);line-height:1.4;"
            >
              {tab.title || t("common.untitled")}
            </span>
            {#if tab.url}
              <span title={tab.url} style="flex-shrink:0;max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:'SF Mono','Menlo','Consolas',monospace;font-size:calc(var(--base-font-size) * 0.7);color:var(--fg-muted);">{getDomain(tab.url)}</span>
            {/if}
            <button
              type="button"
              onclick={() => closeTab(tab)}
              disabled={closingTabs.has(tab.id)}
              title={t("process.closeThisTab")}
              aria-label={t("processView.closeTabLabel", { title: tab.title || t("common.untitled") })}
              style="appearance:none;background:none;border:none;width:20px;min-width:20px;height:20px;flex-shrink:0;display:flex;align-items:center;justify-content:center;color:var(--fg-muted);cursor:pointer;font-size:calc(var(--base-font-size) * 0.75);padding:0;border-radius:6px;"
            >&#10005;</button>
          </div>
        {/each}
        {#if filteredTabs.length === 0}
          <div class="tab-empty">{t("common.noMatchingTabs")}</div>
        {/if}
      {/if}

      <div class="section-divider"></div>
      <div class="ai-section">
        <div class="ai-header-row">
          <div class="section-label">{t("process.aiAnalysis")}</div>
          <Button class="btn-ask-ai" variant="primary" onclick={askAi} disabled={aiAnalyzing} aria-busy={aiAnalyzing} aria-label={aiAnalyzing ? t("processView.analyzingAria") : t("process.askAi")}>
            {aiAnalyzing ? t("process.analyzing") : t("process.askAi")}
          </Button>
        </div>
        {#if aiError}
          <div class="ai-error">{aiError}</div>
        {/if}
        {#if aiAnalyzing}
          <div class="ai-skeleton" role="status" aria-label={t("processView.analyzingAria")}>
            <Skeleton width="38%" height="12px" borderRadius="999px" />
            <Skeleton width="100%" height="12px" borderRadius="999px" />
            <Skeleton width="92%" height="12px" borderRadius="999px" />
            <Skeleton width="72%" height="12px" borderRadius="999px" />
          </div>
        {:else if aiResponse}
          <div class="ai-response">{aiResponse}</div>
        {:else if !aiAnalyzing && !aiError}
          <div class="ai-hint">{allBrowserTabs.length > 0 ? t("process.aiHintWithTabs") : t("process.aiHint")}</div>
        {/if}
      </div>
    </div>
    <div class="footer">
      <Button variant="secondary" size="sm" onclick={onclose}>{t("common.close")}</Button>
      <button class="focus-sentinel" type="button" tabindex="0" aria-hidden="true"></button>
    </div>
  </div>
  </ModalShell>
</div>

<style>
  /* ── Backdrop ── */
  /* ── Header ── */
  .header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-alt) 90%, var(--accent) 6%);
  }

  .title {
    font-weight: 700;
    font-size: calc(var(--base-font-size) * 1.08);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
    color: var(--fg);
  }

  .pid {
    color: var(--fg-muted);
    font-size: calc(var(--base-font-size) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    flex-shrink: 0;
  }

  .close-btn {
    font-size: calc(var(--base-font-size) * 1.2);
    line-height: 1;
  }

  /* ── Body ── */
  .body {
    padding: 8px 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    min-height: 0;
  }

  .mode-banner {
    margin: 4px 14px 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md, 10px);
    border: 1px solid var(--border-subtle, var(--border));
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.78);
    line-height: 1.5;
  }

  /* ── Section labels — subtle, not screaming ── */
  .section-label {
    padding: 6px 14px 3px;
    font-size: calc(var(--base-font-size) * 0.7);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--fg-dim);
  }

  /* ── Data rows ── */
  .row {
    display: flex;
    align-items: baseline;
    padding: 4px 14px;
    font-size: calc(var(--base-font-size) * 0.917);
    gap: 10px;
    border-radius: 4px;
    margin: 0 4px;
    transition: background 0.12s ease;
  }
  .row:hover {
    background: var(--bg-hover);
  }

  .label {
    width: 76px;
    flex-shrink: 0;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.8);
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
    color: var(--fg);
  }

  .mono {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.917);
    font-variant-numeric: tabular-nums;
  }

  .section-divider {
    height: 1px;
    background: var(--border-subtle, var(--border));
    margin: 8px 14px;
  }

  /* ── Tab section ── */
  .tabs-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 14px;
  }

  .tabs-actions {
    display: flex;
    gap: 6px;
  }

  :global(.btn-tab-action) {
    min-height: 26px;
    font-size: calc(var(--base-font-size) * 0.75) !important;
  }

  :global(.btn-tab-close-selected) {
    letter-spacing: 0.3px;
    min-height: 26px;
    font-size: calc(var(--base-font-size) * 0.75) !important;
  }

  .tab-filter-row {
    padding: 4px 14px;
  }

  .tab-filter {
    width: 100%;
    padding: 5px 8px;
    border: 1px solid var(--border-subtle, var(--border));
    border-radius: var(--radius-sm, 6px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    outline: none;
    transition: border-color 0.15s ease;
  }
  .tab-filter:focus {
    border-color: var(--accent);
  }

  .tab-filter-info {
    padding: 2px 14px;
    font-size: calc(var(--base-font-size) * 0.7);
    color: var(--fg-dim);
  }

  .tab-empty {
    padding: 8px 14px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    font-style: italic;
  }

  /* ── AI section ── */
  .ai-section {
    padding: 0 0 4px;
  }

  .ai-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 14px;
  }

  :global(.btn-ask-ai) {
    min-height: 30px;
    font-size: calc(var(--base-font-size) * 0.833) !important;
  }

  .ai-error {
    padding: 6px 14px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--danger);
  }

  .ai-response {
    padding: 8px 12px;
    font-size: calc(var(--base-font-size) * 0.833);
    line-height: 1.55;
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow-y: auto;
    background: var(--bg);
    margin: 6px 14px;
    border-radius: var(--radius-md, 10px);
    border: 1px solid var(--border-subtle, var(--border));
  }

  .ai-skeleton {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    margin: 6px 14px;
    border-radius: var(--radius-md, 10px);
    border: 1px solid var(--border-subtle, var(--border));
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
  }

  .ai-hint {
    padding: 4px 14px;
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-muted);
    font-style: italic;
  }

  /* ── Footer ── */
  .footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-top: 1px solid var(--border-subtle, var(--border));
  }

  .focus-sentinel {
    width: 0;
    height: 0;
    opacity: 0;
    padding: 0;
    border: 0;
    pointer-events: none;
  }

</style>
