<script lang="ts">
  import type { ProcessEntry, AiProviderKind } from "../lib/types";
  import type { LocaleCode } from "../lib/i18n";
  import type { ThemeId, ThemeTokens } from "../lib/theme";
  import type { ThemeMode } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import { AI_PROVIDERS } from "../lib/types";
  import AlertPanel from "./AlertPanel.svelte";
  import InfoPopover from "./InfoPopover.svelte";

  interface Props {
    searchValue: string;
    onsearch: (event: Event) => void;
    onclearsearch: () => void;
    selectedCount: number;
    selectedRamMB: number;
    grouping: boolean;
    totalFindings: number;
    aiLoading: boolean;
    aiProfile: string;
    fontSize: number;
    onselectall: () => void;
    onselectnone: () => void;
    onkillselected: () => void;
    ontogglegrouping: () => void;
    onchangepofile: (value: string) => void;
    onanalyze: () => void;
    onopensecurity: () => void;
    ontoggledashboard: () => void;
    dashboardCollapsed: boolean;
    ontoggleautomations: () => void;
    onopenplugins: () => void;
    onopensettings: () => void;
    onopenhelp: () => void;
    ondecreasefont: () => void;
    onincreasefont: () => void;
  }

  let {
    searchValue,
    onsearch,
    onclearsearch,
    selectedCount,
    selectedRamMB,
    grouping,
    totalFindings,
    aiLoading,
    aiProfile,
    fontSize,
    onselectall,
    onselectnone,
    onkillselected,
    ontogglegrouping,
    onchangepofile,
    onanalyze,
    onopensecurity,
    ontoggledashboard,
    dashboardCollapsed,
    ontoggleautomations,
    onopenplugins,
    onopensettings,
    onopenhelp,
    ondecreasefont,
    onincreasefont,
  }: Props = $props();

  function profileLabel(id: string): string {
    return t(`toolbar.${id === "battery" ? "batterySaver" : id}`);
  }

  function profileDescription(id: string): string {
    return t(`toolbar.${id === "battery" ? "batteryDesc" : `${id}Desc`}`);
  }
</script>

