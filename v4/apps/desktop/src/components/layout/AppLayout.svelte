<script lang="ts">
  import type { Snippet } from "svelte";

  let { 
    fontSize,
    header, 
    sidebar, 
    main, 
    footer,
    modals
  }: { 
    fontSize: number;
    header: Snippet;
    sidebar: Snippet;
    main: Snippet;
    footer: Snippet;
    modals?: Snippet;
  } = $props();
</script>

<main style="--base-font-size: {fontSize}px">
  {@render header()}
  
  <div class="layout-body">
    <aside class="layout-sidebar">
      {@render sidebar()}
    </aside>
    
    <div class="layout-content">
      {@render main()}
    </div>
  </div>

  {@render footer()}
</main>

{#if modals}
  {@render modals()}
{/if}

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-primary, #0a0a0b);
    color: var(--text-primary, #ededef);
  }
  
  .layout-body {
    display: flex;
    flex: 1 1 auto;
    overflow: hidden;
    min-height: 0;
  }
  
  .layout-sidebar {
    width: 320px;
    flex-shrink: 0;
    border-right: 1px solid var(--border-subtle, #2a2a3a);
    overflow-y: auto;
    background: var(--bg-secondary, #121214);
    display: flex;
    flex-direction: column;
  }
  
  .layout-content {
    flex: 1;
    overflow-y: auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary, #0a0a0b);
  }
</style>
