<script lang="ts">
  import { toasts, dismissToast, type ToastLevel } from "../stores/toasts";
  import { fly, fade } from "svelte/transition";
  import { t } from "../lib/i18n";

  import IconButton from "./IconButton.svelte";

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
  {#each $toasts as toastItem (toastItem.id)}
    <div
      class="toast toast-{toastItem.level}"
      role="alert"
      in:fly={{ y: 20, duration: 250 }}
      out:fade={{ duration: 180 }}
    >
      <span class="toast-icon">{levelIcon(toastItem.level)}</span>
      <div class="toast-content">
        <span class="toast-title">{toastItem.title}</span>
        {#if toastItem.message}
          <span class="toast-message">{toastItem.message}</span>
        {/if}
      </div>
      <IconButton
        class="toast-dismiss"
        onclick={() => dismissToast(toastItem.id)}
        ariaLabel={t("toast.dismiss")}
        title={t("toast.dismiss")}
        size="sm"
      >&times;</IconButton>
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
    line-height: 1;
  }
</style>
