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
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--bg-alt, #121214);
    cursor: pointer;
    user-select: none;
    min-height: calc(var(--base-font-size) * 1.8);
  }

  .section-header:hover {
    background: var(--bg-hover, rgba(255,255,255,0.05));
  }

  .section-chevron {
    font-size: calc(var(--base-font-size) * 0.6);
    color: var(--fg-dim, #888);
    transition: transform 0.15s ease;
    display: inline-block;
  }

  .section-chevron.open {
    transform: rotate(90deg);
  }

  .section-label {
    font-size: calc(var(--base-font-size) * 1.05);
    font-weight: 500;
    color: var(--fg, #ededef);
  }

  .config-content {
    padding: 12px;
    background: var(--bg, #0a0a0b);
  }
</style>
