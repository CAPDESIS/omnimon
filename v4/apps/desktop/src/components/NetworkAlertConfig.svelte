<script lang="ts">
  import Button from "./Button.svelte";
  import { t } from "../lib/i18n";
  import { networkAlertRules } from "../stores/preferences";
  import type { NetworkAlertCondition, NetworkAlertDirection, NetworkAlertRule, NetworkAlertSeverity } from "../lib/types";

  type ConditionKind = NetworkAlertCondition["kind"];

  const conditionOptions: ConditionKind[] = [
    "high_bandwidth",
    "new_external_connection",
    "unusual_port",
    "process_network_spike",
    "connection_count_exceeded",
    "suspicious_destination",
  ];

  const severityOptions: NetworkAlertSeverity[] = ["info", "warning", "critical"];
  const directionOptions: NetworkAlertDirection[] = ["upload", "download", "both"];

  let showModal = $state(false);
  let editId = $state<string | null>(null);
  let ruleName = $state("");
  let enabled = $state(true);
  let severity = $state<NetworkAlertSeverity>("warning");
  let cooldownSeconds = $state(30);
  let notifyAi = $state(false);
  let conditionKind = $state<ConditionKind>("high_bandwidth");
  let thresholdMbps = $state(400);
  let direction = $state<NetworkAlertDirection>("upload");
  let process = $state("");
  let excludeKnown = $state(true);
  let suspiciousPorts = $state("4444, 6667, 8443, 31337");
  let processName = $state("chrome");
  let multiplier = $state(5);
  let maxConnections = $state(200);
  let patterns = $state("(^198\\.51\\.100\\.)|malware|botnet");

  function getConditionLabel(kind: ConditionKind): string {
    return t(`networkAlerts.types.${kind}`);
  }

  function getSeverityLabel(level: NetworkAlertSeverity): string {
    return t(`networkAlerts.severities.${level}`);
  }

  function getDirectionLabel(value: NetworkAlertDirection): string {
    return t(`networkAlerts.directions.${value}`);
  }

  function resetForm() {
    editId = null;
    ruleName = "";
    enabled = true;
    severity = "warning";
    cooldownSeconds = 30;
    notifyAi = false;
    conditionKind = "high_bandwidth";
    thresholdMbps = 400;
    direction = "upload";
    process = "";
    excludeKnown = true;
    suspiciousPorts = "4444, 6667, 8443, 31337";
    processName = "chrome";
    multiplier = 5;
    maxConnections = 200;
    patterns = "(^198\\.51\\.100\\.)|malware|botnet";
  }

  function openCreate() {
    resetForm();
    showModal = true;
  }

  function openEdit(rule: NetworkAlertRule) {
    resetForm();
    editId = rule.id;
    ruleName = rule.name;
    enabled = rule.enabled;
    severity = rule.severity;
    cooldownSeconds = rule.cooldown_seconds;
    notifyAi = rule.notify_ai;
    conditionKind = rule.condition.kind;

    switch (rule.condition.kind) {
      case "high_bandwidth":
        thresholdMbps = rule.condition.threshold_mbps;
        direction = rule.condition.direction;
        process = rule.condition.process ?? "";
        break;
      case "new_external_connection":
        excludeKnown = rule.condition.exclude_known;
        break;
      case "unusual_port":
        suspiciousPorts = rule.condition.suspicious_ports.join(", ");
        break;
      case "process_network_spike":
        processName = rule.condition.process_name;
        multiplier = rule.condition.multiplier;
        break;
      case "connection_count_exceeded":
        maxConnections = rule.condition.max_connections;
        process = rule.condition.process ?? "";
        break;
      case "suspicious_destination":
        patterns = rule.condition.patterns.join(", ");
        break;
    }

    showModal = true;
  }

  function buildCondition(): NetworkAlertCondition {
    switch (conditionKind) {
      case "high_bandwidth":
        return {
          kind: "high_bandwidth",
          threshold_mbps: Math.max(0.1, thresholdMbps),
          direction,
          process: process.trim() || null,
        };
      case "new_external_connection":
        return { kind: "new_external_connection", exclude_known: excludeKnown };
      case "unusual_port":
        return {
          kind: "unusual_port",
          suspicious_ports: suspiciousPorts
            .split(",")
            .map((value) => Number.parseInt(value.trim(), 10))
            .filter((value) => Number.isInteger(value) && value >= 1 && value <= 65535),
        };
      case "process_network_spike":
        return {
          kind: "process_network_spike",
          process_name: processName.trim() || "chrome",
          multiplier: Math.max(1.1, multiplier),
        };
      case "connection_count_exceeded":
        return {
          kind: "connection_count_exceeded",
          max_connections: Math.max(1, Math.round(maxConnections)),
          process: process.trim() || null,
        };
      case "suspicious_destination":
        return {
          kind: "suspicious_destination",
          patterns: patterns.split(",").map((value) => value.trim()).filter(Boolean),
        };
    }
  }

  function submitRule() {
    const condition = buildCondition();
    const nextRule: NetworkAlertRule = {
      id: editId ?? `net-rule-${Date.now()}`,
      name: ruleName.trim() || getConditionLabel(conditionKind) || t("networkAlerts.newRuleFallback"),
      enabled,
      condition,
      severity,
      cooldown_seconds: Math.max(0, Math.round(cooldownSeconds)),
      notify_ai: notifyAi,
    };

    networkAlertRules.update((rules: NetworkAlertRule[]) => {
      if (editId) {
        return rules.map((rule: NetworkAlertRule) => (rule.id === editId ? nextRule : rule));
      }
      return [...rules, nextRule];
    });

    showModal = false;
  }

  function removeRule(id: string) {
    networkAlertRules.update((rules: NetworkAlertRule[]) => rules.filter((rule: NetworkAlertRule) => rule.id !== id));
  }

  function toggleRule(id: string) {
    networkAlertRules.update((rules: NetworkAlertRule[]) =>
      rules.map((rule: NetworkAlertRule) => (rule.id === id ? { ...rule, enabled: !rule.enabled } : rule)),
    );
  }

  function conditionSummary(condition: NetworkAlertCondition): string {
    switch (condition.kind) {
      case "high_bandwidth":
        return t("networkAlerts.summaries.highBandwidth", {
          threshold: condition.threshold_mbps,
          direction: getDirectionLabel(condition.direction),
          process: condition.process ?? t("networkAlerts.anyProcess"),
        });
      case "new_external_connection":
        return condition.exclude_known
          ? t("networkAlerts.summaries.newExternalKnown")
          : t("networkAlerts.summaries.newExternalAny");
      case "unusual_port":
        return t("networkAlerts.summaries.unusualPort", {
          ports: condition.suspicious_ports.join(", "),
        });
      case "process_network_spike":
        return t("networkAlerts.summaries.processSpike", {
          process: condition.process_name,
          multiplier: condition.multiplier,
        });
      case "connection_count_exceeded":
        return t("networkAlerts.summaries.connectionCount", {
          max: condition.max_connections,
          process: condition.process ?? t("networkAlerts.anyProcess"),
        });
      case "suspicious_destination":
        return t("networkAlerts.summaries.suspiciousDestination", {
          patterns: condition.patterns.join(", "),
        });
    }
  }
