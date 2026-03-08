<script lang="ts">
  import { confirmDialogState, resolveConfirmDialog } from "../lib/confirm";
  import { t } from "../lib/i18n";

  let dialogEl = $state<HTMLDivElement | undefined>();

  $effect(() => {
    if ($confirmDialogState.open && dialogEl) {
      const btn = dialogEl.querySelector<HTMLButtonElement>(".confirm-primary");
      btn?.focus();
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (!$confirmDialogState.open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      resolveConfirmDialog(false);
    } else if (e.key === "Enter") {
      e.preventDefault();
      resolveConfirmDialog(true);
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      resolveConfirmDialog(false);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $confirmDialogState.open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="confirm-backdrop" onmousedown={handleBackdropClick} role="alertdialog" aria-modal="true" aria-labelledby="confirm-msg">
    <div class="confirm-dialog" bind:this={dialogEl}>
      <p class="confirm-message" id="confirm-msg">{$confirmDialogState.message}</p>
      <div class="confirm-actions">
        <button class="confirm-btn confirm-cancel" onclick={() => resolveConfirmDialog(false)}>
          {t("common.no")}
        </button>
        <button class="confirm-btn confirm-primary" onclick={() => resolveConfirmDialog(true)}>
          {t("common.yes")}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .confirm-dialog {
    background: var(--bg-surface, var(--bg-alt));
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    padding: 20px 24px 16px;
    min-width: 320px;
    max-width: 440px;
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0, 0, 0, 0.5));
  }

  .confirm-message {
    margin: 0 0 16px;
    font-size: calc(var(--base-font-size) * 1.05);
    line-height: 1.45;
    color: var(--fg);
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .confirm-btn {
    padding: 6px 16px;
    border-radius: var(--radius-sm, 4px);
    font-size: calc(var(--base-font-size) * 0.917);
    font-weight: 600;
    cursor: pointer;
    border: 1px solid var(--border);
    transition: background 0.12s, border-color 0.12s;
  }

  .confirm-cancel {
    background: var(--bg);
    color: var(--fg);
  }
  .confirm-cancel:hover {
    background: var(--bg-hover);
  }

  .confirm-primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .confirm-primary:hover {
    background: var(--accent-hover, #2563eb);
  }
</style>
