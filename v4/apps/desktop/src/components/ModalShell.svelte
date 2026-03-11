<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    titleId: string;
    onclose?: () => void;
    labelledBy?: string;
    role?: "dialog" | "alertdialog";
    width?: string;
    maxHeight?: string;
    backdropClass?: string;
    panelClass?: string;
    closeOnBackdrop?: boolean;
    closeOnEscape?: boolean;
    children?: Snippet;
  }

  let {
    titleId,
    onclose,
    labelledBy,
    role = "dialog",
    width = "min(1120px, calc(100vw - 28px))",
    maxHeight = "calc(100vh - 36px)",
    backdropClass = "",
    panelClass = "",
    closeOnBackdrop = true,
    closeOnEscape = true,
    children,
  }: Props = $props();

  let panelEl = $state<HTMLDivElement | undefined>();

  function handleBackdropMouseDown(event: MouseEvent) {
    if (!closeOnBackdrop) return;
    if (event.target === event.currentTarget) onclose?.();
  }

  function handleBackdropKeydown(event: KeyboardEvent) {
    if (!closeOnEscape) return;
    if (event.key === "Escape") {
      event.preventDefault();
      onclose?.();
    }
  }
</script>

<div
  class={`ui-modal-backdrop ${backdropClass}`.trim()}
  onmousedown={handleBackdropMouseDown}
  onclick={(event) => {
    if (closeOnBackdrop && event.target === event.currentTarget) onclose?.();
  }}
  onkeydown={handleBackdropKeydown}
  role="presentation"
>
  <div
    class={`ui-modal-panel ${panelClass}`.trim()}
    bind:this={panelEl}
    onmousedown={(event) => event.stopPropagation()}
    role={role}
    aria-modal="true"
    aria-labelledby={labelledBy ?? titleId}
    tabindex="-1"
    style={`--modal-width:${width};--modal-max-height:${maxHeight}`}
  >
    {@render children?.()}
  </div>
</div>

<style>
  .ui-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .ui-modal-panel {
    width: var(--modal-width);
    max-height: var(--modal-max-height);
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    background: var(--bg-surface, var(--bg-alt, #121214));
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0, 0, 0, 0.5));
  }
</style>
