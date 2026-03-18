<script lang="ts">
  import { t } from "../../lib/i18n";

  let { 
    activeTab, 
    ontabchange,
    showNetwork = true,
    showBrowser = true
  }: { 
    activeTab: "processes" | "network" | "browser" | "aichat" | "settings";
    ontabchange: (tab: "processes" | "network" | "browser" | "aichat" | "settings") => void;
    showNetwork?: boolean;
    showBrowser?: boolean;
  } = $props();
</script>

<div class="tabs-container" role="tablist">
  <button 
    role="tab" 
    class="tab" 
    class:active={activeTab === "processes"} 
    aria-selected={activeTab === "processes"}
    onclick={() => ontabchange("processes")}
  >
      {t("common.processes", { count: 0 }).split(' ')[0]}
  </button>
  
  {#if showNetwork}
    <button 
      role="tab" 
      class="tab" 
      class:active={activeTab === "network"} 
      aria-selected={activeTab === "network"}
      onclick={() => ontabchange("network")}
    >
      {t("common.networkMap")}
    </button>
  {/if}

  {#if showBrowser}
    <button 
      role="tab" 
      class="tab" 
      class:active={activeTab === "browser"} 
      aria-selected={activeTab === "browser"}
      onclick={() => ontabchange("browser")}
    >
      {t("common.browserTabs")}
    </button>
  {/if}

  <button 
    role="tab" 
    class="tab" 
    class:active={activeTab === "aichat"} 
    aria-selected={activeTab === "aichat"}
    onclick={() => ontabchange("aichat")}
  >
    {t("aiChat.title")}
  </button>
  
  <div class="tab-spacer"></div>
  
  <button 
    role="tab" 
    class="tab tab-settings" 
    class:active={activeTab === "settings"} 
    aria-selected={activeTab === "settings"}
    onclick={() => ontabchange("settings")}
  >
    {t("settings.title")}
  </button>
</div>

<style>
  .tabs-container {
    display: flex;
    flex-direction: row;
    background: var(--bg-secondary, #121214);
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
    padding: 0 8px;
    gap: 2px;
    flex-shrink: 0;
  }
  
  .tab {
    background: transparent;
    border: none;
    color: var(--text-secondary, #888);
    padding: 10px 16px;
    font-size: calc(var(--base-font-size) * 1.1);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
    outline: none;
    user-select: none;
    border-radius: 4px 4px 0 0;
  }
  
  .tab:hover {
    color: var(--text-primary, #ededef);
    background: var(--bg-hover, #1a1a1e);
  }
  
  .tab.active {
    color: var(--accent, #007aff);
    border-bottom: 2px solid var(--accent, #007aff);
    font-weight: 500;
  }
  
  .tab-spacer {
    flex: 1;
  }

  .tab-settings {
    font-weight: 600;
    color: var(--accent);
  }
</style>
