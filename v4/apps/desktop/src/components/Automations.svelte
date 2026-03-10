<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import EmptyState from "./EmptyState.svelte";
  import { Settings } from "lucide-svelte";
  
  interface AutomationRule {
    id: string;
    process_pattern: string;
    metric: string;
    threshold: number;
    duration_secs: number;
    action: string;
  }

  let rules = $state<AutomationRule[]>([]);
  let process_pattern = $state("");
  let metric = $state("ram");
  let threshold = $state(1024);
  let duration_secs = $state(60);
  let action = $state("alert");

  async function loadRules() {
    rules = await invoke<AutomationRule[]>("get_automation_rules");
  }

  async function addRule() {
    const rule: AutomationRule = {
      id: crypto.randomUUID(),
      process_pattern,
      metric,
      threshold,
      duration_secs,
      action,
    };
    await invoke("add_automation_rule", { rule });
    await loadRules();
  }

  async function removeRule(id: string) {
    await invoke("remove_automation_rule", { id });
    await loadRules();
  }

  onMount(() => {
    loadRules();
  });
</script>

<div class="automations-container">
  <div class="automations-header">
    <h2>Automations Engine</h2>
  </div>
  
  <div class="builder">
    <input type="text" bind:value={process_pattern} placeholder="Process Name (Regex)" />
    
    <select bind:value={metric}>
      <option value="ram">RAM (MB)</option>
      <option value="cpu">CPU (%)</option>
    </select>
    
    <input type="number" bind:value={threshold} placeholder="Threshold" />
    
    <input type="number" bind:value={duration_secs} placeholder="Duration (seconds)" />
    
    <select bind:value={action}>
      <option value="alert">Alert</option>
      <option value="kill">Kill Process</option>
    </select>
    
    <button onclick={addRule}>Add Rule</button>
  </div>

  <div class="rules-list">
    {#if rules.length === 0}
      <EmptyState icon={Settings} title="No Automations" description="Create a rule above to automate actions." />
    {:else}
      {#each rules as rule}
        <div class="rule-item">
          <span>{rule.process_pattern} > {rule.threshold} {rule.metric} for {rule.duration_secs}s -> {rule.action}</span>
          <button onclick={() => removeRule(rule.id)}>Delete</button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .automations-container { padding: 1rem; color: #e2e8f0; }
  .automations-header { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; }
  .builder { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  .rule-item { display: flex; justify-content: space-between; padding: 0.5rem; background: #1e293b; margin-bottom: 0.5rem; }
</style>
