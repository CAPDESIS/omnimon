<script lang="ts">
  import { t } from "../lib/i18n";
  import { theme } from "../stores/preferences";
  import { themes, type ThemeId } from "../lib/theme";

  const THEME_OPTIONS: { id: ThemeId; name: string; accent: string; bg: string; border: string }[] = [
    { id: "dark", name: "themeSelector.dark", accent: "#3b82f6", bg: "#0a0a0b", border: "#27272a" },
    { id: "light", name: "themeSelector.light", accent: "#2563eb", bg: "#fafafa", border: "#e4e4e7" },
    { id: "cyberpunk", name: "themeSelector.cyberpunk", accent: "#c026d3", bg: "#0b0014", border: "#2d1b4e" },
    { id: "auto", name: "themeSelector.auto", accent: "#a855f7", bg: "linear-gradient(135deg, #0a0a0b 50%, #fafafa 50%)", border: "#71717a" },
  ];

  let showDropdown = $state(false);

  function selectTheme(id: ThemeId) {
    theme.set(id);
    showDropdown = false;
  }

  function currentName(): string {
    const key = THEME_OPTIONS.find((t) => t.id === $theme)?.name ?? "themeSelector.dark";
    return t(key);
  }

  function toggleDropdown() {
    showDropdown = !showDropdown;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") showDropdown = false;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="theme-selector">
  <button class="selector-btn" onclick={toggleDropdown} aria-label={t("themeSelector.selectTheme")}>
    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="5"></circle>
      <line x1="12" y1="1" x2="12" y2="3"></line>
      <line x1="12" y1="21" x2="12" y2="23"></line>
      <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
      <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
      <line x1="1" y1="12" x2="3" y2="12"></line>
      <line x1="21" y1="12" x2="23" y2="12"></line>
      <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
      <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
    </svg>
    <span class="theme-name">{currentName()}</span>
  </button>

  {#if showDropdown}
    <div class="dropdown">
      {#each THEME_OPTIONS as option}
        <button class="dropdown-item" class:active={$theme === option.id} onclick={() => selectTheme(option.id)}>
          <span class="color-preview" style="background: {option.bg}; border-color: {option.border};">
            <span class="color-dot" style="background: {option.accent};"></span>
          </span>
          {t(option.name)}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theme-selector {
    position: relative;
    display: inline-block;
  }
  .selector-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: transparent;
    border: 1px solid var(--border, #27272a);
    color: var(--fg, #ededef);
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }
  .selector-btn:hover {
    background: var(--bg-hover, #1a1a1e);
    border-color: var(--border, #27272a);
  }
  .theme-name {
    font-weight: 500;
  }
  .dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 0.5rem;
    background: var(--bg-card, #161618);
    border: 1px solid var(--border, #27272a);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    min-width: 150px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    padding: 0.25rem;
  }
  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: transparent;
    border: none;
    color: var(--fg, #ededef);
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }
  .dropdown-item:hover {
    background: var(--bg-hover, #1a1a1e);
  }
  .dropdown-item.active {
    background: var(--bg-hover, #1a1a1e);
    color: var(--accent, #3b82f6);
  }
  .color-preview {
    display: inline-block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid;
    position: relative;
  }
  .color-dot {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
</style>
