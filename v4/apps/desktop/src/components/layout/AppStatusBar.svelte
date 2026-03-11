<script lang="ts">
  import { t } from "../../lib/i18n";
  import StatusBar from "../StatusBar.svelte";

  let { 
    filteredCount, 
    totalCount, 
    selectedCount, 
    selectedRamMB 
  } = $props();
</script>

<div class="app-statusbar-container">
  <StatusBar />
  <footer class="app-footer">
    <span>
      <span class="version-label">OmniMon v6.0.1</span> &nbsp;&middot;&nbsp;
      {t("footer.processes", { count: filteredCount })}{#if filteredCount !== totalCount}
        &nbsp;{t("footer.filteredFrom", { count: totalCount })}{/if}
      {#if selectedCount > 0}
        <span aria-hidden="true">&nbsp;&middot;&nbsp;</span>{t("footer.selected", { count: selectedCount, ram: selectedRamMB.toFixed(0) })}
      {/if}
    </span>
    <span class="shortcuts" aria-hidden="true">
      <kbd>Cmd+I</kbd> {t("footer.shortcutDetail")} 
      <kbd>Cmd+F</kbd> {t("footer.shortcutSearch")} 
      <kbd>Del</kbd> {t("footer.shortcutClose")}
    </span>
  </footer>
</div>

<style>
  .app-statusbar-container {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .app-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 2px 10px;
    background: var(--bg-secondary, #121214);
    border-top: 1px solid var(--border-subtle, #2a2a3a);
    font-size: calc(var(--base-font-size) * 0.833);
    color: var(--text-secondary, #888);
    height: calc(var(--base-font-size) * 1.5);
    flex-shrink: 0;
  }

  .version-label {
    font-weight: 600;
    color: var(--text-primary, #ededef);
  }

  .shortcuts kbd {
    background: var(--bg-hover, #1a1a1e);
    border: 1px solid var(--border-subtle, #2a2a3a);
    border-radius: 3px;
    padding: 0 3px;
    font-family: inherit;
    font-size: 0.9em;
    margin: 0 2px;
  }
</style>