<header class="toolbar-shell">
  <div class="toolbar-topline">
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
        oninput={onsearch}
      />
      {#if searchValue}
        <button class="search-clear" onclick={onclearsearch} aria-label={t("toolbar.clearSearch")}>×</button>
      {/if}
    </div>

    <a
      class="btn btn-sponsor"
      href="https://github.com/sponsors/chochy2001"
      target="_blank"
      rel="noopener noreferrer"
      title={t("settings.sponsorTitle")}
    >
      <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
        <path fill-rule="evenodd" d="M4.25 2.5c-1.336 0-2.75 1.164-2.75 3 0 2.15 1.58 4.144 3.365 5.682A20.565 20.565 0 008 13.393a20.561 20.561 0 003.135-2.211C12.92 9.644 14.5 7.65 14.5 5.5c0-1.836-1.414-3-2.75-3-1.373 0-2.609.986-3.029 2.456a.75.75 0 01-1.442 0C6.859 3.486 5.623 2.5 4.25 2.5zM8 14.25l-.345.666-.002-.001-.006-.003-.018-.01a7.643 7.643 0 01-.31-.17 22.075 22.075 0 01-3.434-2.414C2.045 10.731 0 8.35 0 5.5 0 2.836 2.086 1 4.25 1 5.797 1 7.153 1.802 8 3.02 8.847 1.802 10.203 1 11.75 1 13.914 1 16 2.836 16 5.5c0 2.85-2.045 5.231-3.885 6.818a22.08 22.08 0 01-3.744 2.584l-.018.01-.006.003h-.002L8 14.25z"></path>
      </svg>
      <span>{t("settings.sponsor")}</span>
    </a>

    <div class="selection-cluster">
      <div class="btn-group">
        <button class="btn btn-text-icon" class:active={grouping} onclick={ontogglegrouping} title={t("toolbar.toggleGrouping")}>
          <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
            <rect x="1" y="1" width="5" height="5" rx="1"/>
            <rect x="1" y="9" width="5" height="5" rx="1"/>
            <rect x="9" y="1" width="5" height="5" rx="1"/>
            <rect x="9" y="9" width="5" height="5" rx="1"/>
          </svg>
          <span>{t("toolbar.groups")}</span>
        </button>
        <button class="btn" onclick={onselectall}>{t("toolbar.all")}</button>
        <button class="btn" onclick={onselectnone}>{t("toolbar.none")}</button>
      </div>
      <button class="btn btn-kill" onclick={onkillselected} disabled={selectedCount === 0} aria-label={t("toolbar.closeSelected")}>
        {t("toolbar.close")}{#if selectedCount > 0}&nbsp;({selectedCount} · {selectedRamMB.toFixed(0)} MB){/if}
      </button>
    </div>
  </div>

  <div class="toolbar-secondary">
    <div class="profile-strip">
      <div class="profile-copy">
        <div class="profile-eyebrow">{t("toolbar.aiProfile")}</div>
        <div class="profile-desc">{profileDescription(aiProfile)}</div>
      </div>
      <select class="profile-select" value={aiProfile} onchange={(e: Event) => onchangepofile((e.target as HTMLSelectElement).value)} aria-label={t("toolbar.aiProfile")} title={t("toolbar.profileHelp")}>
        <option value="general">{profileLabel("general")}</option>
        <option value="developer">{profileLabel("developer")}</option>
        <option value="gaming">{profileLabel("gaming")}</option>
        <option value="battery">{profileLabel("battery")}</option>
      </select>
      <InfoPopover label={t("toolbar.aiProfile")} content={t("toolbar.profileBehavior")} />
      <button class="btn btn-accent" onclick={onanalyze} disabled={aiLoading}>{aiLoading ? t("toolbar.analyzing") : t("toolbar.aiAnalyze")}</button>
    </div>

    <div class="toolbar-actions">
      <AlertPanel />
      <button class="btn btn-icon btn-text-icon" class:has-findings={totalFindings > 0} onclick={onopensecurity} title={t("toolbar.securityFindings", { count: String(totalFindings) })}>
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor"><path d="M8 0L2 3v5c0 4 2.6 6.5 6 8 3.4-1.5 6-4 6-8V3L8 0zm0 2l4 2v4c0 3-1.9 5-4 6.3C5.9 13 4 11 4 8V4l4-2zm-1 4v3h2V6H7zm0 4v1.5h2V10H7z"/></svg>
        <span>{t("toolbar.security")}</span>
        {#if totalFindings > 0}<span class="findings-badge">{totalFindings}</span>{/if}
      </button>
      <button class="btn btn-icon btn-text-icon" onclick={ontoggledashboard} title={dashboardCollapsed ? t("toolbar.showDashboard") : t("toolbar.hideDashboard")}>
        <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor">
          {#if dashboardCollapsed}
            <path d="M3 4h10v1H3zM3 7h10v1H3zM3 10h10v1H3z"/>
          {:else}
            <path d="M1 1h6v6H1zM9 1h6v6H9zM1 9h6v6H1zM9 9h6v6H9z" fill="none" stroke="currentColor" stroke-width="1.2"/>
          {/if}
        </svg>
        <span>{t("toolbar.dashboard")}</span>
      </button>
      <button class="btn btn-icon btn-text-icon" onclick={ontoggleautomations} title={t("toolbar.automations")}><span>●</span><span>{t("toolbar.automations")}</span></button>
      <button class="btn btn-icon btn-text-icon" onclick={onopenplugins} title={t("toolbar.plugins")}><span>Lua</span><span>{t("toolbar.plugins")}</span></button>
      <button class="btn btn-icon btn-text-icon" onclick={onopensettings} title={t("toolbar.aiSettings")}><span>⚙</span><span>{t("toolbar.aiSettings")}</span></button>
      <button class="btn btn-icon btn-text-icon" onclick={onopenhelp} title={t("toolbar.helpCenter")}><span class="btn-icon-glyph">?</span><span>{t("toolbar.helpCenter")}</span></button>
      <div class="font-controls">
        <button class="btn btn-icon" onclick={ondecreasefont} title={t("toolbar.decreaseFont")} aria-label={t("toolbar.decreaseFontLabel")}>A-</button>
        <span class="font-size-display">{fontSize}</span>
        <button class="btn btn-icon" onclick={onincreasefont} title={t("toolbar.increaseFont")} aria-label={t("toolbar.increaseFontLabel")}>A+</button>
      </div>
    </div>
  </div>
</header>

<style>
  .toolbar-shell {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 10px 12px 12px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--accent) 10%, transparent), transparent 42%),
      var(--bg-alt);
    border-bottom: 1px solid var(--border);
  }

  .toolbar-topline,
  .toolbar-secondary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }

  .search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    max-width: 360px;
    flex: 1 1 280px;
  }

  .search-icon {
    position: absolute;
    left: 8px;
    color: var(--fg-dim);
    pointer-events: none;
  }

  .search {
    width: 100%;
    padding: 10px 30px 10px 30px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg) 90%, white 5%);
    color: var(--fg);
    outline: none;
    transition: border-color 0.18s ease, box-shadow 0.18s ease, transform 0.18s ease;
  }

  .search:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
    transform: translateY(-1px);
  }

  .search-clear {
    position: absolute;
    right: 8px;
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--fg-dim);
    cursor: pointer;
  }

  .selection-cluster,
  .toolbar-actions,
  .profile-strip,
  .font-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .profile-strip {
    flex: 1 1 320px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 92%, white 3%);
  }

  .profile-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 180px;
  }

  .profile-eyebrow {
    font-size: calc(var(--base-font-size, 12px) * 0.72);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
    font-weight: 700;
  }

  .profile-desc {
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    color: var(--fg-dim);
    line-height: 1.35;
  }

  .btn,
  .profile-select {
    transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease, background 0.18s ease;
  }

  .btn:hover,
  .profile-select:hover,
  .btn:focus-visible,
  .profile-select:focus-visible {
    transform: translateY(-1px);
    box-shadow: 0 8px 22px rgba(0,0,0,0.18);
  }

  .btn-sponsor {
    border-color: var(--accent);
    color: var(--accent);
  }

  .btn-icon-glyph {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 11px;
    line-height: 1;
  }

  @media (max-width: 980px) {
    .toolbar-topline,
    .toolbar-secondary {
      align-items: stretch;
    }

    .search-wrapper,
    .profile-strip {
      max-width: 100%;
      width: 100%;
    }
  }
</style>
