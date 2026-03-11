<script lang="ts">
  import { AlertTriangle } from "lucide-svelte";
  import { slide, fade } from "svelte/transition";

  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";
  import { smartAlerts, dismissSmartAlert, dismissAllSmartAlerts } from "../stores/alerts";
  import { killSingle } from "../stores/processes";
  import { t } from "../lib/i18n";
  import { renderMarkdown } from "../lib/markdown";

  function handleIgnore(id: string) {
    dismissSmartAlert(id);
  }

  async function handleForceQuit(id: string, pid?: number) {
    if (pid) {
      await killSingle(pid);
    }
    dismissSmartAlert(id);
  }

  const MAX_VISIBLE_ALERTS = 5;
  let visibleAlerts = $derived($smartAlerts.slice(-MAX_VISIBLE_ALERTS));
  let hiddenCount = $derived(Math.max(0, $smartAlerts.length - MAX_VISIBLE_ALERTS));
</script>

{#if $smartAlerts.length > 0}
  <div class="smart-alerts-container" transition:fade={{ duration: 200 }}>
    {#if $smartAlerts.length > 1}
      <div class="alerts-global-actions">
        {#if hiddenCount > 0}
          <span class="hidden-count">+{hiddenCount} alertas más</span>
        {/if}
        <Button class="close-all-btn" variant="secondary" size="sm" onclick={dismissAllSmartAlerts}>
          ✕ Cerrar todas
        </Button>
      </div>
    {/if}

    {#each visibleAlerts as alert (alert.id)}
      <div class="smart-alert-card" transition:slide={{ duration: 250 }}>
        <div class="alert-header">
          <span class="icon"><AlertTriangle size={14} /></span>
          <strong>{t("smartAlerts.title")}</strong>
          {#if alert.updateCount && alert.updateCount > 1}
            <span class="update-badge">Actualizada {alert.updateCount}x</span>
          {/if}
          <IconButton class="close-btn" onclick={() => handleIgnore(alert.id)} ariaLabel={t("common.dismiss")} title={t("common.dismiss")} size="sm">✕</IconButton>
        </div>
        
        <div class="alert-body">
          <div class="problem">{@html renderMarkdown(alert.problem)}</div>
          <div class="explanation">{@html renderMarkdown(alert.explanation)}</div>
        </div>

        <div class="alert-actions">
          <Button class="action-btn ignore" variant="ghost" size="sm" onclick={() => handleIgnore(alert.id)}>
            {t("common.ignore")}
          </Button>
          {#if alert.processPid}
            <Button class="action-btn force-quit" variant="danger" size="sm" onclick={() => handleForceQuit(alert.id, alert.processPid)}>
              {t("process.forceQuit")}
            </Button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .smart-alerts-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 380px;
    pointer-events: none; /* Container itself shouldn't block clicks */
  }

  .smart-alert-card {
    pointer-events: auto; /* Re-enable for the card */
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .alerts-global-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 12px;
    margin-bottom: 4px;
    pointer-events: auto;
  }

  .hidden-count {
    font-size: 12px;
    color: var(--fg-dim);
    font-weight: 500;
  }

  .alert-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
  }

  .update-badge {
    font-size: 10px;
    background: var(--blue, #3b82f6);
    color: white;
    padding: 2px 6px;
    border-radius: 10px;
    font-weight: 600;
  }

  .alert-header .icon {
    font-size: 14px;
  }

  .alert-header strong {
    flex: 1;
    font-size: 13px;
    color: var(--yellow);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .alert-body {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .problem {
    font-weight: 600;
    font-size: 13px;
    color: var(--fg);
    margin: 0;
  }

  .explanation {
    font-size: 12px;
    line-height: 1.4;
    color: var(--fg-dim);
    margin: 0;
  }

  .explanation :global(p) { margin: 0 0 4px; }
  .explanation :global(p:last-child) { margin-bottom: 0; }
  .explanation :global(strong) { color: var(--fg); font-weight: 700; }
  .explanation :global(em) { font-style: italic; color: var(--fg-dim); }
  .explanation :global(ul) { margin: 4px 0; padding-left: 18px; list-style: disc; }
  .explanation :global(li) { margin: 2px 0; }
  .explanation :global(pre), .explanation :global(code) { background: var(--bg-hover); padding: 2px 4px; border-radius: 4px; font-family: monospace; }
  .problem :global(strong) { color: var(--fg); font-weight: 700; }

  .alert-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle, #2a2a3a);
    background: var(--bg-alt);
  }

</style>
