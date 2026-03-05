<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import ProcessTable from "./components/ProcessTable.svelte";
  import ChromeTabManager from "./components/ChromeTabManager.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import ProcessDetailsModal from "./components/ProcessDetailsModal.svelte";
  import type { ProcessEntry } from "./lib/types";
  import { AI_PROVIDERS } from "./lib/types";
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
    aiProviderConfig,
    loadPreferences,
    initPreferenceSubscriptions,
    increaseFontSize,
    decreaseFontSize,
  } from "./stores/preferences";

  let detailProcess: ProcessEntry | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let searchValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  // AI settings modal state
  let showSettings = $state(false);
  let apiKeyInput = $state("");
  let settingsSaving = $state(false);
  let settingsError = $state<string | null>(null);
  let settingsSaved = $state(false);

  let selectedProviderModels = $derived(
    AI_PROVIDERS.find((p) => p.id === $aiProviderConfig.provider)?.models ?? []
  );

  async function handleSaveSettings() {
    settingsSaving = true;
    settingsError = null;
    settingsSaved = false;
    try {
      await saveAiConfigAction($aiProviderConfig.provider, $aiProviderConfig.model, apiKeyInput);
      settingsSaved = true;
      apiKeyInput = "";
    } catch (e) {
      settingsError = e instanceof Error ? e.message : String(e);
    } finally {
      settingsSaving = false;
    }
  }

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
      startPolling(2000);
    });
    const unsubPrefs = initPreferenceSubscriptions();
    return () => {
      stopPolling();
      clearTimeout(debounceTimer);
      unsubPrefs();
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
      <span class="separator"></span>
      <select
        class="profile-select"
        value={$aiProfile}
        onchange={(e) => $aiProfile = (e.target as HTMLSelectElement).value}
        aria-label="AI profile"
      >
        <option value="general">General</option>
        <option value="developer">Developer</option>
        <option value="gaming">Gaming</option>
        <option value="battery">Battery Saver</option>
      </select>
      <button
        class="btn btn-ai"
        onclick={() => analyzeWithAi($aiProviderConfig.provider, $aiProviderConfig.model)}
        disabled={$aiLoading}
      >
        {$aiLoading ? "Analyzing..." : "AI Analyze"}
      </button>
      <button
        class="btn btn-sm"
        onclick={() => showSettings = true}
        title="AI Settings"
      >
        Settings
      </button>
      <span class="separator"></span>
      <button
        class="btn btn-sm"
        onclick={decreaseFontSize}
        title="Decrease font size (Cmd+-)"
        aria-label="Decrease font size"
      >A-</button>
      <span class="font-size-display">{$fontSize}</span>
      <button
        class="btn btn-sm"
        onclick={increaseFontSize}
        title="Increase font size (Cmd+=)"
        aria-label="Increase font size"
      >A+</button>
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
      columns={$columns}
      oninspect={inspectProcess}
    />
  {/if}

  {#if $aiError || $aiSuggestions.length > 0}
    <div class="ai-panel">
      <div class="ai-header">
        <span class="ai-title">AI Suggestions</span>
        <button class="btn btn-sm" onclick={dismissAiSuggestions}>Dismiss</button>
      </div>
      {#if $aiError}
        <div class="ai-error">{$aiError}</div>
      {/if}
      {#each $aiSuggestions as suggestion (suggestion.pid)}
        <div class="ai-row">
          <span class="ai-name">{suggestion.name}</span>
          <span class="ai-pid">PID {suggestion.pid}</span>
          <span class="ai-reason">{suggestion.reason}</span>
          <button
            class="btn btn-kill btn-sm"
            onclick={() => killSingle(suggestion.pid)}
          >Close</button>
        </div>
      {/each}
    </div>
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

{#if showSettings}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="backdrop" onclick={closeSettings} onkeydown={(e) => { if (e.key === "Escape") closeSettings(); }} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="settings-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1">
      <div class="settings-header">
        <h2 class="settings-title" id="settings-title">AI Settings</h2>
        <button class="close-btn" onclick={closeSettings} aria-label="Close settings">&times;</button>
      </div>
      <div class="settings-body">
        <div class="settings-row">
          <label class="settings-label" for="provider-select">Provider</label>
          <select
            id="provider-select"
            class="settings-select"
            value={$aiProviderConfig.provider}
            onchange={(e) => {
              const newProvider = (e.target as HTMLSelectElement).value;
              const providerDef = AI_PROVIDERS.find((p) => p.id === newProvider);
              aiProviderConfig.set({
                provider: newProvider,
                model: providerDef?.models[0] ?? "",
              });
            }}
          >
            {#each AI_PROVIDERS as p}
              <option value={p.id}>{p.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="model-select">Model</label>
          <select
            id="model-select"
            class="settings-select"
            value={$aiProviderConfig.model}
            onchange={(e) => {
              aiProviderConfig.update((c) => ({ ...c, model: (e.target as HTMLSelectElement).value }));
            }}
          >
            {#each selectedProviderModels as m}
              <option value={m}>{m}</option>
            {/each}
          </select>
        </div>
        <div class="settings-row">
          <label class="settings-label" for="api-key-input">API Key</label>
          <input
            id="api-key-input"
            class="settings-input"
            type="password"
            placeholder="Enter {AI_PROVIDERS.find((p) => p.id === $aiProviderConfig.provider)?.label ?? ''} API key"
            bind:value={apiKeyInput}
          />
        </div>
        {#if settingsError}
          <div class="settings-error">{settingsError}</div>
        {/if}
        {#if settingsSaved}
          <div class="settings-success">API key saved to keychain.</div>
        {/if}

        <div class="settings-divider"></div>
        <div class="settings-section-label">Visible Columns</div>
        <div class="settings-columns">
          {#each Object.entries($columns) as [key, visible]}
            <label class="col-toggle">
              <input
                type="checkbox"
                checked={visible}
                onchange={() => columns.update((c) => ({ ...c, [key]: !c[key as keyof typeof c] }))}
              />
              <span>{key.charAt(0).toUpperCase() + key.slice(1)}</span>
            </label>
          {/each}
        </div>
      </div>
      <div class="settings-footer">
        <button
          class="btn btn-ai"
          onclick={handleSaveSettings}
          disabled={settingsSaving || !apiKeyInput}
        >
          {settingsSaving ? "Saving..." : "Save"}
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

  .separator {
    width: 1px;
    height: 14px;
    background: var(--border);
    flex-shrink: 0;
  }

  .font-size-display {
    font-size: 10px;
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
    font-size: 10px;
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
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--accent);
  }

  .ai-error {
    padding: 4px 8px;
    font-size: 10px;
    color: var(--danger);
  }

  .ai-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 8px;
    font-size: 11px;
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
    font-size: 10px;
    color: var(--fg-dim);
    flex-shrink: 0;
  }

  .ai-reason {
    flex: 1;
    font-size: 10px;
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
    font-size: 12px;
    margin: 0;
  }

  .close-btn {
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: 16px;
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
    font-size: 11px;
  }

  .settings-label {
    width: 64px;
    flex-shrink: 0;
    font-size: 10px;
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
    font-size: 11px;
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
    font-size: 11px;
    outline: none;
    height: 22px;
  }
  .settings-input:focus {
    border-color: var(--accent);
  }

  .settings-error {
    font-size: 10px;
    color: var(--danger);
    padding: 2px 0;
  }

  .settings-success {
    font-size: 10px;
    color: var(--green);
    padding: 2px 0;
  }

  .settings-divider {
    height: 1px;
    background: var(--border);
    margin: 6px 0;
  }

  .settings-section-label {
    font-size: 9px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent);
    margin-bottom: 4px;
  }

  .settings-columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px 12px;
  }

  .col-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    cursor: pointer;
  }
  .col-toggle input {
    margin: 0;
    width: 12px;
    height: 12px;
    cursor: pointer;
  }

  .settings-footer {
    padding: 6px 10px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
  }
</style>
