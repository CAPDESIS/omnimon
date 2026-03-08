<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import ProcessTable from "./components/ProcessTable.svelte";
  import SystemDashboard from "./components/SystemDashboard.svelte";
  import ToastContainer from "./components/ToastContainer.svelte";
  import AiCommandBar from "./components/AiCommandBar.svelte";
  import AiInsightCard from "./components/AiInsightCard.svelte";
  import InfoPopover from "./components/InfoPopover.svelte";
  import SmartAlerts from "./components/SmartAlerts.svelte";
  import AppToolbar from "./components/AppToolbar.svelte";
  import Button from "./components/Button.svelte";
  import ProfilePanel from "./components/ProfilePanel.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import SkeletonBlock from "./components/SkeletonBlock.svelte";
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
    selectedCount,
    selectedRamMB,
    focusedPid,
    grouping,
    startPolling,
    stopPolling,
    setPollingTarget,
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
    customTheme,
    userMode,
    type ThemeMode,
    profilesCollapsedStore,
    mainTableCollapsedStore,
    networkMapCollapsedStore,
    browserTabsCollapsedStore,
    aiChatCollapsedStore,
    aiConfigCollapsedStore,
  } from "./stores/preferences";
  import { ipcValidateApiKey, ipcCheckApiKey } from "./lib/ipc";
  import { listen } from "@tauri-apps/api/event";
  import { t, locale, initI18n } from "./lib/i18n";
  import { inspectProcessRequest } from "./stores/uiActions";
  import type { LocaleCode } from "./lib/i18n";
  import { focusFirstFocusable, trapFocus, rememberActiveElement, restoreFocus } from "./lib/focusTrap";

  let detailProcess: ProcessEntry | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let searchValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  // Panel collapse states
  let dashboardCollapsed = $state(false);
  let showSecurityReport = $state(false);
  let showAutomations = $state(false);
  let showPlugins = $state(false);
  let showHelpCenter = $state(false);
  let activeMetricModal = $state<"cpu" | "ram" | "network" | "swap" | "processes" | null>(null);
  let settingsModalEl: HTMLDivElement | undefined = $state();
  let settingsReturnFocusEl: HTMLElement | null = $state(null);
  let chromeTabsHost: HTMLDivElement | undefined = $state();
  let networkMapHost: HTMLDivElement | undefined = $state();
  let aiChatHost: HTMLDivElement | undefined = $state();

  let chromeTabManagerPromise = $state<Promise<any> | null>(null);
  let processDetailsModalPromise = $state<Promise<any> | null>(null);
  let securityReportViewPromise = $state<Promise<any> | null>(null);
  let helpCenterModalPromise = $state<Promise<any> | null>(null);
  let systemMetricModalPromise = $state<Promise<any> | null>(null);
  let aiChatPromise = $state<Promise<any> | null>(null);
  let networkMapPromise = $state<Promise<any> | null>(null);
  let cloudSyncPromise = $state<Promise<any> | null>(null);
  let automationsPromise = $state<Promise<any> | null>(null);
  let pluginsPromise = $state<Promise<any> | null>(null);

  function loadChromeTabManager() {
    chromeTabManagerPromise ??= import("./components/ChromeTabManager.svelte");
    setPollingTarget("browserTabs", true);
  }

  function loadProcessDetailsModal() {
    processDetailsModalPromise ??= import("./components/ProcessDetailsModal.svelte");
  }

  function loadSecurityReportView() {
    securityReportViewPromise ??= import("./components/SecurityReportView.svelte");
  }

  function loadHelpCenterModal() {
    helpCenterModalPromise ??= import("./components/HelpCenterModal.svelte");
  }

  function loadSystemMetricModal() {
    systemMetricModalPromise ??= import("./components/SystemMetricModal.svelte");
  }

  function loadAiChat() {
    aiChatPromise ??= import("./components/AIChat.svelte");
  }

  function loadNetworkMap() {
    networkMapPromise ??= import("./components/NetworkMap.svelte");
    setPollingTarget("network", true);
  }

  function loadCloudSync() {
    cloudSyncPromise ??= import("./components/CloudSync.svelte");
  }

  function loadAutomations() {
    automationsPromise ??= import("./components/Automations.svelte");
  }

  function loadPlugins() {
    pluginsPromise ??= import("./components/Plugins.svelte");
  }

  function closeWhenBackdropMatches(event: MouseEvent, onclose: () => void) {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }

  function closeSettingsFromBackdrop(event: MouseEvent) {
    closeWhenBackdropMatches(event, closeSettings);
  }

  function stopMouseEventPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  function observeVisibility(node: HTMLElement | undefined, onVisible: () => void): IntersectionObserver | null {
    if (!node || typeof IntersectionObserver === "undefined") {
      onVisible();
      return null;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            onVisible();
            observer.disconnect();
            break;
          }
        }
      },
      { rootMargin: "240px 0px" },
    );

    observer.observe(node);
    return observer;
  }

  // Resizable tab panel (backed by store for persistence)
  let tabPanelHeight = $state($tabPanelHeightStore);
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartHeight = 0;
  let aiChatPanelHeight = $state(220);
  let aiChatDragging = $state(false);
  let aiChatDragStartY = 0;
  let aiChatDragStartHeight = 0;

  // Platform detection for OS-specific styles
  let platform = $state<"macos" | "windows" | "linux">("macos");

  // Open ProcessDetailsModal when AI chat (or any other component) requests it
  $effect(() => {
    const proc = $inspectProcessRequest;
    if (proc) {
      loadProcessDetailsModal();
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

  function onDividerKeydown(event: KeyboardEvent) {
    const step = event.shiftKey ? 40 : 20;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      tabPanelHeight = Math.max(40, tabPanelHeight - step);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      tabPanelHeight = Math.min(window.innerHeight - 200, tabPanelHeight + step);
    }
  }

  function onAiChatDividerMousedown(e: MouseEvent) {
    e.preventDefault();
    aiChatDragging = true;
    aiChatDragStartY = e.clientY;
    aiChatDragStartHeight = aiChatPanelHeight;
    window.addEventListener("mousemove", onAiChatDividerMousemove);
    window.addEventListener("mouseup", onAiChatDividerMouseup);
  }

  function onAiChatDividerMousemove(e: MouseEvent) {
    const delta = e.clientY - aiChatDragStartY;
    aiChatPanelHeight = Math.max(140, Math.min(aiChatDragStartHeight + delta, 640));
  }

  function onAiChatDividerMouseup() {
    aiChatDragging = false;
    window.removeEventListener("mousemove", onAiChatDividerMousemove);
    window.removeEventListener("mouseup", onAiChatDividerMouseup);
  }

  function onAiChatDividerKeydown(event: KeyboardEvent) {
    const step = event.shiftKey ? 40 : 20;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      aiChatPanelHeight = Math.max(140, aiChatPanelHeight - step);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      aiChatPanelHeight = Math.min(640, aiChatPanelHeight + step);
    }
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
        settingsError = t("settings.apiKeyKeyringError");
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

  $effect(() => {
    if (showSettings) {
      settingsReturnFocusEl = rememberActiveElement();
      loadCloudSync();
    }
  });

  function closeSettings() {
    showSettings = false;
    settingsError = null;
    settingsSaved = false;
    restoreFocus(settingsReturnFocusEl);
  }

  function handleSettingsKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeSettings();
      return;
    }
    trapFocus(event, settingsModalEl);
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
      autostartError = t("settings.autostartUnavailable");
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
      autostartError = t("settings.autostartUpdateFailed");
    }
  }

  onMount(() => {
    let disposed = false;
    const unlistenFns: Array<() => void> = [];
    const observers: Array<IntersectionObserver> = [];
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
      listen<boolean>("window-visibility", (event: { payload: boolean }) => {
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

    const chromeObserver = observeVisibility(chromeTabsHost, loadChromeTabManager);
    if (chromeObserver) observers.push(chromeObserver);

    const networkObserver = observeVisibility(networkMapHost, loadNetworkMap);
    if (networkObserver) observers.push(networkObserver);

    const aiChatObserver = observeVisibility(aiChatHost, loadAiChat);
    if (aiChatObserver) observers.push(aiChatObserver);

    return () => {
      disposed = true;
      stopPolling();
      clearTimeout(debounceTimer);
      window.removeEventListener("mousemove", onDividerMousemove);
      window.removeEventListener("mouseup", onDividerMouseup);
      window.removeEventListener("mousemove", onAiChatDividerMousemove);
      window.removeEventListener("mouseup", onAiChatDividerMouseup);
      unsubPrefs();
      unsubLocale();
      for (const unlisten of unlistenFns) {
        unlisten();
      }
      for (const observer of observers) {
        observer.disconnect();
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
    loadProcessDetailsModal();
    detailProcess = proc;
  }

  function closeDetail() {
    detailProcess = null;
  }

  function openSecurityReport() {
    loadSecurityReportView();
    showSecurityReport = true;
  }

  function toggleAutomations() {
    if (!showAutomations) {
      loadAutomations();
    }
    showAutomations = !showAutomations;
  }

  function openPlugins() {
    loadPlugins();
    showPlugins = true;
  }

  function openHelpCenter() {
    loadHelpCenterModal();
    showHelpCenter = true;
  }

  function openMetricModal(metric: "cpu" | "ram" | "network" | "swap" | "processes") {
    loadSystemMetricModal();
    activeMetricModal = metric;
  }

  let visibleColumns = $derived.by(() => {
    if ($userMode === "pro") return $columns;
    return {
      ...$columns,
      detail: false,
      energy: false,
      network: false,
      uptime: false,
      pid: false,
      state: false,
    };
  });

  let basicModeNetworkHint = $derived($userMode === "basic");

  $effect(() => {
    if (showSettings) {
      requestAnimationFrame(() => focusFirstFocusable(settingsModalEl));
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<main style="--base-font-size: {$fontSize}px">
  <AppToolbar
    searchValue={searchValue}
    onsearch={onSearchInput}
    onclearsearch={() => { searchValue = ""; $search = ""; }}
    selectedCount={$selectedCount}
    selectedRamMB={$selectedRamMB}
    grouping={$grouping}
    totalFindings={$totalFindings}
    aiLoading={$aiLoading}
    aiProfile={$aiProfile}
    fontSize={$fontSize}
    onselectall={selectAllVisible}
    onselectnone={selectNone}
    onkillselected={killSelected}
    ontogglegrouping={() => $grouping = !$grouping}
    onchangepofile={(value) => $aiProfile = value}
    onanalyze={() => analyzeWithAi($aiProviderConfig.provider, $aiProviderConfig.model)}
    onopensecurity={openSecurityReport}
    ontoggledashboard={() => dashboardCollapsed = !dashboardCollapsed}
    dashboardCollapsed={dashboardCollapsed}
    ontoggleautomations={toggleAutomations}
    onopenplugins={openPlugins}
    onopensettings={() => showSettings = true}
    onopenhelp={openHelpCenter}
    ondecreasefont={decreaseFontSize}
    onincreasefont={increaseFontSize}
  />

  <div class="section-header" role="button" tabindex="0"
    onclick={() => $profilesCollapsedStore = !$profilesCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $profilesCollapsedStore = !$profilesCollapsedStore; } }}
    aria-expanded={!$profilesCollapsedStore}
  >
    <span class="section-chevron" class:open={!$profilesCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("toolbar.aiProfile")}</span>
  </div>
  {#if !$profilesCollapsedStore}
    <div class="profiles-shell">
      <ProfilePanel />
    </div>
  {/if}

  <!-- Dashboard with charts -->
  <SystemDashboard collapsed={dashboardCollapsed} mode={$userMode} onopenmetric={openMetricModal} />

  <!-- Browser Tabs Panel -->
  <div class="section-header" role="button" tabindex="0"
    onclick={() => $browserTabsCollapsedStore = !$browserTabsCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $browserTabsCollapsedStore = !$browserTabsCollapsedStore; } }}
    aria-expanded={!$browserTabsCollapsedStore}
  >
    <span class="section-chevron" class:open={!$browserTabsCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("common.browserTabs", { default: "Browser Tabs" })}</span>
  </div>
  {#if !$browserTabsCollapsedStore}
    <div class="tab-panel" style="height: {tabPanelHeight}px" bind:this={chromeTabsHost}>
      {#if chromeTabManagerPromise}
        {#await chromeTabManagerPromise then ChromeTabManagerModule}
          <ChromeTabManagerModule.default filter={searchValue} />
        {:catch}
          <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
            <SkeletonBlock width="100%" height="100%" rounded="12px" />
          </div>
        {/await}
      {:else}
        <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
          <SkeletonBlock width="100%" height="100%" rounded="12px" />
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="resize-divider"
      class:active={dragging}
      onmousedown={onDividerMousedown}
      onkeydown={onDividerKeydown}
      aria-label={t("common.resizeTabPanel")}
    ></button>
  {/if}

  <!-- Process Table -->
  <div class="section-header" role="button" tabindex="0"
    onclick={() => $mainTableCollapsedStore = !$mainTableCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $mainTableCollapsedStore = !$mainTableCollapsedStore; } }}
    aria-expanded={!$mainTableCollapsedStore}
  >
    <span class="section-chevron" class:open={!$mainTableCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("common.processes", { default: "Processes" })}</span>
  </div>
  {#if !$mainTableCollapsedStore}
    {#if $loading}
      <div class="loading-shell" role="status" aria-busy="true" aria-label={t("common.loadingAria")}>
        <div class="loading-toolbar-card">
          <SkeletonBlock width="22%" height="14px" rounded="999px" />
          <SkeletonBlock width="100%" height="42px" rounded="14px" />
        </div>
        <div class="loading-table-card">
          <div class="loading-table-header">
            <SkeletonBlock width="14%" height="12px" rounded="999px" />
            <SkeletonBlock width="10%" height="12px" rounded="999px" />
            <SkeletonBlock width="12%" height="12px" rounded="999px" />
            <SkeletonBlock width="8%" height="12px" rounded="999px" />
          </div>
          {#each Array(7) as _, index}
            <div class="loading-row" style={`animation-delay:${index * 50}ms`}>
              <SkeletonBlock width="28px" height="28px" rounded="8px" />
              <SkeletonBlock width="20%" height="12px" rounded="999px" />
              <SkeletonBlock width="12%" height="12px" rounded="999px" />
              <SkeletonBlock width="16%" height="12px" rounded="999px" />
              <SkeletonBlock width="10%" height="12px" rounded="999px" />
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <ProcessTable
        processes={$filtered}
        grouping={$grouping}
        columns={visibleColumns}
        columnOrder={$columnOrder}
        oninspect={inspectProcess}
      />
    {/if}
  {/if}

  <!-- AI Suggestions Panel -->
  {#if $aiError || $aiSuggestions.length > 0}
    <div class="ai-panel" role="region" aria-label={t("ai.suggestions")}>
      <div class="ai-header">
        <span class="ai-title">{t("ai.suggestions")}</span>
        <InfoPopover label={t("ai.suggestions")} content={t("toolbar.aiSuggestionsHelp")} />
        <Button variant="ghost" size="sm" onclick={dismissAiSuggestions}>{t("ai.dismiss")}</Button>
      </div>
      {#if $aiError}
        <div class="ai-error">{$aiError}</div>
      {/if}
      {#each $aiSuggestions as suggestion (suggestion.pid)}
        <div class="ai-row">
          <span class="ai-name">{suggestion.name}</span>
          <span class="ai-pid">{t("ai.pid", { pid: suggestion.pid })}</span>
          <span class="ai-reason">{suggestion.reason}</span>
          <Button
            variant="danger"
            size="sm"
            onclick={() => killSingle(suggestion.pid)}
          >{t("ai.close")}</Button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- AI Security Insights (human-readable) -->
  {#if $userMode === "pro"}
    <AiInsightCard />
  {/if}

  <!-- Network Connection Map -->
  <div class="section-header" role="button" tabindex="0"
    onclick={() => $networkMapCollapsedStore = !$networkMapCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $networkMapCollapsedStore = !$networkMapCollapsedStore; } }}
    aria-expanded={!$networkMapCollapsedStore}
  >
    <span class="section-chevron" class:open={!$networkMapCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("common.networkMap", { default: "Network Map" })}</span>
  </div>
  {#if !$networkMapCollapsedStore}
    <div bind:this={networkMapHost}>
      {#if networkMapPromise}
        {#await networkMapPromise then NetworkMapModule}
          <NetworkMapModule.default mode={$userMode} />
        {:catch}
          <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
            <SkeletonBlock width="100%" height="140px" rounded="14px" />
          </div>
        {/await}
      {:else}
        <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
          <SkeletonBlock width="100%" height="140px" rounded="14px" />
        </div>
      {/if}
    </div>
    {#if basicModeNetworkHint}
      <div class="mode-hint-card" role="note">
        <span class="mode-hint-label">{t("common.userView")}</span>
        <span>{t("profiles.proHint")}</span>
      </div>
    {/if}
  {/if}

  <!-- AI Interactive Chat (Tool Calling) -->
  <div class="section-header" role="button" tabindex="0"
    onclick={() => $aiChatCollapsedStore = !$aiChatCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $aiChatCollapsedStore = !$aiChatCollapsedStore; } }}
    aria-expanded={!$aiChatCollapsedStore}
  >
    <span class="section-chevron" class:open={!$aiChatCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("aiChat.title")}</span>
  </div>
  {#if !$aiChatCollapsedStore}
    <div class="ai-chat-panel" style="height: {aiChatPanelHeight}px" bind:this={aiChatHost}>
      {#if aiChatPromise}
        {#await aiChatPromise then AIChatModule}
          <AIChatModule.default />
        {:catch}
          <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
            <SkeletonBlock width="100%" height="100%" rounded="12px" />
          </div>
        {/await}
      {:else}
        <div class="lazy-panel-fallback" role="status" aria-label={t("common.loadingAria")}>
          <SkeletonBlock width="100%" height="100%" rounded="12px" />
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="resize-divider"
      class:active={aiChatDragging}
      onmousedown={onAiChatDividerMousedown}
      onkeydown={onAiChatDividerKeydown}
      aria-label={t("common.expand")}
    ></button>
  {/if}

  <!-- AI Command Bar (Natural Language Config) -->
  <div class="section-header" role="button" tabindex="0"
    onclick={() => $aiConfigCollapsedStore = !$aiConfigCollapsedStore}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); $aiConfigCollapsedStore = !$aiConfigCollapsedStore; } }}
    aria-expanded={!$aiConfigCollapsedStore}
  >
    <span class="section-chevron" class:open={!$aiConfigCollapsedStore}>&#9654;</span>
    <span class="section-label">{t("aiConfig.title")}</span>
  </div>
  {#if !$aiConfigCollapsedStore}
    <AiCommandBar />
  {/if}

  <!-- Status Footer -->
  <footer class="statusline" aria-live="polite" aria-atomic="true">
    <span>
      <span class="version-label">OmniMon v6.0.0</span> &nbsp;&middot;&nbsp;
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
  {#if processDetailsModalPromise}
    {#await processDetailsModalPromise then ProcessDetailsModalModule}
      <ProcessDetailsModalModule.default process={detailProcess} onclose={closeDetail} />
    {/await}
  {/if}
{/if}

{#if showSettings}
  <div class="backdrop" onmousedown={closeSettingsFromBackdrop} role="presentation">
    <div class="settings-modal" bind:this={settingsModalEl} onkeydown={handleSettingsKeydown} onmousedown={stopMouseEventPropagation} role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1">
      <div class="settings-header">
        <h2 class="settings-title" id="settings-title">{t("settings.title")}</h2>
        <Button variant="ghost" size="icon" class="settings-close-button" onclick={closeSettings} aria-label={t("settings.closeSettings")}>&times;</Button>
      </div>
      <div class="settings-body">
        <div class="settings-row">
          <label class="settings-label" for="provider-select">{t("settings.provider")}</label>
          <select
            id="provider-select"
            class="settings-select"
            value={$aiProviderConfig.provider}
            onchange={(e: Event) => {
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
            oninput={(e: Event) => {
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
                <Button
                  class="col-move-btn"
                  variant="ghost"
                  size="sm"
                  disabled={i === 0}
                  onclick={() => moveColumnUp(key)}
                  title={t("settings.moveUp")}
                  aria-label={t("settings.moveColumnUp", { column: key })}
                >&#9650;</Button>
                <Button
                  class="col-move-btn"
                  variant="ghost"
                  size="sm"
                  disabled={i === $columnOrder.length - 1}
                  onclick={() => moveColumnDown(key)}
                  title={t("settings.moveDown")}
                  aria-label={t("settings.moveColumnDown", { column: key })}
                >&#9660;</Button>
              </div>
            </div>
          {/each}
        </div>

        <div class="settings-divider"></div>
        <div class="settings-section-label">{t("settings.appearance")}</div>
        <div class="settings-row">
          <label class="settings-label" for="user-mode-select">{t("common.userView")}</label>
          <div class="settings-field-stack">
            <select
              id="user-mode-select"
              class="settings-select"
              value={$userMode}
              onchange={(e: Event) => { $userMode = (e.target as HTMLSelectElement).value === "basic" ? "basic" : "pro"; }}
            >
              <option value="basic">{t("profiles.basic")}</option>
              <option value="pro">{t("profiles.pro")}</option>
            </select>
            <span class="settings-hint">{t("common.userViewHelp")}</span>
          </div>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="theme-select">{t("settings.theme")}</label>
          <select
            id="theme-select"
            class="settings-select"
            value={$theme}
            onchange={(e: Event) => { $theme = (e.target as HTMLSelectElement).value as ThemeMode; }}
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
                onchange={(e: Event) => {
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
                  oninput={(e: Event) => {
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
            onchange={(e: Event) => { $localePreference = (e.target as HTMLSelectElement).value as LocaleCode; }}
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
            oninput={(e: Event) => {
              const v = parseFloat((e.target as HTMLInputElement).value);
              if (!isNaN(v) && v >= MIN_IDLE_THRESHOLD && v <= MAX_IDLE_THRESHOLD) {
                $idleThreshold = v;
              }
            }}
          />
          <span class="settings-hint">{t("settings.idleHint")}</span>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="autostart-toggle">{t("settings.autostart")}</label>
          <label class="settings-toggle" for="autostart-toggle">
            <input
              id="autostart-toggle"
              type="checkbox"
              checked={autostartEnabled}
              disabled={autostartLoading}
              onchange={handleAutostartToggle}
            />
            <span>{t("settings.launchAtLogin")}</span>
          </label>
        </div>
        {#if autostartError}
          <div class="settings-error">{autostartError}</div>
        {/if}
        {#if cloudSyncPromise}
          {#await cloudSyncPromise then CloudSyncModule}
            <CloudSyncModule.default />
          {/await}
        {/if}
      </div>
      <div class="settings-footer">
        <Button
          variant="primary"
          onclick={handleSaveSettings}
          disabled={settingsSaving || !apiKeyInput}
        >
          {settingsSaving ? t("settings.saving") : t("settings.saveApiKey")}
        </Button>
      </div>
    </div>
  </div>
{/if}

{#if showSecurityReport}
  {#if securityReportViewPromise}
    {#await securityReportViewPromise then SecurityReportViewModule}
      <SecurityReportViewModule.default onclose={() => showSecurityReport = false} />
    {/await}
  {/if}
{/if}

<ToastContainer />
<SmartAlerts />

{#if showAutomations}
  {#if automationsPromise}
    {#await automationsPromise then AutomationsModule}
      <AutomationsModule.default />
    {/await}
  {/if}
{/if}

{#if showPlugins}
  {#if pluginsPromise}
    {#await pluginsPromise then PluginsModule}
      <PluginsModule.default onclose={() => showPlugins = false} />
    {/await}
  {/if}
{/if}

{#if showHelpCenter}
  {#if helpCenterModalPromise}
    {#await helpCenterModalPromise then HelpCenterModalModule}
      <HelpCenterModalModule.default onclose={() => showHelpCenter = false} />
    {/await}
  {/if}
{/if}

{#if activeMetricModal}
  {#if systemMetricModalPromise}
    {#await systemMetricModalPromise then SystemMetricModalModule}
      <SystemMetricModalModule.default metric={activeMetricModal} mode={$userMode} onclose={() => activeMetricModal = null} />
    {/await}
  {/if}
{/if}

<ConfirmDialog />

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
     BUTTONS
     ============================== */
  /* ==============================
     COLLAPSIBLE SECTION HEADERS
     ============================== */
  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
    cursor: pointer;
    user-select: none;
    min-height: calc(var(--base-font-size) * 1.8);
    flex-shrink: 0;
  }
  .section-header:hover {
    background: var(--bg-hover);
  }
  .section-chevron {
    font-size: calc(var(--base-font-size) * 0.6);
    color: var(--fg-dim);
    transition: transform 0.15s ease;
    display: inline-block;
  }
  .section-chevron.open {
    transform: rotate(90deg);
  }
  .section-label {
    font-size: calc(var(--base-font-size) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
  }
  .profiles-shell {
    padding: 0 16px 16px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
  }

  /* ==============================
     PANELS
     ============================== */
  .tab-panel {
    flex: 1 1 auto;
    overflow: auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }

  .lazy-panel-fallback {
    width: 100%;
    height: 100%;
    padding: 14px 16px;
    background: var(--bg-alt);
  }

  .resize-divider {
    flex-shrink: 0;
    height: 3px;
    width: 100%;
    padding: 0;
    border: none;
    background: var(--border);
    cursor: ns-resize;
    position: relative;
    transition: background 0.15s;
  }
  .resize-divider:hover, .resize-divider.active {
    background: var(--accent);
  }

  .loading-shell {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 18px 16px 22px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-alt) 92%, white 2%), var(--bg));
  }

  .loading-toolbar-card,
  .loading-table-card,
  .mode-hint-card {
    border: 1px solid var(--border);
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 92%, white 2%);
    box-shadow: 0 18px 28px rgba(0, 0, 0, 0.08);
  }

  .loading-toolbar-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
  }

  .loading-table-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    flex: 1;
  }

  .loading-table-header,
  .loading-row {
    display: grid;
    grid-template-columns: 28px 2fr 1fr 1fr 0.8fr;
    gap: 10px;
    align-items: center;
  }

  .mode-hint-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 12px 16px 0;
    padding: 14px 16px;
    color: var(--fg-dim);
    line-height: 1.45;
  }

  .mode-hint-label {
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    font-size: calc(var(--base-font-size) * 0.72);
    font-weight: 800;
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

  .ai-chat-panel {
    flex: 1 1 auto;
    overflow: auto;
    min-height: 0;
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
    padding: 8px 16px;
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--fg-dim);
    background: var(--bg-alt);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    min-height: calc(var(--base-font-size) * 2.4);
    line-height: calc(var(--base-font-size) * 1.333);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .version-label {
    color: var(--accent);
    font-weight: 700;
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
    border-radius: 18px;
    width: min(460px, calc(100vw - 32px));
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0,0,0,0.5));
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border);
  }

  .settings-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size) * 1.083);
    margin: 0;
  }

  :global(.settings-close-button) {
    flex-shrink: 0;
  }

  .settings-body {
    padding: 16px 18px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .settings-row {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: calc(var(--base-font-size) * 0.917);
  }

  .settings-field-stack {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
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
    gap: 8px;
    padding: 6px 8px;
    font-size: calc(var(--base-font-size) * 0.917);
    border-radius: 10px;
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
  .col-order-btns { display: flex; gap: 6px; }
  :global(.col-move-btn) {
    min-width: 30px;
    padding: 0 8px;
  }

  .settings-hint {
    font-size: calc(var(--base-font-size) * 0.75);
    color: var(--fg-dim);
    white-space: normal;
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
    padding: 16px 18px 18px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 840px) {
    .loading-table-header,
    .loading-row {
      grid-template-columns: 24px 1.8fr 1fr;
    }

    .loading-table-header :global(.skeleton-block:nth-child(n+4)),
    .loading-row :global(.skeleton-block:nth-child(n+4)) {
      display: none;
    }

    .settings-row {
      align-items: flex-start;
      flex-direction: column;
    }

    .settings-label {
      min-width: 0;
    }
  }
</style>
