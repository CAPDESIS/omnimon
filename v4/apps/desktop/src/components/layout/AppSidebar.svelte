<script lang="ts">
  import { t } from "../../lib/i18n";
  import ProfilePanel from "../ProfilePanel.svelte";
  import SystemDashboard from "../SystemDashboard.svelte";
  import SmartAlerts from "../SmartAlerts.svelte";

  let { 
    dashboardCollapsed, 
    userMode, 
    onopenmetric 
  } = $props();

  let profileCollapsed = $state(false);
</script>

<div class="sidebar-container">
  <!-- Smart Alerts -->
  <div class="sidebar-section alerts-section">
    <SmartAlerts />
  </div>

  <!-- Dashboard with charts -->
  <div class="sidebar-section">
    <SystemDashboard 
      collapsed={dashboardCollapsed} 
      mode={userMode} 
      onopenmetric={onopenmetric} 
    />
  </div>

  <!-- AI Profile Panel -->
  <div class="sidebar-section">
    <div class="section-header" role="button" tabindex="0"
      onclick={() => profileCollapsed = !profileCollapsed}
      onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); profileCollapsed = !profileCollapsed; } }}
      aria-expanded={!profileCollapsed}
    >
      <span class="section-chevron" class:open={!profileCollapsed}>&#9654;</span>
      <span class="section-label">{t("toolbar.aiProfile")}</span>
    </div>
    {#if !profileCollapsed}
      <div class="profiles-shell">
        <ProfilePanel />
      </div>
    {/if}
  </div>
</div>

<style>
  .sidebar-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 8px 0;
    gap: 8px;
  }

  .sidebar-section {
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
    padding-bottom: 8px;
  }

  .alerts-section {
    padding: 0 8px;
    border-bottom: none;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    background: var(--bg-alt, #121214);
    cursor: pointer;
    user-select: none;
    min-height: calc(var(--base-font-size) * 1.8);
    flex-shrink: 0;
  }

  .section-header:hover {
    background: var(--bg-hover, rgba(255,255,255,0.05));
  }

  .section-chevron {
    font-size: calc(var(--base-font-size) * 0.6);
    color: var(--fg-dim, #888);
    transition: transform 0.15s ease;
    display: inline-block;
  }

  .section-chevron.open {
    transform: rotate(90deg);
  }

  .section-label {
    font-size: calc(var(--base-font-size) * 0.95);
    font-weight: 500;
    color: var(--fg, #ededef);
  }

  .profiles-shell {
    padding: 4px 8px;
  }
</style>
