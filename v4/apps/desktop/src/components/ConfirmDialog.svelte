<script lang="ts">
  import { confirmDialogState, resolveConfirmDialog } from "../lib/confirm";
  import { t } from "../lib/i18n";
  import Button from "./Button.svelte";
  import ModalShell from "./ModalShell.svelte";
  import { fade, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";

  let dialogEl = $state<HTMLDivElement | undefined>();

  const hasItems = $derived($confirmDialogState.items.length > 0);
  const hasAskAi = $derived(!!$confirmDialogState.onAskAi);
  const dialogWidth = $derived(
    hasItems ? "min(520px, calc(100vw - 32px))" : "min(440px, calc(100vw - 32px))"
  );

  function handleAskAi() {
    const fn = $confirmDialogState.onAskAi;
    resolveConfirmDialog(false);
    fn?.();
  }

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
    <ModalShell titleId="confirm-msg" role="alertdialog" backdropClass="confirm-backdrop" panelClass="confirm-dialog" onclose={() => resolveConfirmDialog(false)} width={dialogWidth} maxHeight="calc(100vh - 64px)">
      <div transition:scale={scaleConfig} bind:this={dialogEl}>
      <p class="confirm-message" id="confirm-msg">{$confirmDialogState.message}</p>

      {#if hasItems}
        <ul class="confirm-items" class:confirm-items--scrollable={$confirmDialogState.items.length > 5}>
          {#each $confirmDialogState.items as item}
            <li class="confirm-item">
              {#if item.icon}
                <img class="confirm-item-icon" src={item.icon} alt="" aria-hidden="true" />
              {:else}
                <div class="confirm-item-icon confirm-item-icon--placeholder">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
                  </svg>
                </div>
              {/if}
              <div class="confirm-item-info">
                <span class="confirm-item-name">{item.label}</span>
                {#if item.detail}
                  <span class="confirm-item-detail">{item.detail}</span>
                {/if}
                {#if item.subItems && item.subItems.length > 0}
                  <div class="confirm-sub-items">
                    <span class="confirm-sub-label">{t("processes.openTabs", { count: item.subItems.length })}</span>
                    {#each item.subItems.slice(0, 5) as sub}
                      <span class="confirm-sub-item">{sub}</span>
                    {/each}
                    {#if item.subItems.length > 5}
                      <span class="confirm-sub-more">+{item.subItems.length - 5} {t("common.more")}</span>
                    {/if}
                  </div>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="confirm-actions">
        {#if hasAskAi}
          <Button variant="ghost" size="sm" class="btn-ask-ai-confirm" onclick={handleAskAi}>
            {t("processes.askAiFirst")}
          </Button>
        {/if}
        <div class="confirm-actions-right">
          <Button variant="secondary" size="sm" onclick={() => resolveConfirmDialog(false)}>
            {t("common.no")}
          </Button>
          <Button variant="primary" size="sm" onclick={() => resolveConfirmDialog(true)}>
            {t("common.yes")}
          </Button>
        </div>
      </div>
      </div>
    </ModalShell>
  </div>
{/if}

<style>
  :global(.confirm-dialog) {
    width: 100%;
    min-height: auto !important;
    padding: 20px 24px 16px;
  }

  .confirm-message {
    margin: 0 0 12px;
    font-size: calc(var(--base-font-size) * 1.05);
    line-height: 1.45;
    color: var(--fg);
  }

  .confirm-items {
    list-style: none;
    margin: 0 0 16px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-radius: var(--radius-md, 8px);
    background: var(--bg-secondary, var(--bg-alt, #1a1a2e));
    padding: 4px;
  }

  .confirm-items--scrollable {
    max-height: 240px;
    overflow-y: auto;
  }

  .confirm-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: var(--radius-sm, 6px);
  }

  .confirm-item:hover {
    background: var(--bg-alt, #222238);
  }

  .confirm-item-icon {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    flex-shrink: 0;
    object-fit: contain;
  }

  .confirm-item-icon--placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-alt, #222238);
    color: var(--fg-muted, #888);
  }

  .confirm-item-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }

  .confirm-item-name {
    font-size: var(--base-font-size, 13px);
    font-weight: 500;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .confirm-item-detail {
    font-size: calc(var(--base-font-size, 13px) * 0.85);
    color: var(--fg-muted, #888);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .confirm-sub-items {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 3px;
    padding-left: 2px;
    border-left: 2px solid var(--accent, #6366f1);
    padding-left: 8px;
  }

  .confirm-sub-label {
    font-size: calc(var(--base-font-size, 13px) * 0.75);
    font-weight: 600;
    color: var(--accent, #6366f1);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .confirm-sub-item {
    font-size: calc(var(--base-font-size, 13px) * 0.8);
    color: var(--fg-dim, #999);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 340px;
  }

  .confirm-sub-more {
    font-size: calc(var(--base-font-size, 13px) * 0.75);
    color: var(--fg-muted, #666);
    font-style: italic;
  }

  .confirm-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .confirm-actions-right {
    display: flex;
    gap: 8px;
  }

  :global(.btn-ask-ai-confirm) {
    font-size: calc(var(--base-font-size, 13px) * 0.85) !important;
    color: var(--accent, #6366f1) !important;
  }
</style>
