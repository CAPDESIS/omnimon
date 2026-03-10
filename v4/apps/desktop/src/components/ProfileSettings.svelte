<script lang="ts">
  import {
    displayName,
    profilePreset,
    dashboardLayout,
    refreshInterval,
    favoriteProcesses,
    notificationLevel,
  } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import Button from "./Button.svelte";

  let newFavorite = $state("");

  function resetToDefaults() {
    $displayName = "User";
    $profilePreset = "balanced";
    $dashboardLayout = "standard";
    $refreshInterval = 2000;
    $favoriteProcesses = [];
    $notificationLevel = "all";
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
      $refreshInterval = 2000;
      $notificationLevel = "all";
    } else if (preset === "power") {
      $dashboardLayout = "expanded";
      $refreshInterval = 1000;
      $notificationLevel = "critical";
    }
  }
</script>

<div class="profile-settings">
  <h2>{t("profileSettings.title")}</h2>

  <div class="form-group">
    <label for="displayName">{t("profileSettings.displayName")}</label>
    <input
      id="displayName"
      type="text"
      bind:value={$displayName}
    />
  </div>

  <div class="form-group presets">
    <h3>{t("profileSettings.preset")}</h3>
    <div class="preset-cards">
      <button 
        class="preset-card" 
        class:active={$profilePreset === 'minimal'} 
        onclick={() => setPreset('minimal')}
      >
        <h4>{t("profileSettings.presetMinimal")}</h4>
        <p>Basic tracking</p>
      </button>
      <button 
        class="preset-card" 
        class:active={$profilePreset === 'balanced'} 
        onclick={() => setPreset('balanced')}
      >
        <h4>{t("profileSettings.presetBalanced")}</h4>
        <p>Standard features</p>
      </button>
      <button 
        class="preset-card" 
        class:active={$profilePreset === 'power'} 
        onclick={() => setPreset('power')}
      >
        <h4>{t("profileSettings.presetPower")}</h4>
        <p>Full tracking & AI</p>
      </button>
    </div>
  </div>

  <div class="form-group">
    <label for="refreshInterval">{t("profileSettings.refreshInterval")} (ms): {$refreshInterval}</label>
    <input
      id="refreshInterval"
      type="range"
      min="1000"
      max="10000"
      step="1000"
      bind:value={$refreshInterval}
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
    <label for="favoriteSearch">{t("profileSettings.favorites")}</label>
    <div class="favorite-input">
      <input
        id="favoriteSearch"
        type="text"
        bind:value={newFavorite}
        placeholder={t("profileSettings.searchFavorite")}
        onkeydown={(e) => e.key === 'Enter' && addFavorite()}
      />
      <Button onclick={addFavorite} aria-label={t("profileSettings.addFavorite")}>{t("profileSettings.addFavorite")}</Button>
    </div>
    <ul class="favorite-list">
      {#each $favoriteProcesses as fav}
        <li>
          <span>{fav}</span>
          <Button onclick={() => removeFavorite(fav)} variant="danger" aria-label="Remove">✕</Button>
        </li>
      {/each}
    </ul>
  </div>

  <div class="actions">
    <Button onclick={resetToDefaults} variant="secondary">{t("profileSettings.resetDefaults")}</Button>
  </div>
</div>

<style>
  .profile-settings {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    color: var(--text-color, #fff);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .preset-cards {
    display: flex;
    gap: 1rem;
  }

  .preset-card {
    flex: 1;
    padding: 1rem;
    border: 1px solid var(--border-color, #444);
    border-radius: 8px;
    background: var(--bg-color-secondary, #222);
    color: inherit;
    cursor: pointer;
    transition: all 0.2s;
  }

  .preset-card.active {
    border-color: var(--primary-color, #007bff);
    background: var(--primary-color-alpha, rgba(0, 123, 255, 0.1));
  }

  .favorite-input {
    display: flex;
    gap: 0.5rem;
  }

  .favorite-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .favorite-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--bg-color-tertiary, #333);
    padding: 0.5rem 1rem;
    border-radius: 4px;
  }

  input, select {
    padding: 0.5rem;
    border-radius: 4px;
    border: 1px solid var(--border-color, #444);
    background: var(--bg-color-secondary, #222);
    color: inherit;
  }

  .actions {
    margin-top: 1rem;
  }
</style>