</script>

<section class="network-alert-config">
  <div class="section-header">
    <div>
      <div class="eyebrow">{t("networkAlerts.eyebrow")}</div>
      <h3>{t("networkAlerts.title")}</h3>
    </div>
    <Button variant="primary" size="sm" onclick={openCreate}>{t("networkAlerts.addRule")}</Button>
  </div>

  <div class="rule-list">
    {#each $networkAlertRules as rule (rule.id)}
      <article class="rule-card">
        <div class="rule-topline">
          <label class="toggle-row">
            <input type="checkbox" checked={rule.enabled} onchange={() => toggleRule(rule.id)} />
            <span>{rule.name}</span>
          </label>
          <span class={`severity severity-${rule.severity}`}>{getSeverityLabel(rule.severity)}</span>
        </div>
        <div class="rule-meta">{conditionSummary(rule.condition)}</div>
        <div class="rule-meta">{t("networkAlerts.cooldown", { seconds: rule.cooldown_seconds })}{rule.notify_ai ? t("networkAlerts.aiEnabled") : ""}</div>
        <div class="rule-actions">
          <Button size="sm" onclick={() => openEdit(rule)}>{t("networkAlerts.edit")}</Button>
          <Button size="sm" variant="ghost" onclick={() => removeRule(rule.id)}>{t("networkAlerts.delete")}</Button>
        </div>
      </article>
    {/each}
  </div>
</section>

{#if showModal}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={() => (showModal = false)}
    onkeydown={(event: KeyboardEvent) => {
      if (event.key === "Escape" || event.key === "Enter" || event.key === " ") showModal = false;
    }}
  >
    <div
      class="modal-card"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(event: MouseEvent) => event.stopPropagation()}
      onkeydown={(event: KeyboardEvent) => event.stopPropagation()}
    >
      <h3>{editId ? t("networkAlerts.editRule") : t("networkAlerts.newRule")}</h3>

      <label>
        <span>{t("networkAlerts.name")}</span>
        <input bind:value={ruleName} placeholder={t("networkAlerts.namePlaceholder")} />
      </label>

      <div class="grid two">
        <label>
          <span>{t("networkAlerts.type")}</span>
          <select bind:value={conditionKind}>
            {#each conditionOptions as option}
              <option value={option}>{getConditionLabel(option)}</option>
            {/each}
          </select>
        </label>
        <label>
          <span>{t("networkAlerts.severity")}</span>
          <select bind:value={severity}>
            {#each severityOptions as level}
              <option value={level}>{getSeverityLabel(level)}</option>
            {/each}
          </select>
        </label>
      </div>

      {#if conditionKind === "high_bandwidth"}
        <div class="grid two">
          <label><span>{t("networkAlerts.thresholdMbps")}</span><input type="number" min="0.1" step="0.1" bind:value={thresholdMbps} /></label>
          <label>
            <span>{t("network.direction")}</span>
            <select bind:value={direction}>
              {#each directionOptions as option}
                <option value={option}>{getDirectionLabel(option)}</option>
              {/each}
            </select>
          </label>
        </div>
        <label><span>{t("networkAlerts.processOptional")}</span><input bind:value={process} placeholder="chrome" /></label>
      {/if}

      {#if conditionKind === "new_external_connection"}
        <label class="inline-check"><input type="checkbox" bind:checked={excludeKnown} />{t("networkAlerts.excludeKnown")}</label>
      {/if}

      {#if conditionKind === "unusual_port"}
        <label><span>{t("networkAlerts.ports")}</span><input bind:value={suspiciousPorts} placeholder="4444, 6667, 8443" /></label>
      {/if}

      {#if conditionKind === "process_network_spike"}
        <div class="grid two">
          <label><span>{t("network.process")}</span><input bind:value={processName} placeholder="chrome" /></label>
          <label><span>{t("networkAlerts.multiplier")}</span><input type="number" min="1.1" step="0.1" bind:value={multiplier} /></label>
        </div>
      {/if}

      {#if conditionKind === "connection_count_exceeded"}
        <div class="grid two">
          <label><span>{t("networkAlerts.maxConnections")}</span><input type="number" min="1" step="1" bind:value={maxConnections} /></label>
          <label><span>{t("networkAlerts.processOptional")}</span><input bind:value={process} placeholder="chrome" /></label>
        </div>
      {/if}

      {#if conditionKind === "suspicious_destination"}
        <label><span>{t("networkAlerts.regexPatterns")}</span><input bind:value={patterns} placeholder="malware, (^198\.51\.100\.)" /></label>
      {/if}

      <div class="grid two">
        <label><span>{t("networkAlerts.cooldownLabel")}</span><input type="number" min="0" step="1" bind:value={cooldownSeconds} /></label>
        <label class="inline-check"><input type="checkbox" bind:checked={notifyAi} />{t("networkAlerts.notifyAi")}</label>
      </div>

      <label class="inline-check"><input type="checkbox" bind:checked={enabled} />{t("networkAlerts.enabled")}</label>

      <div class="modal-actions">
        <Button variant="ghost" onclick={() => (showModal = false)}>{t("networkAlerts.cancel")}</Button>
        <Button variant="primary" onclick={submitRule}>{t("networkAlerts.saveRule")}</Button>
      </div>
    </div>
  </div>
{/if}

<style>
  .network-alert-config { display: flex; flex-direction: column; gap: 12px; }
  .section-header { display: flex; justify-content: space-between; gap: 12px; align-items: center; }
  .eyebrow { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--accent); font-weight: 700; }
  h3 { margin: 4px 0 0; font-size: 16px; }
  .rule-list { display: grid; gap: 10px; }
  .rule-card { padding: 12px; border: 1px solid var(--border); border-radius: 14px; background: color-mix(in srgb, var(--bg-surface, var(--bg-alt)) 94%, white 3%); display: flex; flex-direction: column; gap: 8px; }
  .rule-topline { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
  .toggle-row { display: inline-flex; align-items: center; gap: 8px; font-weight: 700; }
  .rule-meta { color: var(--fg-dim); font-size: 12px; }
  .rule-actions { display: flex; gap: 8px; }
  .severity { text-transform: uppercase; font-size: 10px; letter-spacing: 0.08em; padding: 4px 8px; border-radius: 999px; font-weight: 700; }
  .severity-info { background: color-mix(in srgb, var(--accent) 14%, var(--bg)); color: var(--accent); }
  .severity-warning { background: color-mix(in srgb, var(--yellow) 14%, var(--bg)); color: var(--yellow); }
  .severity-critical { background: color-mix(in srgb, var(--danger) 60%, var(--bg)); color: #fff; }
  .modal-backdrop { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.7); display: grid; place-items: center; z-index: 1200; padding: 20px; }
  .modal-card { width: min(720px, 100%); background: var(--bg); border: 1px solid var(--border); border-radius: 20px; padding: 18px; display: flex; flex-direction: column; gap: 12px; }
  .grid { display: grid; gap: 10px; }
  .grid.two { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--fg-dim); }
  .inline-check { flex-direction: row; align-items: center; }
  input, select { width: 100%; min-height: 38px; border-radius: 10px; border: 1px solid var(--border); background: color-mix(in srgb, var(--bg-alt) 92%, white 3%); color: var(--fg); padding: 0 12px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; }
  @media (max-width: 720px) { .grid.two { grid-template-columns: 1fr; } }
</style>
