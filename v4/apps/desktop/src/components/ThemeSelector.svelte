<script lang="ts">
  import { theme } from "../stores/preferences";
  import { applyTheme, getTheme, themes } from "../lib/themes";

  let showDropdown = $state(false);

  function selectTheme(id: string) {
    theme.set(id as any);
    applyTheme(getTheme(id));
    showDropdown = false;
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
  <button class="selector-btn" onclick={toggleDropdown} aria-label="Select Theme">
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
    <span class="theme-name">{getTheme($theme).name}</span>
  </button>

  {#if showDropdown}
    <div class="dropdown">
      {#each Object.values(themes) as t}
        <button class="dropdown-item" class:active={$theme === t.id} onclick={() => selectTheme(t.id)}>
          <span class="color-preview" style="background: {t.colors.bgPrimary}; border-color: {t.colors.border};">
            <span class="color-dot" style="background: {t.colors.accent};"></span>
          </span>
          {t.name}
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
    border: 1px solid var(--border, var(--border-subtle, #30363d));
    color: var(--text-primary, var(--fg, #e6edf3));
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }
  .selector-btn:hover {
    background: var(--bg-hover, #292e36);
    border-color: var(--border-hover, #484f58);
  }
  .theme-name {
    font-weight: 500;
  }
  .dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 0.5rem;
    background: var(--bg-card, var(--bg-surface, #1c2128));
    border: 1px solid var(--border, #30363d);
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
    color: var(--text-primary, var(--fg, #e6edf3));
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    font-size: 0.85rem;
    transition: all 0.15s ease;
  }
  .dropdown-item:hover {
    background: var(--bg-hover, #292e36);
  }
  .dropdown-item.active {
    background: var(--bg-hover, #292e36);
    color: var(--accent, #58a6ff);
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
