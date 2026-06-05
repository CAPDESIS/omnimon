<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    displayName,
    profilePreset,
    dashboardLayout,
    refreshInterval,
    favoriteProcesses,
    notificationLevel,
    aiPrivacyMode,
    aiDailyLimit,
  } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import Button from "./Button.svelte";

  let newFavorite = $state("");
  // Mirrored string binding for the number input so the user can temporarily
  // clear the field (which makes the store null = "use default") without
  // fighting Svelte's number coercion.
  let aiDailyLimitText = $state<string>(
    $aiDailyLimit === null ? "" : String($aiDailyLimit),
  );
  let aiDailyUsed = $state<number>(0);
  let aiDailyLimitEffective = $state<number>(0);

  $effect(() => {
    // Keep the text field in sync when the store changes externally.
    const current = $aiDailyLimit;
    aiDailyLimitText = current === null ? "" : String(current);
  });

  function onAiDailyLimitInput(ev: Event) {
    const raw = (ev.target as HTMLInputElement).value.trim();
    if (raw === "") {
      aiDailyLimit.set(null);
      return;
    }
    const parsed = Number(raw);
    if (!Number.isFinite(parsed) || !Number.isInteger(parsed) || parsed < 0 || parsed > 100_000) {
      // Ignore invalid input — the user can keep typing to reach a valid value.
      return;
    }
    aiDailyLimit.set(parsed);
  }

  async function refreshDailyUsage() {
    try {
      const [used, limit] = await invoke<[number, number]>("get_ai_daily_usage");
      aiDailyUsed = used;
      aiDailyLimitEffective = limit;
    } catch (e) {
      console.warn("[ProfileSettings] get_ai_daily_usage failed:", e);
    }
  }

  onMount(() => {
    refreshDailyUsage();
  });

  const DEFAULT_DISPLAY_NAME = "";

  function resetToDefaults() {
    $displayName = DEFAULT_DISPLAY_NAME;
    $profilePreset = "balanced";
    $dashboardLayout = "standard";
    $refreshInterval = 500;
    $favoriteProcesses = [];
    $notificationLevel = "all";
    $aiPrivacyMode = false;
    $aiDailyLimit = null;
  }

  function addFavorite() {
    const trimmed = newFavorite.trim();
    if (trimmed && !$favoriteProcesses.includes(trimmed)) {
      $favoriteProcesses = [...$favoriteProcesses, trimmed];
      newFavorite = "";
    }
  }

  function removeFavorite(processName: string) {
    $favoriteProcesses = $favoriteProcesses.filter((p) => p !== processName);
  }

  function setPreset(preset: "minimal" | "balanced" | "power") {
    $profilePreset = preset;
    if (preset === "minimal") {
      $dashboardLayout = "compact";
      $refreshInterval = 5000;
      $notificationLevel = "off";
    } else if (preset === "balanced") {
      $dashboardLayout = "standard";
      $refreshInterval = 500;
      $notificationLevel = "all";
    } else if (preset === "power") {
      $dashboardLayout = "expanded";
      $refreshInterval = 500;
      $notificationLevel = "critical";
    }
  }
</script>

