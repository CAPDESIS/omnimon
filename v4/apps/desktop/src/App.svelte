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
  import AppLayout from "./components/layout/AppLayout.svelte";
  import AppHeader from "./components/layout/AppHeader.svelte";
  import AppSidebar from "./components/layout/AppSidebar.svelte";
  import AppStatusBar from "./components/layout/AppStatusBar.svelte";
  import NavigationTabs from "./components/layout/NavigationTabs.svelte";
  import AIConfigPanel from "./components/layout/AIConfigPanel.svelte";

  import Button from "./components/Button.svelte";
  import ProfilePanel from "./components/ProfilePanel.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import Skeleton from "./components/Skeleton.svelte";
  import AIChat from "./components/AIChat.svelte";
  import ThemeSelector from "./components/ThemeSelector.svelte";
  import { totalFindings } from "./stores/security";
  import { initSecurityAlertListener } from "./stores/alerts";
  import type { ProcessEntry } from "./lib/types";
  import { AI_PROVIDERS, type AiProviderKind } from "./lib/types";
  import { detectPlatform } from "./lib/theme";
  import { applyTheme, getTheme } from "./lib/themes";
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
  // AIChat is now eagerly imported (no lazy loading needed for ~11KB component)
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

  // Section resize heights
  let networkMapExtraHeight = $state(0);
  let aiChatExtraHeight = $state(0);

  function resizeSection(section: "tabs" | "network" | "aichat", delta: number) {
    if (section === "tabs") {
      tabPanelHeight = Math.max(80, Math.min(tabPanelHeight + delta, 800));
    } else if (section === "network") {
      networkMapExtraHeight = Math.max(-200, Math.min(networkMapExtraHeight + delta, 400));
    } else if (section === "aichat") {
      aiChatExtraHeight = Math.max(-100, Math.min(aiChatExtraHeight + delta, 400));
    }
  }

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
    applyTheme(getTheme($theme));
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
    console.debug("[APP] onMount started");
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
        .catch((err) => {
          console.warn("[APP] Listener registration failed:", err);
        });
    };

    platform = detectPlatform();
    console.debug(`[APP] Platform detected: ${platform}`);
    document.documentElement.setAttribute("data-platform", platform);

    loadPreferences().then(() => {
      console.debug("[APP] Preferences loaded, initializing i18n and polling.");
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

    // AI Chat loads on-click (no observer needed since host is inside {#if collapsed})

    console.debug("[APP] onMount finished");
    return () => {
      console.debug("[APP] onMount cleanup (disposed)");
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
  let activeTab = $state<"processes" | "network" | "browser" | "aichat" | "settings">("processes");

</script>

<svelte:window onkeydown={handleKeydown} />

<AppLayout fontSize={$fontSize}>
  {#snippet header()}
    <AppHeader
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
      onchangepofile={(value: string) => $aiProfile = value}
      onanalyze={() => analyzeWithAi($aiProviderConfig.provider, $aiProviderConfig.model)}
      onopensecurity={openSecurityReport}
      ontoggledashboard={() => dashboardCollapsed = !dashboardCollapsed}
      dashboardCollapsed={dashboardCollapsed}
      ontoggleautomations={toggleAutomations}
      onopenplugins={openPlugins}
      onopensettings={() => { showSettings = true; activeTab = "settings"; }}
      onopenhelp={openHelpCenter}
      ondecreasefont={decreaseFontSize}
      onincreasefont={increaseFontSize}
    />
  {/snippet}

  {#snippet sidebar()}
    <AppSidebar 
      dashboardCollapsed={dashboardCollapsed}
      userMode={$userMode}
      onopenmetric={openMetricModal}
    />
  {/snippet}

  {#snippet main()}
    <NavigationTabs 
      {activeTab} 
      ontabchange={(t) => {
        activeTab = t;
        if (t === "browser") loadChromeTabManager();
        if (t === "network") loadNetworkMap();
      }}
    />
    
    <div class="main-content-area">
      {#if activeTab === "processes"}
        <div class="tab-pane">
          {#if $loading}
            <div class="loading-shell" role="status" aria-busy="true" aria-label={t("common.loadingAria")}>
              <div class="loading-toolbar-card">
                <Skeleton width="22%" height="14px" borderRadius="999px" />
                <Skeleton width="100%" height="42px" borderRadius="14px" />
              </div>
              <div class="loading-table-card">
                <div class="loading-table-header">
                  <Skeleton width="14%" height="12px" borderRadius="999px" />
                  <Skeleton width="10%" height="12px" borderRadius="999px" />
                  <Skeleton width="12%" height="12px" borderRadius="999px" />
                </div>
                <div class="loading-table-body">
                  {#each Array(12) as _, i}
                    <Skeleton width="100%" height="24px" borderRadius="4px" />
                  {/each}
                </div>
              </div>
            </div>
          {:else}
            <ProcessTable processes={$filtered} grouping={$grouping} columns={visibleColumns} columnOrder={$columnOrder} oninspect={inspectProcess} />
          {/if}
        </div>
      {/if}

      {#if activeTab === "network"}
        <div class="tab-pane" bind:this={networkMapHost}>
          {#if basicModeNetworkHint}
            <div class="mode-hint-card" role="note" style="border: 1px solid var(--border); border-radius: 16px; background: color-mix(in srgb, var(--bg-card, var(--bg-secondary)) 92%, white 2%); padding: 14px 16px; margin: 12px 16px; display: flex; flex-direction: column; gap: 6px;">
              <span class="mode-hint-label" style="color: var(--accent); text-transform: uppercase; font-size: calc(var(--base-font-size) * 0.75); font-weight: 800; letter-spacing: 0.5px;">{t("common.userView")}</span>
              <span style="font-size: calc(var(--base-font-size) * 0.95); line-height: 1.4;">{t("profiles.proHint")}</span>
            </div>
          {/if}
          {#if networkMapPromise}
            {#await networkMapPromise then NetworkMapModule}
              <NetworkMapModule.default filter={searchValue} />
            {:catch}
              <div class="lazy-panel-fallback">
                <Skeleton width="100%" height="100%" borderRadius="12px" />
              </div>
            {/await}
          {:else}
            <div class="lazy-panel-fallback">
              <Skeleton width="100%" height="100%" borderRadius="12px" />
            </div>
          {/if}
        </div>
      {/if}

      {#if activeTab === "browser"}
        <div class="tab-pane" bind:this={chromeTabsHost}>
          {#if chromeTabManagerPromise}
            {#await chromeTabManagerPromise then ChromeTabManagerModule}
              <ChromeTabManagerModule.default filter={searchValue} />
            {:catch}
              <div class="lazy-panel-fallback">
                <Skeleton width="100%" height="100%" borderRadius="12px" />
              </div>
            {/await}
          {:else}
            <div class="lazy-panel-fallback">
              <Skeleton width="100%" height="100%" borderRadius="12px" />
            </div>
          {/if}
        </div>
      {/if}

      {#if activeTab === "aichat"}
        <div class="tab-pane aichat-pane" bind:this={aiChatHost}>
          <AIConfigPanel 
            isCollapsed={$aiConfigCollapsedStore} 
            ontoggle={() => $aiConfigCollapsedStore = !$aiConfigCollapsedStore} 
          />
          <AIChat />
        </div>
      {/if}

      {#if activeTab === "settings"}
        <div class="tab-pane settings-pane">
          <!-- We can move the settings content here or just let the modal show -->
          <div style="padding: 24px; color: var(--text-secondary);">
            {t("settings.title")} 
            <br/><br/>
            <Button onclick={() => showSettings = true}>Open Settings Modal</Button>
          </div>
        </div>
      {/if}
    </div>
  {/snippet}

  {#snippet footer()}
    <AppStatusBar 
      filteredCount={$filtered.length}
      totalCount={$processes.length}
      selectedCount={$selectedCount}
      selectedRamMB={$selectedRamMB}
    />
  {/snippet}

  {#snippet modals()}
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
          <ThemeSelector />
        </div>
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

  {/snippet}
</AppLayout>
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
    background: var(--bg-primary, #0a0a0b);
    color: var(--text-primary, #ededef);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }

  /* ... layout ... */
  .main-content-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    background: var(--bg-primary, #0a0a0b);
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow-y: auto;
  }

  .aichat-pane {
    overflow: hidden;
  }

  /* ==============================
     MODALS & BACKDROPS
     ============================== */
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .settings-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 600px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 32px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .settings-title {
    margin: 0;
    font-size: calc(var(--base-font-size) * 1.2);
    font-weight: 600;
  }

  .settings-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .settings-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .settings-label {
    min-width: 140px;
    font-weight: 500;
  }

  .settings-input {
    flex: 1;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: var(--base-font-size);
  }

  .settings-hint {
    font-size: calc(var(--base-font-size) * 0.85);
    color: var(--text-secondary);
    margin-top: 4px;
    margin-left: 152px;
  }

  .settings-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .lazy-panel-fallback {
    display: flex;
    justify-content: center;
    align-items: center;
    flex: 1;
    padding: 20px;
    height: 100%;
  }

  /* Re-add loading card styles specifically for the process table skeleton */
  .loading-shell {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
    height: 100%;
  }

  .loading-toolbar-card,
  .loading-table-card {
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--bg-secondary);
    padding: 16px;
  }

  .loading-toolbar-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .loading-table-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .loading-table-header {
    display: flex;
    gap: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
  }

  .loading-table-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
