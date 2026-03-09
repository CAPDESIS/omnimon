<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { aiProfile } from "../stores/processes";
  import { userMode, profilePresets, activeProfilePreset, applyProfilePresetById, syncAiProfileToPreset } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import InfoPopover from "./InfoPopover.svelte";

  const profiles = [
    { id: "general", accent: "var(--accent)", icon: "◎" },
    { id: "developer", accent: "var(--green)", icon: "</>" },
    { id: "gaming", accent: "var(--yellow)", icon: "▲" },
    { id: "battery", accent: "var(--danger)", icon: "◌" },
  ] as const;

  function descriptionFor(id: string): string {
    return t(`toolbar.${id === "battery" ? "batteryDesc" : `${id}Desc`}`);
  }

  const userModes = [
    { id: "basic", accent: "var(--yellow)" },
    { id: "pro", accent: "var(--accent)" },
  ] as const;
</script>

<div class="profile-panel" role="group" aria-label={t("toolbar.aiProfile")}>
  <div class="profile-header">
    <div>
      <div class="profile-eyebrow">{t("toolbar.aiProfile")}</div>
      <div class="profile-title">{t(`toolbar.${$aiProfile === "battery" ? "batterySaver" : $aiProfile}`)}</div>
    </div>
    <InfoPopover label={t("toolbar.aiProfile")} content={t("toolbar.profileBehavior")} />
  </div>

  <div class="profile-grid">
    {#each profiles as profile}
      <button
        class="profile-card"
        class:selected={$aiProfile === profile.id}
        style={`--card-accent:${profile.accent}`}
        onclick={() => {
          aiProfile.set(profile.id);
          syncAiProfileToPreset(profile.id as "general" | "developer" | "gaming" | "battery");
        }}
        transition:fly={{ y: 8, duration: 180 }}
      >
        <span class="profile-icon">{profile.icon}</span>
        <span class="profile-name">{t(`toolbar.${profile.id === "battery" ? "batterySaver" : profile.id}`)}</span>
        <span class="profile-desc">{descriptionFor(profile.id)}</span>
      </button>
    {/each}
  </div>

  <div class="workspace-mode" role="group" aria-label="Shared profile presets">
    <div class="workspace-copy">
      <div class="profile-eyebrow">Shared presets</div>
      <div class="workspace-title">{$activeProfilePreset}</div>
      <div class="profile-footnote">Thresholds and intervals stay aligned across UI and CLI.</div>
    </div>
    <select
      class="preset-select"
      value={$activeProfilePreset}
      onchange={(event: Event) => {
        const value = (event.target as HTMLSelectElement).value;
        applyProfilePresetById(value);
        aiProfile.set(value);
      }}
      aria-label="Shared profile presets"
    >
      {#each $profilePresets as preset}
        <option value={preset.id}>{preset.label}</option>
      {/each}
    </select>
  </div>

  <div class="workspace-mode" role="group" aria-label={t("profiles.userMode")}>
    <div class="workspace-copy">
      <div class="profile-eyebrow">{t("profiles.userMode")}</div>
      <div class="workspace-title">{t("profiles.activeMode")}: {$userMode === "basic" ? t("profiles.basic") : t("profiles.pro")}</div>
      <div class="profile-footnote">{t("profiles.activeModeDesc")}</div>
    </div>
    <InfoPopover label={t("profiles.userMode")} content={t("profiles.userModeHelp")} />
  </div>

  <div class="mode-grid">
    {#each userModes as mode}
      <button
        class="mode-card"
        class:selected={$userMode === mode.id}
        style={`--mode-accent:${mode.accent}`}
        onclick={() => userMode.set(mode.id)}
        transition:fly={{ y: 8, duration: 180 }}
      >
        <span class="mode-name">{mode.id === "basic" ? t("profiles.basic") : t("profiles.pro")}</span>
        <span class="mode-desc">{mode.id === "basic" ? t("profiles.basicDesc") : t("profiles.proDesc")}</span>
      </button>
    {/each}
  </div>

  <div class="profile-footnote" transition:fade={{ duration: 180 }}>
    {$userMode === "basic" ? t("profiles.proHint") : t("toolbar.profileBehavior")}
  </div>
</div>

<style>
  .profile-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background:
      radial-gradient(circle at top right, color-mix(in srgb, var(--accent) 16%, transparent), transparent 42%),
      var(--bg-surface, var(--bg-alt));
  }

  .profile-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .profile-eyebrow {
    font-size: calc(var(--base-font-size, 12px) * 0.72);
    color: var(--fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 700;
  }

  .profile-title {
    font-size: calc(var(--base-font-size, 12px) * 1.1);
    font-weight: 800;
  }

  .profile-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 10px;
  }

  .profile-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: rgba(255,255,255,0.02);
    color: var(--fg);
    cursor: pointer;
    text-align: left;
    transition: transform 0.18s ease, border-color 0.18s ease, background 0.18s ease, box-shadow 0.18s ease;
  }

  .profile-card:hover,
  .profile-card:focus-visible {
    transform: translateY(-1px);
    border-color: var(--card-accent);
    box-shadow: 0 10px 24px rgba(0,0,0,0.22);
  }

  .profile-card.selected {
    border-color: var(--card-accent);
    background: color-mix(in srgb, var(--card-accent) 14%, transparent);
  }

  .profile-icon {
    color: var(--card-accent);
    font-weight: 800;
    font-family: "SF Mono", "Menlo", monospace;
  }

  .profile-name {
    font-weight: 700;
  }

  .profile-desc,
  .profile-footnote {
    font-size: calc(var(--base-font-size, 12px) * 0.78);
    line-height: 1.45;
    color: var(--fg-dim);
  }

  .workspace-mode {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    padding: 12px;
    border: 1px solid color-mix(in srgb, var(--border) 85%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg) 90%, white 3%);
  }

  .preset-select {
    min-width: 160px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-alt);
    color: var(--fg);
    padding: 8px 10px;
  }

  .workspace-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .workspace-title {
    font-weight: 800;
    font-size: calc(var(--base-font-size, 12px) * 0.95);
  }

  .mode-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 10px;
  }

  .mode-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background:
      linear-gradient(160deg, color-mix(in srgb, var(--mode-accent) 10%, transparent), transparent 45%),
      color-mix(in srgb, var(--bg-alt) 92%, white 2%);
    text-align: left;
    color: var(--fg);
    cursor: pointer;
    transition: transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
  }

  .mode-card:hover,
  .mode-card:focus-visible {
    transform: translateY(-1px);
    border-color: var(--mode-accent);
    box-shadow: 0 12px 24px rgba(0,0,0,0.18);
  }

  .mode-card.selected {
    border-color: var(--mode-accent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--mode-accent) 45%, transparent);
  }

  .mode-name {
    font-weight: 800;
  }

  .mode-desc {
    font-size: calc(var(--base-font-size, 12px) * 0.78);
    line-height: 1.45;
    color: var(--fg-dim);
  }
</style>
