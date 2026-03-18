<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import EmptyState from "./EmptyState.svelte";
  import Button from "./Button.svelte";
  import { Settings } from "lucide-svelte";
  import { t } from "../lib/i18n";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

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

<div
  class="automations-backdrop"
  role="presentation"
  onclick={(event: MouseEvent) => {
    if (event.target === event.currentTarget) onclose();
  }}
>
  <div class="automations-dialog" role="dialog" aria-modal="true" aria-labelledby="automations-title">
    <header class="automations-header">
      <h2 id="automations-title">{t("automations.title")}</h2>
      <Button variant="ghost" size="icon" type="button" onclick={onclose} aria-label={t("common.close")}>×</Button>
    </header>

    <div class="automations-body">
      <div class="builder">
        <input class="auto-input" type="text" bind:value={process_pattern} placeholder={t("automations.processNameRegex")} />
        <select class="auto-select" bind:value={metric}>
          <option value="ram">{t("automations.ramMb")}</option>
          <option value="cpu">{t("automations.cpuPct")}</option>
        </select>
        <input class="auto-input" type="number" bind:value={threshold} placeholder={t("automations.threshold")} />
        <input class="auto-input" type="number" bind:value={duration_secs} placeholder={t("automations.durationSeconds")} />
        <select class="auto-select" bind:value={action}>
          <option value="alert">{t("automations.alert")}</option>
          <option value="kill">{t("automations.killProcess")}</option>
        </select>
        <Button variant="primary" type="button" onclick={addRule}>{t("automations.addRule")}</Button>
      </div>

      <div class="rules-list">
        {#if rules.length === 0}
          <EmptyState icon={Settings} title={t("automations.emptyTitle")} description={t("automations.emptyBody")} />
        {:else}
          {#each rules as rule}
            <div class="rule-item">
              <span class="rule-desc">{t("automations.ruleDescription", { process: rule.process_pattern, threshold: rule.threshold, metric: rule.metric, duration: rule.duration_secs, action: rule.action })}</span>
              <Button variant="danger" size="sm" type="button" onclick={() => removeRule(rule.id)}>{t("automations.delete")}</Button>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .automations-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 1000;
  }

  .automations-dialog {
    width: min(800px, 100%);
    max-height: min(80vh, 640px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--bg-primary);
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
  }

  .automations-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .automations-header h2 {
    margin: 0;
    font-size: calc(var(--base-font-size) * 1.2);
    font-weight: 600;
  }

  .automations-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .builder {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .auto-input,
  .auto-select {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: var(--base-font-size);
    flex: 1 1 120px;
    min-width: 100px;
  }

  .rules-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .rule-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 14px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    gap: 12px;
  }

  .rule-desc {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size) * 0.9);
    color: var(--text-primary);
  }

  @media (max-width: 600px) {
    .automations-backdrop {
      padding: 12px;
    }

    .builder {
      flex-direction: column;
    }

    .auto-input,
    .auto-select {
      flex: 1 1 100%;
    }
  }
</style>
