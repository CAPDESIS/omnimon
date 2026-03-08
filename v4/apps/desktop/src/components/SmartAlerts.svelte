<script lang="ts">
  import { slide, fade } from "svelte/transition";
  import { smartAlerts, dismissSmartAlert } from "../stores/alerts";
  import { killSingle } from "../stores/processes";
  import { t } from "../lib/i18n";

  function handleIgnore(id: string) {
    dismissSmartAlert(id);
  }

  async function handleForceQuit(id: string, pid?: number) {
    if (pid) {
      await killSingle(pid);
    }
    dismissSmartAlert(id);
  }
</script>

{#if $smartAlerts.length > 0}
  <div class="smart-alerts-container" transition:fade={{ duration: 200 }}>
    {#each $smartAlerts as alert (alert.id)}
      <div class="smart-alert-card" transition:slide={{ duration: 250 }}>
        <div class="alert-header">
          <span class="icon">⚠️</span>
          <strong>{t("smartAlerts.title")}</strong>
          <button class="close-btn" onclick={() => handleIgnore(alert.id)} aria-label={t("common.dismiss")}>✕</button>
        </div>
        
        <div class="alert-body">
          <p class="problem">{alert.problem}</p>
          <p class="explanation">{alert.explanation}</p>
        </div>

        <div class="alert-actions">
          <button class="action-btn ignore" onclick={() => handleIgnore(alert.id)}>
            {t("common.ignore")}
          </button>
          {#if alert.processPid}
            <button class="action-btn force-quit" onclick={() => handleForceQuit(alert.id, alert.processPid)}>
              {t("process.forceQuit")}
            </button>
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

  .alert-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg-alt);
    border-bottom: 1px solid var(--border);
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

  .close-btn {
    background: none;
    border: none;
    color: var(--fg-dim);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .close-btn:hover {
    color: var(--fg);
    background: var(--bg-hover);
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

  .alert-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle, rgba(255,255,255,0.05));
    background: var(--bg-alt);
  }

  .action-btn {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid transparent;
  }

  .action-btn.ignore {
    background: transparent;
    border-color: var(--border);
    color: var(--fg);
  }

  .action-btn.ignore:hover {
    background: var(--bg-hover);
  }

  .action-btn.force-quit {
    background: var(--danger);
    color: #fff;
  }

  .action-btn.force-quit:hover {
    background: var(--danger-hover);
  }
</style>
