<script lang="ts">
  import { t } from "../../lib/i18n";
  import AiCommandBar from "../AiCommandBar.svelte";

  let { 
    isCollapsed = false,
    ontoggle
  } = $props();
</script>

<div class="ai-config-panel">
  <div class="section-header" role="button" tabindex="0"
    onclick={ontoggle}
    onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); ontoggle(); } }}
    aria-expanded={!isCollapsed}
  >
    <span class="section-chevron" class:open={!isCollapsed}>&#9654;</span>
    <span class="section-label">{t("aiConfig.title")}</span>
  </div>
  
  {#if !isCollapsed}
    <div class="config-content">
      <AiCommandBar />
    </div>
  {/if}
</div>

<style>
  .ai-config-panel {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--bg-secondary, #121214);
    cursor: pointer;
    user-select: none;
    min-height: calc(var(--base-font-size) * 1.8);
  }

  .section-header:hover {
    background: var(--bg-hover, #1a1a1e);
  }

  .section-chevron {
    font-size: calc(var(--base-font-size) * 0.6);
    color: var(--text-secondary, #888);
    transition: transform 0.15s ease;
    display: inline-block;
  }

  .section-chevron.open {
    transform: rotate(90deg);
  }

  .section-label {
    font-size: calc(var(--base-font-size) * 1.05);
    font-weight: 500;
    color: var(--text-primary, #ededef);
  }

  .config-content {
    padding: 12px;
    background: var(--bg-primary, #0a0a0b);
  }
</style>
