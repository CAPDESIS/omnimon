<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import ProcessTable from "./components/ProcessTable.svelte";
  import ChromeTabManager from "./components/ChromeTabManager.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import ProcessDetailsModal from "./components/ProcessDetailsModal.svelte";
  import SystemDashboard from "./components/SystemDashboard.svelte";
  import ToastContainer from "./components/ToastContainer.svelte";
  import AlertPanel from "./components/AlertPanel.svelte";
  import AiCommandBar from "./components/AiCommandBar.svelte";
  import AIChat from "./components/AIChat.svelte";
  import NetworkMap from "./components/NetworkMap.svelte";
  import SecurityReportView from "./components/SecurityReportView.svelte";
  import AiInsightCard from "./components/AiInsightCard.svelte";
  import CloudSync from "./components/CloudSync.svelte";
  import Automations from "./components/Automations.svelte";
  import { totalFindings } from "./stores/security";
  import { initSecurityAlertListener } from "./stores/alerts";
  import type { ProcessEntry } from "./lib/types";
  import { AI_PROVIDERS, type AiProviderKind } from "./lib/types";
  import { applyThemeTokens, detectPlatform, type ThemeId, type ThemeTokens } from "./lib/theme";
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
  import { customTheme, type ThemeMode } from "./stores/preferences";
  import { ipcValidateApiKey, ipcCheckApiKey, ipcAnalyzeContext } from "./lib/ipc";
  import { listen } from "@tauri-apps/api/event";
  import { t, locale, initI18n } from "./lib/i18n";
  import { inspectProcessRequest } from "./stores/uiActions";
  import type { LocaleCode } from "./lib/i18n";

  let detailProcess: ProcessEntry | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let searchValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  // Dashboard collapse state
  let dashboardCollapsed = $state(false);
  let showSecurityReport = $state(false);
  let showAutomations = $state(false);

  // Resizable tab panel (backed by store for persistence)
  let tabPanelHeight = $state($tabPanelHeightStore);
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartHeight = 0;

  // Platform detection for OS-specific styles
  let platform = $state<"macos" | "windows" | "linux">("macos");

  // Open ProcessDetailsModal when AI chat (or any other component) requests it
  $effect(() => {
    const proc = $inspectProcessRequest;
    if (proc) {
      detailProcess = proc;
      inspectProcessRequest.set(null); // Reset after consuming
    }
  });

  // Sync local to store when resizing
  $effect(() => {
    $tabPanelHeightStore = tabPanelHeight;
  });
  // Sync store to local on load
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
  let autostartEnabled = $state(false);
  let autostartLoading = $state(true);
  let autostartError = $state<string | null>(null);

  async function handleSaveSettings() {
    settingsSaving = true;
    settingsError = null;
    settingsSaved = false;
    try {
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
      const stored = await ipcCheckApiKey($aiProviderConfig.provider);
      if (!stored) {
        settingsError = "API key could not be saved to the system keyring.";
        return;
      }
      settingsSaved = true;
      apiKeyInput = "";
    } catch (e) {
      settingsError = e instanceof Error ? e.message : String(e);
    } finally {
      settingsSaving = false;
    }
  }

  // Apply theme engine
  $effect(() => {
    applyThemeTokens($theme as ThemeId);
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

  async function loadAutostartState() {
    autostartLoading = true;
    autostartError = null;
    try {
      const autostart = await import("@tauri-apps/plugin-autostart");
      autostartEnabled = await autostart.isEnabled();
    } catch {
      autostartError = "Auto-start is unavailable in this runtime.";
    } finally {
      autostartLoading = false;
    }
  }

  async function handleAutostartToggle(e: Event) {
    const next = (e.target as HTMLInputElement).checked;
    autostartError = null;
    const prev = autostartEnabled;
    autostartEnabled = next;
    try {
      const autostart = await import("@tauri-apps/plugin-autostart");
      if (next) await autostart.enable();
      else await autostart.disable();
    } catch {
      autostartEnabled = prev;
      autostartError = "Failed to update auto-start setting.";
    }
  }

  onMount(() => {
    let disposed = false;
    const unlistenFns: Array<() => void> = [];
    const registerUnlistener = (promise: Promise<() => void>) => {
      promise
        .then((fn) => {
          if (disposed) {
            fn();
            return;
          }
          unlistenFns.push(fn);
        })
        .catch(() => {
          // Listener registration failed (non-Tauri context/tests). Ignore.
        });
    };

    platform = detectPlatform();
    document.documentElement.setAttribute("data-platform", platform);

    loadPreferences().then(() => {
      initI18n($localePreference);
      startPolling(2000);
    });
    loadAutostartState();
    const unsubPrefs = initPreferenceSubscriptions();
    const unsubLocale = localePreference.subscribe((val) => {
      locale.set(val);
    });

    registerUnlistener(initSecurityAlertListener());
    registerUnlistener(
      listen<boolean>("window-visibility", (event) => {
        if (event.payload) {
          startPolling(2000);
        } else {
          stopPolling();
        }
      }),
    );

    registerUnlistener(
      listen("open-settings", () => {
        showSettings = true;
      }),
    );

    return () => {
      disposed = true;
      stopPolling();
      clearTimeout(debounceTimer);
      window.removeEventListener("mousemove", onDividerMousemove);
      window.removeEventListener("mouseup", onDividerMouseup);
      unsubPrefs();
      unsubLocale();
      for (const unlisten of unlistenFns) {
        unlisten();
      }
      unlistenFns.length = 0;
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

    if (mod && e.key === "f") {
      e.preventDefault();
      searchInput?.focus();
      return;
    }
    if (mod && e.key === "i") {
      e.preventDefault();
      openDetailForFocused();
      return;
    }
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
  <!-- Toolbar -->
  <header class="toolbar">
    <div class="toolbar-left">
      <div class="search-wrapper">
        <svg class="search-icon" viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="6.5" cy="6.5" r="5"/>
          <line x1="10" y1="10" x2="14" y2="14"/>
        </svg>
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
    </div>
    <div class="toolbar-right">
      <!-- GitHub Sponsors Banner -->
      <a
        class="btn btn-sponsor"
        href="https://github.com/sponsors/chochy2001/dashboard"
        target="_blank"
        rel="noopener noreferrer"
        title="Support OmniMon on GitHub Sponsors"
        style="color: var(--accent); border-color: var(--accent); display: flex; align-items: center; gap: 4px;"
      >
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path fill-rule="evenodd" d="M4.25 2.5c-1.336 0-2.75 1.164-2.75 3 0 2.15 1.58 4.144 3.365 5.682A20.565 20.565 0 008 13.393a20.561 20.561 0 003.135-2.211C12.92 9.644 14.5 7.65 14.5 5.5c0-1.836-1.414-3-2.75-3-1.373 0-2.609.986-3.029 2.456a.75.75 0 01-1.442 0C6.859 3.486 5.623 2.5 4.25 2.5zM8 14.25l-.345.666-.002-.001-.006-.003-.018-.01a7.643 7.643 0 01-.31-.17 22.075 22.075 0 01-3.434-2.414C2.045 10.731 0 8.35 0 5.5 0 2.836 2.086 1 4.25 1 5.797 1 7.153 1.802 8 3.02 8.847 1.802 10.203 1 11.75 1 13.914 1 16 2.836 16 5.5c0 2.85-2.045 5.231-3.885 6.818a22.08 22.08 0 01-3.744 2.584l-.018.01-.006.003h-.002L8 14.25z"></path>
        </svg>
        Sponsor
      </a>
      <span class="separator"></span>

      <div class="btn-group">
        <button
          class="btn"
          class:active={$grouping}
          onclick={() => $grouping = !$grouping}
          title={t("toolbar.toggleGrouping")}
        >
          <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
            <rect x="1" y="1" width="5" height="5" rx="1"/>
            <rect x="1" y="9" width="5" height="5" rx="1"/>
            <rect x="9" y="1" width="5" height="5" rx="1"/>
            <rect x="9" y="9" width="5" height="5" rx="1"/>
          </svg>
        </button>
        <button class="btn" onclick={selectAllVisible} aria-label={t("toolbar.selectAll")}>{t("toolbar.all")}</button>
        <button class="btn" onclick={selectNone} aria-label={t("toolbar.deselectAll")}>{t("toolbar.none")}</button>
      </div>

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
        title={t("toolbar.profileHelp")}
      >
        <option value="general" title={t("toolbar.generalDesc")}>{t("toolbar.general")}</option>
        <option value="developer" title={t("toolbar.developerDesc")}>{t("toolbar.developer")}</option>
        <option value="gaming" title={t("toolbar.gamingDesc")}>{t("toolbar.gaming")}</option>
        <option value="battery" title={t("toolbar.batteryDesc")}>{t("toolbar.batterySaver")}</option>
      </select>
      <button
        class="btn btn-accent"
        onclick={() => analyzeWithAi($aiProviderConfig.provider, $aiProviderConfig.model)}
        disabled={$aiLoading}
      >
        {$aiLoading ? t("toolbar.analyzing") : t("toolbar.aiAnalyze")}
      </button>

      <span class="separator"></span>

      <AlertPanel />

      <button
        class="btn btn-icon"
        class:has-findings={$totalFindings > 0}
        onclick={() => showSecurityReport = true}
        title={t("toolbar.securityFindings", { count: String($totalFindings) })}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path d="M8 0L2 3v5c0 4 2.6 6.5 6 8 3.4-1.5 6-4 6-8V3L8 0zm0 2l4 2v4c0 3-1.9 5-4 6.3C5.9 13 4 11 4 8V4l4-2zm-1 4v3h2V6H7zm0 4v1.5h2V10H7z"/>
        </svg>
        {#if $totalFindings > 0}
          <span class="findings-badge">{$totalFindings}</span>
        {/if}
      </button>

      <button
        class="btn btn-icon"
        onclick={() => dashboardCollapsed = !dashboardCollapsed}
        title={dashboardCollapsed ? t("toolbar.showDashboard") : t("toolbar.hideDashboard")}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          {#if dashboardCollapsed}
            <path d="M3 4h10v1H3zM3 7h10v1H3zM3 10h10v1H3z"/>
          {:else}
            <path d="M1 1h6v6H1zM9 1h6v6H9zM1 9h6v6H1zM9 9h6v6H9z" fill="none" stroke="currentColor" stroke-width="1.2"/>
          {/if}
        </svg>
      </button>

      <button
        class="btn btn-icon"
        onclick={() => showAutomations = true}
        title={t("toolbar.automations")}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path d="M8 0a8 8 0 100 16A8 8 0 008 0zm1 11H7V7h2v4zm0-5H7V4h2v2z"/>
        </svg>
      </button>

      <button
        class="btn btn-icon"
        onclick={() => showSettings = true}
        title={t("toolbar.aiSettings")}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          <path d="M7 1h2v2.1a5 5 0 011.2.5l1.5-1.5 1.4 1.4-1.5 1.5a5 5 0 01.5 1.2H14v2h-2.1a5 5 0 01-.5 1.2l1.5 1.5-1.4 1.4-1.5-1.5a5 5 0 01-1.2.5V14H7v-2.1a5 5 0 01-1.2-.5l-1.5 1.5-1.4-1.4 1.5-1.5a5 5 0 01-.5-1.2H2V7h2.1a5 5 0 01.5-1.2L3.1 4.3l1.4-1.4 1.5 1.5A5 5 0 017 3.9V1zm1 5a2 2 0 100 4 2 2 0 000-4z"/>
        </svg>
      </button>

      <div class="font-controls">
        <button
          class="btn btn-icon"
          onclick={decreaseFontSize}
          title={t("toolbar.decreaseFont")}
          aria-label={t("toolbar.decreaseFontLabel")}
        >A-</button>
        <span class="font-size-display">{$fontSize}</span>
        <button
          class="btn btn-icon"
          onclick={increaseFontSize}
          title={t("toolbar.increaseFont")}
          aria-label={t("toolbar.increaseFontLabel")}
        >A+</button>
      </div>
    </div>
  </header>

  <!-- Dashboard with charts -->
  <SystemDashboard collapsed={dashboardCollapsed} />

  <!-- Browser Tabs Panel -->
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

  <!-- Process Table -->
  {#if $loading}
    <div class="loading" role="status" aria-busy="true">
      <div class="loading-spinner"></div>
      <span>{t("common.loading")}</span>
    </div>
  {:else}
    <ProcessTable
      processes={$filtered}
      grouping={$grouping}
      columns={$columns}
      columnOrder={$columnOrder}
      oninspect={inspectProcess}
    />
  {/if}

  <!-- AI Suggestions Panel -->
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

  <!-- AI Security Insights (human-readable) -->
  <AiInsightCard />

  <!-- Network Connection Map -->
  <NetworkMap />

  <!-- AI Interactive Chat (Tool Calling) -->
  <AIChat />

  <!-- AI Command Bar (Natural Language Config) -->
  <AiCommandBar />

  <!-- Status Footer -->
  <footer class="statusline" aria-live="polite" aria-atomic="true">
    <span>
      <span class="version-label" style="color: var(--accent); font-weight: 600;">OmniMon v5.0.0</span> &nbsp;&middot;&nbsp;
      {t("footer.processes", { count: $filtered.length })}{#if $filtered.length !== $processes.length}
        &nbsp;{t("footer.filteredFrom", { count: $processes.length })}{/if}
      {#if $selectedCount > 0}
        <span aria-hidden="true">&nbsp;&middot;&nbsp;</span>{t("footer.selected", { count: $selectedCount, ram: $selectedRamMB.toFixed(0) })}
      {/if}
    </span>
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
            <option value="cyberpunk">Cyberpunk</option>
            <option value="custom">Custom</option>
          </select>
        </div>
        {#if $theme === "custom"}
          <div class="custom-theme-editor">
            <div class="settings-row">
              <label class="settings-label" for="custom-base">Base</label>
              <select
                id="custom-base"
                class="settings-select"
                value={$customTheme?.base ?? "dark"}
                onchange={(e) => {
                  const base = (e.target as HTMLSelectElement).value as "dark" | "light" | "cyberpunk";
                  customTheme.update((ct) => ({ name: ct?.name ?? "My Theme", base, overrides: ct?.overrides ?? {} }));
                }}
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
                <option value="cyberpunk">Cyberpunk</option>
              </select>
            </div>
            {#each [
              { key: "--accent" as keyof ThemeTokens, label: "Accent" },
              { key: "--bg" as keyof ThemeTokens, label: "Background" },
              { key: "--fg" as keyof ThemeTokens, label: "Text" },
              { key: "--danger" as keyof ThemeTokens, label: "Danger" },
              { key: "--green" as keyof ThemeTokens, label: "Success" },
              { key: "--yellow" as keyof ThemeTokens, label: "Warning" },
            ] as colorOpt (colorOpt.key)}
              <div class="settings-row color-row">
                <label class="settings-label" for={`color-${colorOpt.key}`}>{colorOpt.label}</label>
                <input
                  id={`color-${colorOpt.key}`}
                  type="color"
                  class="color-picker"
                  value={$customTheme?.overrides?.[colorOpt.key] ?? ""}
                  oninput={(e) => {
                    const val = (e.target as HTMLInputElement).value;
                    customTheme.update((ct) => ({
                      name: ct?.name ?? "My Theme",
                      base: ct?.base ?? "dark",
                      overrides: { ...ct?.overrides, [colorOpt.key]: val },
                    }));
                  }}
                />
                {#if $customTheme?.overrides?.[colorOpt.key]}
                  <button
                    class="btn btn-sm"
                    onclick={() => {
                      customTheme.update((ct) => {
                        if (!ct) return ct;
                        const { [colorOpt.key]: _, ...rest } = ct.overrides;
                        return { ...ct, overrides: rest };
                      });
                    }}
                  >Reset</button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
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
        <div class="settings-row">
          <label class="settings-label" for="autostart-toggle">Auto-start</label>
          <label class="settings-toggle" for="autostart-toggle">
            <input
              id="autostart-toggle"
              type="checkbox"
              checked={autostartEnabled}
              disabled={autostartLoading}
              onchange={handleAutostartToggle}
            />
            <span>Launch OmniMon at login</span>
          </label>
        </div>
        {#if autostartError}
          <div class="settings-error">{autostartError}</div>
        {/if}
        <CloudSync />
      </div>
      <div class="settings-footer">
        <button
          class="btn btn-accent"
          onclick={handleSaveSettings}
          disabled={settingsSaving || !apiKeyInput}
        >
          {settingsSaving ? t("settings.saving") : t("settings.saveApiKey")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showSecurityReport}
  <SecurityReportView onclose={() => showSecurityReport = false} />
{/if}

<ToastContainer />

<style>
  /* ==============================
     GLOBAL RESET & BASE
     ============================== */
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI",
      Roboto, "Helvetica Neue", sans-serif;
    font-size: var(--base-font-size, 12px);
    background: var(--bg, #0a0a0b);
    color: var(--fg, #ededef);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }

  /* Platform-specific font tuning */
  :global([data-platform="windows"]) :global(body) {
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
  }
  :global([data-platform="linux"]) :global(body) {
    font-family: "Cantarell", "Noto Sans", system-ui, sans-serif;
  }

  /* Fallback theme vars for components that load before theme engine */
  :global(:root) {
    --bg: #0a0a0b;
    --bg-alt: #111113;
    --bg-hover: #1a1a1e;
    --bg-selected: #0d2847;
    --bg-surface: #161618;
    --fg: #ededef;
    --fg-dim: #71717a;
    --border: #27272a;
    --border-subtle: rgba(255,255,255,0.06);
    --accent: #3b82f6;
    --accent-hover: #2563eb;
    --accent-dim: rgba(59,130,246,0.15);
    --danger: #ef4444;
    --danger-hover: #dc2626;
    --green: #22c55e;
    --yellow: #eab308;
    --chart-cpu: #3b82f6;
    --chart-ram: #a855f7;
    --chart-net-rx: #22c55e;
    --chart-net-tx: #f97316;
    --chart-grid: rgba(255,255,255,0.04);
    --chart-bg: #0a0a0b;
    --toast-bg: #18181b;
    --toast-border: #27272a;
    --shadow-sm: 0 1px 2px rgba(0,0,0,0.4);
    --shadow-md: 0 4px 12px rgba(0,0,0,0.5);
    --shadow-lg: 0 8px 32px rgba(0,0,0,0.6);
    --radius-sm: 4px;
    --radius-md: 8px;
    --radius-lg: 12px;
  }

  /* Smooth scrollbar styling */
  :global(::-webkit-scrollbar) {
    width: 6px;
    height: 6px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--border);
    border-radius: 3px;
  }
  :global(::-webkit-scrollbar-thumb:hover) {
    background: var(--fg-dim);
  }

  /* ==============================
     LAYOUT
     ============================== */
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  /* ==============================
     TOOLBAR
     ============================== */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: calc(var(--base-font-size) * 2.5);
  }

  .toolbar-left {
    flex: 1;
    min-width: 0;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    max-width: 320px;
  }

  .search-icon {
    position: absolute;
    left: 8px;
    color: var(--fg-dim);
    pointer-events: none;
  }

  .search {
    width: 100%;
    padding: 4px 24px 4px 28px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: calc(var(--base-font-size) * 2);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .search:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-dim, rgba(59,130,246,0.15));
  }

  .search-clear {
    position: absolute;
    right: 4px;
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

  /* ==============================
     BUTTONS
     ============================== */
  .btn {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    cursor: pointer;
    white-space: nowrap;
    height: calc(var(--base-font-size) * 2);
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .btn:hover { background: var(--bg-hover); }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .btn.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-sm { padding: 2px 6px; height: auto; }

  .btn-icon {
    padding: 4px 6px;
    font-size: calc(var(--base-font-size) * 0.833);
    font-weight: 600;
  }

  .btn-group {
    display: flex;
    gap: 0;
  }
  .btn-group .btn {
    border-radius: 0;
    margin-left: -1px;
  }
  .btn-group .btn:first-child {
    border-radius: var(--radius-sm, 4px) 0 0 var(--radius-sm, 4px);
    margin-left: 0;
  }
  .btn-group .btn:last-child {
    border-radius: 0 var(--radius-sm, 4px) var(--radius-sm, 4px) 0;
  }

  .btn-kill {
    background: var(--danger);
    color: white;
    border-color: var(--danger);
    font-weight: 600;
  }
  .btn-kill:hover:not(:disabled) { background: var(--danger-hover, #b71c1c); }

  .btn-accent {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    font-weight: 600;
  }
  .btn-accent:hover:not(:disabled) { background: var(--accent-hover, #1d4ed8); }

  /* ==============================
     CONTROLS
     ============================== */
  .separator {
    width: 1px;
    height: calc(var(--base-font-size) * 1.333);
    background: var(--border);
    flex-shrink: 0;
  }

  .has-findings {
    position: relative;
    color: var(--yellow);
    border-color: var(--yellow);
  }
  .findings-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    border-radius: 7px;
    background: var(--danger);
    color: white;
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 3px;
  }

  .font-controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .font-size-display {
    font-size: calc(var(--base-font-size) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--fg-dim);
    min-width: calc(var(--base-font-size) * 1.667);
    text-align: center;
  }

  .profile-select {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    height: calc(var(--base-font-size) * 2);
    outline: none;
    cursor: pointer;
  }
  .profile-select:focus { border-color: var(--accent); }

  /* ==============================
     PANELS
     ============================== */
  .tab-panel {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .resize-divider {
    flex-shrink: 0;
    height: 3px;
    background: var(--border);
    cursor: ns-resize;
    position: relative;
    transition: background 0.15s;
  }
  .resize-divider:hover, .resize-divider.active {
    background: var(--accent);
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.917);
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ==============================
     AI PANEL
     ============================== */
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
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
  }

  .ai-title {
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
  }

  .ai-error {
    padding: 4px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--danger);
  }

  .ai-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    font-size: calc(var(--base-font-size) * 0.917);
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
    transition: background 0.1s;
  }
  .ai-row:hover { background: var(--bg-hover); }

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

  /* ==============================
     FOOTER
     ============================== */
  .statusline {
    display: flex;
    justify-content: space-between;
    padding: 3px 10px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    background: var(--bg-alt);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    min-height: calc(var(--base-font-size) * 1.667);
    line-height: calc(var(--base-font-size) * 1.333);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .shortcuts { opacity: 0.5; }
  .shortcuts :global(kbd) {
    font-family: inherit;
    font-size: inherit;
    background: var(--bg-hover);
    padding: 1px 4px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }

  /* ==============================
     SETTINGS MODAL
     ============================== */
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .settings-modal {
    background: var(--bg-surface, var(--bg-alt));
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    width: 400px;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0,0,0,0.5));
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .settings-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size) * 1.083);
    margin: 0;
  }

  .close-btn {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: var(--radius-sm, 4px);
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 1.333);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    transition: background 0.1s;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }

  .settings-body {
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .settings-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: calc(var(--base-font-size) * 0.917);
  }

  .settings-label {
    min-width: 80px;
    width: auto;
    flex-shrink: 0;
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 700;
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .settings-select {
    flex: 1;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: calc(var(--base-font-size) * 2);
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .settings-select:focus { border-color: var(--accent); }

  .settings-input {
    flex: 1;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.917);
    outline: none;
    height: calc(var(--base-font-size) * 2);
    transition: border-color 0.15s;
  }
  .settings-input:focus { border-color: var(--accent); }

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
    margin: 8px 0;
  }

  .settings-section-label {
    font-size: calc(var(--base-font-size) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.8px;
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
    padding: 3px 4px;
    font-size: calc(var(--base-font-size) * 0.917);
    border-radius: var(--radius-sm, 4px);
    transition: background 0.1s;
  }
  .col-order-row:hover { background: var(--bg-hover); }

  .col-order-row input[type="checkbox"] {
    margin: 0;
    width: 14px;
    height: 14px;
    cursor: pointer;
    accent-color: var(--accent);
  }
  .col-order-name { flex: 1; }
  .col-order-btns { display: flex; gap: 2px; }
  .col-move-btn {
    width: 20px;
    height: 18px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size) * 0.667);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s;
  }
  .col-move-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .col-move-btn:disabled { opacity: 0.3; cursor: default; }

  .settings-hint {
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
    white-space: nowrap;
  }

  .settings-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--fg);
    font-size: calc(var(--base-font-size) * 0.833);
    cursor: pointer;
  }

  .settings-toggle input[type="checkbox"] {
    margin: 0;
    width: 14px;
    height: 14px;
    accent-color: var(--accent);
  }

  .custom-theme-editor {
    padding: 6px 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    padding: 8px;
    background: var(--bg);
  }

  .color-row {
    gap: 6px;
  }

  .color-picker {
    width: 32px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    cursor: pointer;
  }
  .color-picker::-webkit-color-swatch-wrapper { padding: 1px; }
  .color-picker::-webkit-color-swatch { border-radius: 2px; border: none; }

  .settings-footer {
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
  }
</style>
