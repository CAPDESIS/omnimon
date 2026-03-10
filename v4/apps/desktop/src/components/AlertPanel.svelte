<script lang="ts">
  import {
    alertRules,
    askAiAboutNetworkAlert,
    clearFiredAlerts,
    clearNetworkAlerts,
    firedAlerts,
    investigateNetworkAlert,
    matchesNetworkAlertFilter,
    networkAlertFilter,
    networkAlerts,
    removeAlertRule,
  } from "../stores/alerts";
  import { slide, fade } from "svelte/transition";

  import Button from "./Button.svelte";
  import IconButton from "./IconButton.svelte";

  let showPanel = $state(false);

  let alertCount = $derived($firedAlerts.length);
  let networkAlertCount = $derived($networkAlerts.length);
  let hasRules = $derived($alertRules.length > 0);
  let filteredNetworkAlerts = $derived(
    [...$networkAlerts].filter((alert) => matchesNetworkAlertFilter(alert, $networkAlertFilter)).reverse().slice(0, 25),
  );

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  function togglePanel() {
    showPanel = !showPanel;
  }
</script>

{#if hasRules || alertCount > 0 || networkAlertCount > 0}
  <div class="alert-trigger">
    <Button
      class={`alert-btn ${alertCount + networkAlertCount > 0 ? "has-alerts" : ""}`}
      onclick={togglePanel}
      title="Alerts ({alertCount + networkAlertCount})"
      variant="secondary"
      size="sm"
    >
      <span class="alert-icon">{alertCount + networkAlertCount > 0 ? "\u26A0" : "\u2713"}</span>
      {#if alertCount + networkAlertCount > 0}
        <span class="alert-badge">{alertCount + networkAlertCount}</span>
      {/if}
    </Button>
  </div>
{/if}

{#if showPanel}
  <div class="alert-panel" transition:slide={{ duration: 200 }}>
    <div class="alert-panel-header">
      <span class="alert-panel-title">Alerts</span>
      <div class="alert-panel-actions">
        {#if alertCount > 0}
          <Button class="btn-link" variant="ghost" size="sm" onclick={clearFiredAlerts}>Clear All</Button>
        {/if}
        {#if networkAlertCount > 0}
          <Button class="btn-link" variant="ghost" size="sm" onclick={clearNetworkAlerts}>Clear Network</Button>
        {/if}
        <IconButton class="close-btn" onclick={togglePanel} ariaLabel="Close alerts" title="Close alerts" size="sm">&times;</IconButton>
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
            <IconButton class="rule-remove" onclick={() => removeAlertRule(i)} ariaLabel="Remove rule" title="Remove rule" size="sm">&times;</IconButton>
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

    {#if networkAlertCount > 0}
      <div class="network-section">
        <span class="section-label">Network Alerts</span>
        <div class="network-filter-row">
          <input
            class="network-filter-input"
            placeholder="Buscar proceso, destino o regla"
            value={$networkAlertFilter.query}
            oninput={(event: Event) => {
              const value = (event.target as HTMLInputElement).value;
              networkAlertFilter.update((filter: { severity: "all" | "info" | "warning" | "critical"; query: string }) => ({ ...filter, query: value }));
            }}
          />
          <select
            class="network-filter-select"
            value={$networkAlertFilter.severity}
            onchange={(event: Event) => {
              const value = (event.target as HTMLSelectElement).value as "all" | "info" | "warning" | "critical";
              networkAlertFilter.update((filter: { severity: "all" | "info" | "warning" | "critical"; query: string }) => ({ ...filter, severity: value }));
            }}
          >
            <option value="all">todas</option>
            <option value="info">info</option>
            <option value="warning">warning</option>
            <option value="critical">critical</option>
          </select>
        </div>

        {#each filteredNetworkAlerts as alert (alert.id)}
          <div class="network-alert-row" transition:fade={{ duration: 150 }}>
            <div class="network-alert-copy">
              <span class={`network-alert-severity network-alert-severity-${alert.severity}`}>{alert.severity}</span>
              <span class="fired-time">{formatTime(alert.triggered_at_unix_ms)}</span>
              <strong>{alert.rule_name}</strong>
              <span>{alert.process_name ?? alert.destination ?? alert.message}</span>
              {#if alert.destination}
                <span class="rule-text">{alert.destination}</span>
              {/if}
            </div>
            <div class="network-alert-actions">
              <button class="btn-link" onclick={() => investigateNetworkAlert(alert)}>Investigar</button>
              <button class="btn-link" onclick={() => askAiAboutNetworkAlert(alert)}>Preguntarle a IA</button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .alert-trigger {
    display: inline-flex;
  }

  :global(.alert-btn) {
    position: relative;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    display: flex;
    align-items: center;
    gap: 4px;
    height: calc(var(--base-font-size, 12px) * 1.667);
  }
  :global(.alert-btn.has-alerts) {
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
    width: 380px;
    max-height: 70vh;
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
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--yellow);
  }

  .alert-panel-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.btn-link) {
    color: var(--accent);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-decoration: underline;
  }
  :global(.btn-link:hover) { color: var(--accent-hover, var(--accent)); }

  :global(.close-btn) {
    font-size: 14px;
    padding: 0;
  }

  .section-label {
    display: block;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
    padding: 6px 12px 4px;
  }
  .rules-section, .fired-section, .network-section { border-bottom: 1px solid var(--border-subtle, var(--border)); }
  .rule-row { display: flex; align-items: center; justify-content: space-between; padding: 4px 12px; }
  .rule-row:hover, .network-alert-row:hover { background: var(--bg-hover); }
  .rule-text {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  :global(.rule-remove) {
    font-size: 12px;
    padding: 0;
  }
  :global(.rule-remove:hover) { color: var(--danger); }

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
  .fired-detail { color: var(--fg); }
  .no-alerts { padding: 16px 12px; text-align: center; color: var(--fg-dim); font-size: calc(var(--base-font-size, 12px) * 0.833); }
  .network-filter-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 110px;
    gap: 8px;
    padding: 0 12px 8px;
  }
  .network-filter-input, .network-filter-select {
    min-height: 30px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    padding: 0 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.8);
  }
  .network-alert-row { display: flex; gap: 8px; justify-content: space-between; padding: 8px 12px; }
  .network-alert-copy { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .network-alert-actions { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; flex-shrink: 0; }
  .network-alert-severity {
    display: inline-flex;
    align-self: flex-start;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.08em;
    border-radius: 999px;
    padding: 3px 6px;
    font-weight: 700;
  }
  .network-alert-severity-info { background: rgba(59,130,246,0.14); color: var(--accent); }
  .network-alert-severity-warning { background: rgba(245,158,11,0.14); color: var(--yellow); }
  .network-alert-severity-critical { background: rgba(239,68,68,0.6); color: var(--danger); }
</style>
