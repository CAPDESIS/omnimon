<script lang="ts">
  import { toasts, dismissToast, type ToastLevel } from "../stores/toasts";
  import { fly, fade } from "svelte/transition";

  function levelIcon(level: ToastLevel): string {
    switch (level) {
      case "info": return "\u2139";     // i
      case "success": return "\u2713";  // checkmark
      case "warning": return "\u26A0";  // warning triangle
      case "error": return "\u2717";    // x mark
    }
  }
</script>

<div class="toast-container" aria-live="polite" aria-relevant="additions">
  {#each $toasts as t (t.id)}
    <div
      class="toast toast-{t.level}"
      role="alert"
      in:fly={{ y: 20, duration: 250 }}
      out:fade={{ duration: 180 }}
    >
      <span class="toast-icon">{levelIcon(t.level)}</span>
      <div class="toast-content">
        <span class="toast-title">{t.title}</span>
        {#if t.message}
          <span class="toast-message">{t.message}</span>
        {/if}
      </div>
      <button
        class="toast-dismiss"
        onclick={() => dismissToast(t.id)}
        aria-label="Dismiss"
      >&times;</button>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 32px;
    right: 12px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 360px;
    pointer-events: none;
  }

  .toast {
    pointer-events: all;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    background: var(--toast-bg, var(--bg-alt));
    border: 1px solid var(--toast-border, var(--border));
    border-radius: var(--radius-md, 8px);
    box-shadow: var(--shadow-md, 0 4px 12px rgba(0,0,0,0.3));
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    color: var(--fg);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .toast-info { border-left: 3px solid var(--accent); }
  .toast-success { border-left: 3px solid var(--green); }
  .toast-warning { border-left: 3px solid var(--yellow); }
  .toast-error { border-left: 3px solid var(--danger); }

  .toast-icon {
    font-size: calc(var(--base-font-size, 12px) * 1.167);
    flex-shrink: 0;
    width: 18px;
    text-align: center;
    line-height: 1.3;
  }
  .toast-info .toast-icon { color: var(--accent); }
  .toast-success .toast-icon { color: var(--green); }
  .toast-warning .toast-icon { color: var(--yellow); }
  .toast-error .toast-icon { color: var(--danger); }

  .toast-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .toast-title {
    font-weight: 600;
    line-height: 1.3;
  }

  .toast-message {
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    color: var(--fg-dim);
    line-height: 1.4;
    word-break: break-word;
  }

  .toast-dismiss {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    padding: 0;
    line-height: 1;
  }
  .toast-dismiss:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
</style>
