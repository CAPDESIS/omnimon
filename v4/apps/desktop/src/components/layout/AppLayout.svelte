<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    fontSize,
    header,
    main,
    footer,
    modals
  }: {
    fontSize: number;
    header: Snippet;
    main: Snippet;
    footer: Snippet;
    modals?: Snippet;
  } = $props();
</script>

<main style="--base-font-size: {fontSize}px">
  {@render header()}

  <div class="layout-content">
    {@render main()}
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

  .layout-content {
    flex: 1 1 auto;
    overflow-y: auto;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary, #0a0a0b);
  }
</style>
