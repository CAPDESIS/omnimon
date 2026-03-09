<script lang="ts">
  import { confirmDialogState, resolveConfirmDialog } from "../lib/confirm";
  import { t } from "../lib/i18n";
  import Button from "./Button.svelte";
  import { fade, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";

  let dialogEl = $state<HTMLDivElement | undefined>();

  $effect(() => {
    if ($confirmDialogState.open && dialogEl) {
      const btn = dialogEl.querySelector<HTMLButtonElement>(".ui-button--primary");
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
  <div class="confirm-backdrop" transition:fade={fadeConfig} onmousedown={handleBackdropClick} role="alertdialog" aria-modal="true" aria-labelledby="confirm-msg">
    <div class="confirm-dialog" transition:scale={scaleConfig} bind:this={dialogEl}>
      <p class="confirm-message" id="confirm-msg">{$confirmDialogState.message}</p>
      <div class="confirm-actions">
        <Button variant="secondary" size="sm" onclick={() => resolveConfirmDialog(false)}>
          {t("common.no")}
        </Button>
        <Button variant="primary" size="sm" onclick={() => resolveConfirmDialog(true)}>
          {t("common.yes")}
        </Button>
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
</style>
