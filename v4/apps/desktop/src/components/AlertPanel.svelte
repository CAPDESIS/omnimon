<script lang="ts">
  import { firedAlerts, clearFiredAlerts, alertRules, removeAlertRule } from "../stores/alerts";
  import { slide, fade } from "svelte/transition";

  let showPanel = $state(false);

  let alertCount = $derived($firedAlerts.length);
  let hasRules = $derived($alertRules.length > 0);

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  function togglePanel() {
    showPanel = !showPanel;
  }
</script>

{#if hasRules || alertCount > 0}
  <div class="alert-trigger">
    <button
      class="alert-btn"
      class:has-alerts={alertCount > 0}
      onclick={togglePanel}
      title="Alerts ({alertCount})"
    >
      <span class="alert-icon">{alertCount > 0 ? "\u26A0" : "\u2713"}</span>
      {#if alertCount > 0}
        <span class="alert-badge">{alertCount}</span>
      {/if}
    </button>
  </div>
{/if}

{#if showPanel}
  <div class="alert-panel" transition:slide={{ duration: 200 }}>
    <div class="alert-panel-header">
      <span class="alert-panel-title">Alerts</span>
      <div class="alert-panel-actions">
        {#if alertCount > 0}
          <button class="btn-link" onclick={clearFiredAlerts}>Clear All</button>
        {/if}
        <button class="close-btn" onclick={togglePanel}>&times;</button>
      </div>
    </div>

    {#if $alertRules.length > 0}
      <div class="rules-section">
        <span class="section-label">Active Rules</span>
        {#each $alertRules as rule, i}
          <div class="rule-row">
            <span class="rule-text">
              {rule.processName ? `${rule.processName}: ` : "System "}
              {rule.metric} {rule.operator} {rule.threshold}
            </span>
            <button class="rule-remove" onclick={() => removeAlertRule(i)}>&times;</button>
          </div>
        {/each}
      </div>
    {/if}

    {#if $firedAlerts.length > 0}
      <div class="fired-section">
        <span class="section-label">Recent Alerts</span>
        {#each [...$firedAlerts].reverse().slice(0, 20) as alert (alert.id)}
          <div class="fired-row" transition:fade={{ duration: 150 }}>
            <span class="fired-time">{formatTime(alert.timestamp)}</span>
            <span class="fired-detail">
              {alert.processName ?? "System"}: {alert.rule.metric} = {alert.value.toFixed(1)}
              (threshold: {alert.rule.operator} {alert.rule.threshold})
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="no-alerts">No alerts fired yet.</div>
    {/if}
  </div>
{/if}

<style>
  .alert-trigger {
    display: inline-flex;
  }

  .alert-btn {
    position: relative;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    height: calc(var(--base-font-size, 12px) * 1.667);
  }
  .alert-btn:hover { background: var(--bg-hover); }
  .alert-btn.has-alerts {
    border-color: var(--yellow);
    color: var(--yellow);
  }

  .alert-icon { font-size: calc(var(--base-font-size, 12px) * 0.917); }

  .alert-badge {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    border-radius: 7px;
    background: var(--danger);
    color: white;
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 3px;
  }

  .alert-panel {
    position: fixed;
    top: 60px;
    right: 12px;
    width: 360px;
    max-height: 400px;
    overflow-y: auto;
    background: var(--bg-surface, var(--bg-alt));
    border: 1px solid var(--border);
    border-radius: var(--radius-md, 8px);
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0,0,0,0.5));
    z-index: 200;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
  }

  .alert-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .alert-panel-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--yellow);
  }

  .alert-panel-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-link {
    border: none;
    background: none;
    color: var(--accent);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
  .btn-link:hover { color: var(--accent-hover, var(--accent)); }

  .close-btn {
    width: 18px;
    height: 18px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    font-size: 14px;
    padding: 0;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--fg); }

  .section-label {
    display: block;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
    padding: 6px 12px 4px;
  }

  .rules-section, .fired-section {
    border-bottom: 1px solid var(--border-subtle, var(--border));
  }

  .rule-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 12px;
  }
  .rule-row:hover { background: var(--bg-hover); }

  .rule-text {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .rule-remove {
    width: 16px;
    height: 16px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    cursor: pointer;
    border-radius: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    padding: 0;
  }
  .rule-remove:hover { color: var(--danger); background: var(--bg-hover); }

  .fired-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 12px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }
  .fired-row:hover { background: var(--bg-hover); }

  .fired-time {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    color: var(--fg-dim);
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
  }

  .fired-detail {
    color: var(--fg);
  }

  .no-alerts {
    padding: 16px 12px;
    text-align: center;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }
</style>
