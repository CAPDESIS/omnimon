<script lang="ts">
  import { confirmDialogState, resolveConfirmDialog } from "../lib/confirm";
  import { t } from "../lib/i18n";
  import Button from "./Button.svelte";
  import ModalShell from "./ModalShell.svelte";
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
  <div transition:fade={fadeConfig}>
    <ModalShell titleId="confirm-msg" role="alertdialog" backdropClass="confirm-backdrop" panelClass="confirm-dialog" onclose={() => resolveConfirmDialog(false)} width="min(440px, calc(100vw - 32px))" maxHeight="calc(100vh - 64px)">
      <div transition:scale={scaleConfig} bind:this={dialogEl}>
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
    </ModalShell>
  </div>
{/if}

<style>
  :global(.confirm-dialog) {
    width: 100%;
    padding: 20px 24px 16px;
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
