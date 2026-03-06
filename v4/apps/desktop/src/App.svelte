<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import ProcessTable from "./components/ProcessTable.svelte";
  import ChromeTabManager from "./components/ChromeTabManager.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import ProcessDetailsModal from "./components/ProcessDetailsModal.svelte";
  import type { ProcessEntry } from "./lib/types";
  import { AI_PROVIDERS, type AiProviderKind } from "./lib/types";
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
    killSingle,
    selectAllVisible,
    selectNone,
    aiSuggestions,
    aiLoading,
    aiError,
    aiProfile,
    analyzeWithAi,
    saveAiConfigAction,
    dismissAiSuggestions,
  } from "./stores/processes";
  import {
    fontSize,
    columns,
    columnOrder,
    aiProviderConfig,
    idleThreshold,
    theme,
    tabPanelHeight as tabPanelHeightStore,
    localePreference,
    loadPreferences,
    initPreferenceSubscriptions,
    increaseFontSize,
    decreaseFontSize,
    moveColumnUp,
    moveColumnDown,
    MIN_IDLE_THRESHOLD,
    MAX_IDLE_THRESHOLD,
  } from "./stores/preferences";
  import type { ThemeMode } from "./stores/preferences";
  import { ipcValidateApiKey } from "./lib/ipc";
  import { t, locale, initI18n } from "./lib/i18n";
  import type { LocaleCode } from "./lib/i18n";

  let detailProcess: ProcessEntry | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let searchValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  // Resizable tab panel (backed by store for persistence)
  let tabPanelHeight = $state($tabPanelHeightStore);
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartHeight = 0;

  // Sync local → store when resizing
  $effect(() => {
    $tabPanelHeightStore = tabPanelHeight;
  });
  // Sync store → local on load
  $effect(() => {
    tabPanelHeight = $tabPanelHeightStore;
  });

  function onDividerMousedown(e: MouseEvent) {
    e.preventDefault();
    dragging = true;
    dragStartY = e.clientY;
    dragStartHeight = tabPanelHeight;
    window.addEventListener("mousemove", onDividerMousemove);
    window.addEventListener("mouseup", onDividerMouseup);
  }

  function onDividerMousemove(e: MouseEvent) {
    const delta = e.clientY - dragStartY;
    tabPanelHeight = Math.max(40, Math.min(dragStartHeight + delta, window.innerHeight - 200));
  }

  function onDividerMouseup() {
    dragging = false;
    window.removeEventListener("mousemove", onDividerMousemove);
    window.removeEventListener("mouseup", onDividerMouseup);
  }

  // AI settings modal state
  let showSettings = $state(false);
  let apiKeyInput = $state("");
  let settingsSaving = $state(false);
  let settingsError = $state<string | null>(null);
  let settingsSaved = $state(false);

  async function handleSaveSettings() {
    settingsSaving = true;
    settingsError = null;
    settingsSaved = false;
    try {
      // Validate key before saving
      const trimmed = apiKeyInput.trim();
      if (!trimmed) {
        settingsError = t("settings.apiKeyEmpty");
        return;
      }
      const valid = await ipcValidateApiKey($aiProviderConfig.provider, trimmed);
      if (!valid) {
        settingsError = t("settings.apiKeyFailed");
        return;
      }
      await saveAiConfigAction($aiProviderConfig.provider, $aiProviderConfig.model, trimmed);
      settingsSaved = true;
      apiKeyInput = "";
    } catch (e) {
      settingsError = e instanceof Error ? e.message : String(e);
    } finally {
      settingsSaving = false;
    }
  }

  // Apply theme to document
  function applyTheme(mode: ThemeMode) {
    if (typeof document === "undefined") return;
    const html = document.documentElement;
    if (mode === "auto") {
      html.removeAttribute("data-theme");
    } else {
      html.setAttribute("data-theme", mode);
    }
  }

  $effect(() => {
    applyTheme($theme);
  });

  function closeSettings() {
    showSettings = false;
    settingsError = null;
    settingsSaved = false;
  }

  function onSearchInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    searchValue = val;
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      $search = val;
    }, 150);
  }

  onMount(() => {
    loadPreferences().then(() => {
      initI18n($localePreference);
      startPolling(2000);
    });
    const unsubPrefs = initPreferenceSubscriptions();
    const unsubLocale = localePreference.subscribe((val) => {
      locale.set(val);
    });
    return () => {
      stopPolling();
      clearTimeout(debounceTimer);
      unsubPrefs();
      unsubLocale();
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

    // Cmd/Ctrl+= → zoom in, Cmd/Ctrl+- → zoom out
    if (mod && (e.key === "=" || e.key === "+")) {
      e.preventDefault();
      increaseFontSize();
      return;
    }
    if (mod && e.key === "-") {
      e.preventDefault();
      decreaseFontSize();
      return;
    }

    // Escape → close modal/settings or blur search
    if (e.key === "Escape") {
      if (showSettings) {
        closeSettings();
        return;
      }
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

<main style="--base-font-size: {$fontSize}px">
  <header class="toolbar">
    <div class="search-wrapper">
      <input
        class="search"
        type="text"
        placeholder={t("toolbar.searchPlaceholder")}
        aria-label={t("toolbar.searchLabel")}
        value={searchValue}
        oninput={onSearchInput}
        bind:this={searchInput}
      />
      {#if searchValue}
        <button
          class="search-clear"
          onclick={() => { searchValue = ""; $search = ""; }}
          aria-label={t("toolbar.clearSearch")}
        >&times;</button>
      {/if}
    </div>
    <div class="actions">
      <button
        class="btn btn-sm"
        class:active={$grouping}
        onclick={() => $grouping = !$grouping}
        title={t("toolbar.toggleGrouping")}
      >
        {t("toolbar.groups")}
      </button>
      <button class="btn btn-sm" onclick={selectAllVisible} aria-label={t("toolbar.selectAll")}>{t("toolbar.all")}</button>
      <button class="btn btn-sm" onclick={selectNone} aria-label={t("toolbar.deselectAll")}>{t("toolbar.none")}</button>
      <button
        class="btn btn-kill"
        onclick={killSelected}
        disabled={$selectedCount === 0}
        aria-label={t("toolbar.closeSelected")}
      >
        {t("toolbar.close")}{#if $selectedCount > 0}
          &nbsp;({$selectedCount} &middot; {$selectedRamMB.toFixed(0)} MB){/if}
      </button>
      <span class="separator"></span>
      <select
        class="profile-select"
        value={$aiProfile}
        onchange={(e) => $aiProfile = (e.target as HTMLSelectElement).value}
        aria-label={t("toolbar.aiProfile")}
      >
        <option value="general">{t("toolbar.general")}</option>
        <option value="developer">{t("toolbar.developer")}</option>
        <option value="gaming">{t("toolbar.gaming")}</option>
        <option value="battery">{t("toolbar.batterySaver")}</option>
      </select>
      <button
        class="btn btn-ai"
        onclick={() => analyzeWithAi($aiProviderConfig.provider, $aiProviderConfig.model)}
        disabled={$aiLoading}
      >
        {$aiLoading ? t("toolbar.analyzing") : t("toolbar.aiAnalyze")}
      </button>
      <button
        class="btn btn-sm"
        onclick={() => showSettings = true}
        title={t("toolbar.aiSettings")}
      >
        {t("toolbar.settings")}
      </button>
      <span class="separator"></span>
      <button
        class="btn btn-sm"
        onclick={decreaseFontSize}
        title={t("toolbar.decreaseFont")}
        aria-label={t("toolbar.decreaseFontLabel")}
      >A-</button>
      <span class="font-size-display">{$fontSize}</span>
      <button
        class="btn btn-sm"
        onclick={increaseFontSize}
        title={t("toolbar.increaseFont")}
        aria-label={t("toolbar.increaseFontLabel")}
      >A+</button>
    </div>
  </header>

  <StatusBar />
  <div class="tab-panel" style="height: {tabPanelHeight}px">
    <ChromeTabManager filter={searchValue} />
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="resize-divider"
    class:active={dragging}
    onmousedown={onDividerMousedown}
    role="separator"
    aria-orientation="horizontal"
    aria-label={t("common.resizeTabPanel")}
    tabindex="-1"
  ></div>

  {#if $loading}
    <div class="loading" role="status" aria-busy="true">{t("common.loading")}</div>
  {:else}
    <ProcessTable
      processes={$filtered}
      grouping={$grouping}
      columns={$columns}
      columnOrder={$columnOrder}
      oninspect={inspectProcess}
    />
  {/if}

  {#if $aiError || $aiSuggestions.length > 0}
    <div class="ai-panel" role="region" aria-label={t("ai.suggestions")}>
      <div class="ai-header">
        <span class="ai-title">{t("ai.suggestions")}</span>
        <button class="btn btn-sm" onclick={dismissAiSuggestions}>{t("ai.dismiss")}</button>
      </div>
      {#if $aiError}
        <div class="ai-error">{$aiError}</div>
      {/if}
      {#each $aiSuggestions as suggestion (suggestion.pid)}
        <div class="ai-row">
          <span class="ai-name">{suggestion.name}</span>
          <span class="ai-pid">{t("ai.pid", { pid: suggestion.pid })}</span>
          <span class="ai-reason">{suggestion.reason}</span>
          <button
            class="btn btn-kill btn-sm"
            onclick={() => killSingle(suggestion.pid)}
          >{t("ai.close")}</button>
        </div>
      {/each}
    </div>
  {/if}

  <footer class="statusline" aria-live="polite" aria-atomic="true">
    {t("footer.processes", { count: $filtered.length })}{#if $filtered.length !== $processes.length}
      &nbsp;{t("footer.filteredFrom", { count: $processes.length })}{/if}
    {#if $selectedCount > 0}
      <span aria-hidden="true">&nbsp;&middot;&nbsp;</span>{t("footer.selected", { count: $selectedCount, ram: $selectedRamMB.toFixed(0) })}
    {/if}
    <span class="shortcuts" aria-hidden="true"><kbd>Cmd+I</kbd> {t("footer.shortcutDetail")} <kbd>Cmd+F</kbd> {t("footer.shortcutSearch")} <kbd>Del</kbd> {t("footer.shortcutClose")}</span>
  </footer>
</main>

{#if detailProcess}
  <ProcessDetailsModal process={detailProcess} onclose={closeDetail} />
{/if}

{#if showSettings}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="backdrop" onclick={closeSettings} onkeydown={(e) => { if (e.key === "Escape") closeSettings(); }} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="settings-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1">
      <div class="settings-header">
        <h2 class="settings-title" id="settings-title">{t("settings.title")}</h2>
        <button class="close-btn" onclick={closeSettings} aria-label={t("settings.closeSettings")}>&times;</button>
      </div>
      <div class="settings-body">
        <div class="settings-row">
          <label class="settings-label" for="provider-select">{t("settings.provider")}</label>
          <select
            id="provider-select"
            class="settings-select"
            value={$aiProviderConfig.provider}
            onchange={(e) => {
              const newProvider = (e.target as HTMLSelectElement).value as AiProviderKind;
              aiProviderConfig.update((c) => ({ ...c, provider: newProvider }));
            }}
          >
            {#each AI_PROVIDERS as p}
              <option value={p.id}>{p.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="model-input">{t("settings.model")}</label>
          <input
            id="model-input"
            class="settings-input"
            type="text"
            placeholder={t("settings.modelPlaceholder")}
            value={$aiProviderConfig.model}
            oninput={(e) => {
              aiProviderConfig.update((c) => ({ ...c, model: (e.target as HTMLInputElement).value }));
            }}
          />
        </div>
        <div class="settings-row">
          <label class="settings-label" for="api-key-input">{t("settings.apiKey")}</label>
          <input
            id="api-key-input"
            class="settings-input"
            type="password"
            placeholder={t("settings.apiKeyPlaceholder", { provider: AI_PROVIDERS.find((p) => p.id === $aiProviderConfig.provider)?.label ?? '' })}
            bind:value={apiKeyInput}
          />
        </div>
        {#if settingsError}
          <div class="settings-error">{settingsError}</div>
        {/if}
        {#if settingsSaved}
          <div class="settings-success">{t("settings.apiKeySaved")}</div>
        {/if}

        <div class="settings-divider"></div>
        <div class="settings-section-label">{t("settings.columns")}</div>
        <div class="settings-columns-list">
          {#each $columnOrder as key, i (key)}
            <div class="col-order-row">
              <input
                type="checkbox"
                checked={$columns[key]}
                onchange={() => columns.update((c) => ({ ...c, [key]: !c[key as keyof typeof c] }))}
              />
              <span class="col-order-name">{key.charAt(0).toUpperCase() + key.slice(1)}</span>
              <div class="col-order-btns">
                <button
                  class="col-move-btn"
                  disabled={i === 0}
                  onclick={() => moveColumnUp(key)}
                  title={t("settings.moveUp")}
                  aria-label={t("settings.moveColumnUp", { column: key })}
                >&#9650;</button>
                <button
                  class="col-move-btn"
                  disabled={i === $columnOrder.length - 1}
                  onclick={() => moveColumnDown(key)}
                  title={t("settings.moveDown")}
                  aria-label={t("settings.moveColumnDown", { column: key })}
                >&#9660;</button>
              </div>
            </div>
          {/each}
        </div>

        <div class="settings-divider"></div>
        <div class="settings-section-label">{t("settings.appearance")}</div>
        <div class="settings-row">
          <label class="settings-label" for="theme-select">{t("settings.theme")}</label>
          <select
            id="theme-select"
            class="settings-select"
            value={$theme}
            onchange={(e) => { $theme = (e.target as HTMLSelectElement).value as ThemeMode; }}
          >
            <option value="auto">{t("settings.themeAuto")}</option>
            <option value="light">{t("settings.themeLight")}</option>
            <option value="dark">{t("settings.themeDark")}</option>
          </select>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="locale-select">{t("settings.language")}</label>
          <select
            id="locale-select"
            class="settings-select"
            value={$localePreference}
            onchange={(e) => { $localePreference = (e.target as HTMLSelectElement).value as LocaleCode; }}
          >
            <option value="auto">{t("settings.langAuto")}</option>
            <option value="en">{t("settings.langEn")}</option>
            <option value="es">{t("settings.langEs")}</option>
          </select>
        </div>

        <div class="settings-divider"></div>
        <div class="settings-section-label">{t("settings.performance")}</div>
        <div class="settings-row">
          <label class="settings-label" for="idle-threshold">{t("settings.idleCpu")}</label>
          <input
            id="idle-threshold"
            class="settings-input"
            type="number"
            step="0.1"
            min={MIN_IDLE_THRESHOLD}
            max={MAX_IDLE_THRESHOLD}
            value={$idleThreshold}
            oninput={(e) => {
              const v = parseFloat((e.target as HTMLInputElement).value);
              if (!isNaN(v) && v >= MIN_IDLE_THRESHOLD && v <= MAX_IDLE_THRESHOLD) {
                $idleThreshold = v;
              }
            }}
          />
          <span class="settings-hint">{t("settings.idleHint")}</span>
        </div>
      </div>
      <div class="settings-footer">
        <button
          class="btn btn-ai"
          onclick={handleSaveSettings}
          disabled={settingsSaving || !apiKeyInput}
        >
          {settingsSaving ? t("settings.saving") : t("settings.saveApiKey")}
        </button>
      </div>
    </div>
  </div>
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
    font-size: var(--base-font-size, 12px);
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }

  :global(:root), :global([data-theme="dark"]) {
    --bg: #1a1a1a;
    --bg-alt: #222;
    --bg-hover: #2a2a2a;
    --bg-selected: #0a3d6e;
    --fg: #ccc;
    --fg-dim: #999;
    --border: #333;
    --accent: #0078d4;
    --danger: #d32f2f;
    --green: #4caf50;
    --yellow: #ffc107;
  }

  :global([data-theme="light"]) {
    --bg: #f8f8f8;
    --bg-alt: #eee;
    --bg-hover: #e0e0e0;
    --bg-selected: #cce5ff;
    --fg: #1a1a1a;
    --fg-dim: #595959;
    --border: #d0d0d0;
  }

  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme="dark"])) {
      --bg: #f8f8f8;
      --bg-alt: #eee;
      --bg-hover: #e0e0e0;
      --bg-selected: #cce5ff;
      --fg: #1a1a1a;
      --fg-dim: #595959;
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

  .search-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .search {
    width: 100%;
    padding: 2px 22px 2px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: 20px;
  }
  .search:focus {
    border-color: var(--accent);
  }

  .search-clear {
    position: absolute;
    right: 2px;
    width: 16px;
    height: 16px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    line-height: 1;
    border-radius: 2px;
  }
  .search-clear:hover {
    color: var(--fg);
    background: var(--bg-hover);
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
    font-size: calc(var(--base-font-size) * 0.833);
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

  .tab-panel {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .resize-divider {
    flex-shrink: 0;
    height: 4px;
    background: var(--border);
    cursor: ns-resize;
    position: relative;
  }
  .resize-divider:hover,
  .resize-divider.active {
    background: var(--accent);
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.917);
  }

  .statusline {
    display: flex;
    justify-content: space-between;
    padding: 2px 8px;
    font-size: calc(var(--base-font-size) * 0.833);
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

  .separator {
    width: 1px;
    height: 14px;
    background: var(--border);
    flex-shrink: 0;
  }

  .font-size-display {
    font-size: calc(var(--base-font-size) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--fg-dim);
    min-width: 16px;
    text-align: center;
    line-height: 20px;
  }

  .profile-select {
    padding: 1px 4px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    height: 20px;
    outline: none;
    cursor: pointer;
  }

  .btn-ai {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    font-weight: 600;
  }
  .btn-ai:hover:not(:disabled) {
    background: #005fa3;
  }

  /* AI results panel */
  .ai-panel {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-alt);
    max-height: 180px;
    overflow-y: auto;
  }

  .ai-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }

  .ai-title {
    font-size: calc(var(--base-font-size) * 0.833);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--accent);
  }

  .ai-error {
    padding: 4px 8px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--danger);
  }

  .ai-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 8px;
    font-size: calc(var(--base-font-size) * 0.917);
    border-bottom: 1px solid var(--border-subtle, rgba(128, 128, 128, 0.15));
  }
  .ai-row:hover {
    background: var(--bg-hover);
  }

  .ai-name {
    font-weight: 600;
    min-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ai-pid {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    flex-shrink: 0;
  }

  .ai-reason {
    flex: 1;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Settings modal */
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .settings-modal {
    background: var(--bg-alt);
    border: 1px solid var(--border);
    border-radius: 6px;
    width: 360px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }

  .settings-title {
    font-weight: 700;
    font-size: var(--base-font-size);
    margin: 0;
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
    line-height: 1;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .settings-body {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .settings-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: calc(var(--base-font-size) * 0.917);
  }

  .settings-label {
    width: 64px;
    flex-shrink: 0;
    font-size: calc(var(--base-font-size) * 0.833);
    font-weight: 600;
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .settings-select {
    flex: 1;
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: 22px;
    cursor: pointer;
  }
  .settings-select:focus {
    border-color: var(--accent);
  }

  .settings-input {
    flex: 1;
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: 22px;
  }
  .settings-input:focus {
    border-color: var(--accent);
  }

  .settings-error {
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--danger);
    padding: 2px 0;
  }

  .settings-success {
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--green);
    padding: 2px 0;
  }

  .settings-divider {
    height: 1px;
    background: var(--border);
    margin: 6px 0;
  }

  .settings-section-label {
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    margin-bottom: 4px;
  }

  .settings-columns-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .col-order-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    font-size: calc(var(--base-font-size) * 0.917);
  }
  .col-order-row input[type="checkbox"] {
    margin: 0;
    width: 12px;
    height: 12px;
    cursor: pointer;
  }
  .col-order-name {
    flex: 1;
  }
  .col-order-btns {
    display: flex;
    gap: 2px;
  }
  .col-move-btn {
    width: 18px;
    height: 16px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 2px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.667);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-move-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .col-move-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .settings-hint {
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
    white-space: nowrap;
  }

  .settings-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
  }
</style>
