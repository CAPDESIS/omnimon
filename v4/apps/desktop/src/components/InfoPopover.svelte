<script lang="ts">
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
  <button
    class="info-trigger"
    type="button"
    aria-label={label}
    aria-expanded={open}
    title={label}
    onclick={toggle}
    onblur={() => setTimeout(close, 120)}
  >
    i
  </button>
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

  .info-trigger {
    width: 18px;
    height: 18px;
    border-radius: 999px;
    border: 1px solid var(--accent);
    background: transparent;
    color: var(--accent);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }

  .info-trigger:hover,
  .info-trigger:focus-visible {
    background: rgba(59, 130, 246, 0.08);
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
