<script lang="ts">
  import Button from "./Button.svelte";

  interface Props {
    label: string;
    content: string;
  }

  let { label, content }: Props = $props();
  let open = $state(false);

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
  }
</script>

<div class="info-popover">
  <Button
    class="info-trigger"
    aria-label={label}
    aria-expanded={open}
    title={label}
    variant="ghost"
    size="icon"
    onclick={toggle}
    onblur={() => setTimeout(close, 120)}
  >
    <span class="info-trigger-glyph">i</span>
  </Button>
  {#if open}
    <div class="info-panel" role="tooltip">
      {content}
    </div>
  {/if}
</div>

<style>
  .info-popover {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  :global(.info-trigger) {
    width: 20px;
    min-width: 20px;
    height: 20px;
    border-radius: 999px;
    color: var(--accent);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
    padding: 0;
  }

  :global(.info-trigger:hover),
  :global(.info-trigger:focus-visible) {
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
  }

  .info-trigger-glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
  }

  .info-panel {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    z-index: 40;
    width: min(320px, 60vw);
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-surface, var(--bg-alt));
    color: var(--fg);
    padding: 10px 12px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.3);
    font-size: calc(var(--base-font-size, 12px) * 0.8);
    line-height: 1.5;
  }
</style>