<div class="profile-settings">
  <h2>{$displayName.trim() ? `${t("profileSettings.title")} — ${$displayName}` : t("profileSettings.title")}</h2>

  <div class="form-group full-width presets">
    <h3>{t("profileSettings.preset")}</h3>
    <div class="preset-cards">
      <button
        class="preset-card"
        class:active={$profilePreset === 'minimal'}
        onclick={() => setPreset('minimal')}
      >
        <h4>{t("profileSettings.presetMinimal")}</h4>
        <p>{t("profileSettings.presetMinimalDesc")}</p>
      </button>
      <button
        class="preset-card"
        class:active={$profilePreset === 'balanced'}
        onclick={() => setPreset('balanced')}
      >
        <h4>{t("profileSettings.presetBalanced")}</h4>
        <p>{t("profileSettings.presetBalancedDesc")}</p>
      </button>
      <button
        class="preset-card"
        class:active={$profilePreset === 'power'}
        onclick={() => setPreset('power')}
      >
        <h4>{t("profileSettings.presetPower")}</h4>
        <p>{t("profileSettings.presetPowerDesc")}</p>
      </button>
    </div>
  </div>

  <div class="form-grid">
    <div class="form-group">
      <label for="displayName">{t("profileSettings.displayName")}</label>
      <input
        id="displayName"
        type="text"
        bind:value={$displayName}
        placeholder={t("profileSettings.defaultDisplayName")}
      />
    </div>

    <div class="form-group">
      <label for="dashboardLayout">{t("profileSettings.layout")}</label>
      <select id="dashboardLayout" bind:value={$dashboardLayout}>
        <option value="compact">{t("profileSettings.layoutCompact")}</option>
        <option value="standard">{t("profileSettings.layoutStandard")}</option>
        <option value="expanded">{t("profileSettings.layoutExpanded")}</option>
      </select>
    </div>

    <div class="form-group">
      <label for="notificationLevel">{t("profileSettings.notifications")}</label>
      <select id="notificationLevel" bind:value={$notificationLevel}>
        <option value="off">{t("profileSettings.notifOff")}</option>
        <option value="critical">{t("profileSettings.notifCritical")}</option>
        <option value="all">{t("profileSettings.notifAll")}</option>
      </select>
    </div>

    <div class="form-group">
      <label for="refreshInterval">{t("profileSettings.refreshInterval")}: {$refreshInterval}ms</label>
      <input
        id="refreshInterval"
        type="range"
        min="500"
        max="10000"
        step="500"
        bind:value={$refreshInterval}
      />
    </div>
  </div>

  <div class="form-group full-width">
    <label for="favoriteSearch">{t("profileSettings.favorites")}</label>
    <div class="favorite-input">
      <input
        id="favoriteSearch"
        type="text"
        bind:value={newFavorite}
        placeholder={t("profileSettings.searchFavorite")}
        onkeydown={(e: KeyboardEvent) => e.key === 'Enter' && addFavorite()}
      />
      <Button onclick={addFavorite} aria-label={t("profileSettings.addFavorite")}>{t("profileSettings.addFavorite")}</Button>
    </div>
    {#if $favoriteProcesses.length > 0}
      <ul class="favorite-list">
        {#each $favoriteProcesses as fav}
          <li>
            <span>{fav}</span>
            <Button onclick={() => removeFavorite(fav)} variant="danger" aria-label={t("profileSettings.removeFavorite")}>✕</Button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="form-group full-width ai-privacy">
    <h3>{t("profileSettings.aiPrivacySection")}</h3>

    <label class="toggle-row">
      <input
        id="aiPrivacyMode"
        type="checkbox"
        bind:checked={$aiPrivacyMode}
      />
      <span class="toggle-label">{t("profileSettings.aiPrivacyMode")}</span>
    </label>
    <p class="help">{t("profileSettings.aiPrivacyModeHelp")}</p>

    <label class="limit-row" for="aiDailyLimit">
      <span class="toggle-label">{t("profileSettings.aiDailyLimit")}</span>
      <input
        id="aiDailyLimit"
        type="number"
        min="0"
        max="100000"
        step="1"
        placeholder={t("profileSettings.aiDailyLimitDefault")}
        value={aiDailyLimitText}
        oninput={onAiDailyLimitInput}
      />
    </label>
    <p class="help">
      {t("profileSettings.aiDailyLimitHelp")} ·
      {t("profileSettings.aiDailyLimitUsage", {
        used: aiDailyUsed,
        limit: aiDailyLimitEffective === 0 ? "∞" : String(aiDailyLimitEffective),
      })}
      <button type="button" class="link-btn" onclick={refreshDailyUsage}>
        {t("profileSettings.aiDailyLimitRefresh")}
      </button>
    </p>
  </div>

  <div class="actions full-width">
    <Button onclick={resetToDefaults} variant="secondary">{t("profileSettings.resetDefaults")}</Button>
  </div>
</div>

<style>
  .profile-settings {
    display: flex;
    flex-direction: column;
    gap: 20px;
    color: var(--text-primary);
    width: 100%;
  }

  .profile-settings h2 {
    font-size: calc(var(--base-font-size, 12px) * 1.25);
    font-weight: 700;
    margin: 0;
    color: var(--text-primary);
  }

  .profile-settings h3 {
    font-size: calc(var(--base-font-size, 12px) * 0.85);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: var(--text-secondary);
    margin: 0 0 4px;
  }

  /* Responsive 2-column grid for form fields */
  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    width: 100%;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }

  .full-width {
    grid-column: 1 / -1;
  }

  .form-group > label {
    font-size: calc(var(--base-font-size, 12px) * 0.9);
    font-weight: 600;
    color: var(--text-secondary);
  }

  /* Preset cards — full width, 3 columns */
  .preset-cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    width: 100%;
  }

  .preset-card {
    padding: 14px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    background: var(--bg-card, var(--bg-secondary));
    color: var(--text-primary);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    text-align: center;
  }

  .preset-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .preset-card.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 18%, var(--bg-secondary));
  }

  .preset-card h4 {
    margin: 0 0 4px;
    font-size: calc(var(--base-font-size, 12px) * 1.05);
    font-weight: 700;
    color: var(--accent);
  }

  .preset-card:not(.active) h4 {
    color: var(--text-primary);
  }

  .preset-card p {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.85);
    color: var(--text-secondary);
  }

  .favorite-input {
    display: flex;
    gap: 8px;
    width: 100%;
  }

  .favorite-input input {
    flex: 1;
  }

  .favorite-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 4px;
  }

  .favorite-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: var(--base-font-size, 12px);
    color: var(--text-primary);
  }

  input, select {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-size: var(--base-font-size, 12px);
    font-family: inherit;
    width: 100%;
  }

  input:hover, select:hover {
    border-color: var(--accent);
  }

  input:focus-visible, select:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }

  input[type="range"] {
    padding: 0;
    accent-color: var(--accent);
    height: 6px;
    cursor: pointer;
  }

  select {
    cursor: pointer;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23a1a1aa' stroke-width='2'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    padding-right: 32px;
  }

  .actions {
    margin-top: 4px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .ai-privacy {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
    user-select: none;
    cursor: pointer;
  }

  .toggle-row input[type="checkbox"] {
    width: auto;
  }

  .toggle-label {
    font-weight: 600;
    color: var(--text-primary);
  }

  .limit-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .limit-row input[type="number"] {
    max-width: 140px;
    text-align: right;
  }

  .help {
    margin: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.85);
    color: var(--text-secondary);
  }

  .link-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: inherit;
    padding: 0 4px;
    text-decoration: underline;
  }

  /* Responsive: collapse to 1 column on narrow windows */
  @media (max-width: 500px) {
    .form-grid {
      grid-template-columns: 1fr;
    }
    .preset-cards {
      grid-template-columns: 1fr;
    }
  }
</style>
