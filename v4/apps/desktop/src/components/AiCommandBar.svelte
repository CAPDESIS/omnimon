<script lang="ts">
  import { get } from "svelte/store";
  import { slide, fade } from "svelte/transition";
  import {
    buildConfigPrompt,
    validateConfigPatch,
    validateAlertRule,
    validateAiRule,
    buildRulesPayload,
    detectPromptInjection,
    type ConfigPatch,
  } from "../lib/aiConfigBridge";
  import type { AlertRule } from "../lib/aiConfigBridge";
  import type { AiRuleV1 } from "../lib/types";
  import { ipcAnalyzeContext, ipcApplyAiRules } from "../lib/ipc";
  import {
    aiProviderConfig,
    idleThreshold,
    fontSize,
    theme,
    localePreference,
    columns,
    columnOrder,
  } from "../stores/preferences";
  import { aiProfile, filtered } from "../stores/processes";
  import { addAlertRule } from "../stores/alerts";
  import { toast } from "../stores/toasts";
  import { t } from "../lib/i18n";
  import type { ThemeId } from "../lib/theme";
  import type { LocaleCode } from "../lib/i18n";

  let input = $state("");
  let loading = $state(false);
  let error = $state<string | null>(null);

  interface ChatMessage {
    role: "user" | "assistant" | "system";
    text: string;
  }

  let messages = $state<ChatMessage[]>([]);
  let chatContainer: HTMLDivElement | undefined = $state();

  // --- Preview state (human-in-the-loop) ---
  interface PendingChange {
    kind: "config" | "alerts" | "ai_rules";
    patch?: ConfigPatch;
    alerts?: AlertRule[];
    aiRules?: AiRuleV1[];
    raw: string; // original AI response for display
  }

  let pendingChange = $state<PendingChange | null>(null);

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (chatContainer) {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }
    });
  }

  function getCurrentConfig(): Record<string, unknown> {
    return {
      idleThreshold: get(idleThreshold),
      fontSize: get(fontSize),
      theme: get(theme),
      locale: get(localePreference),
      aiProfile: get(aiProfile),
      columns: get(columns),
      columnOrder: get(columnOrder),
    };
  }

  function applyPatch(patch: Record<string, unknown>) {
    if ("idleThreshold" in patch && typeof patch.idleThreshold === "number") {
      idleThreshold.set(patch.idleThreshold);
    }
    if ("fontSize" in patch && typeof patch.fontSize === "number") {
      fontSize.set(patch.fontSize);
    }
    if ("theme" in patch && typeof patch.theme === "string") {
      theme.set(patch.theme as ThemeId);
    }
    if ("locale" in patch && typeof patch.locale === "string") {
      localePreference.set(patch.locale as LocaleCode);
    }
    if ("aiProfile" in patch && typeof patch.aiProfile === "string") {
      aiProfile.set(patch.aiProfile);
    }
  }

  async function confirmPendingChange() {
    if (!pendingChange) return;

    if (pendingChange.kind === "config" && pendingChange.patch) {
      applyPatch(pendingChange.patch);
      const keys = Object.keys(pendingChange.patch).join(", ");
      messages = [...messages, {
        role: "assistant",
        text: `Applied: ${keys}`,
      }];
      toast.success("Config Updated", `Changed: ${keys}`);
    }

    if (pendingChange.kind === "alerts" && pendingChange.alerts) {
      for (const rule of pendingChange.alerts) {
        addAlertRule(rule);
      }
      messages = [...messages, {
        role: "assistant",
        text: `Created ${pendingChange.alerts.length} alert rule(s).`,
      }];
      toast.success("Alerts Created", `${pendingChange.alerts.length} rule(s) added`);
    }

    if (pendingChange.kind === "ai_rules" && pendingChange.aiRules) {
      try {
        const payload = buildRulesPayload(pendingChange.aiRules);
        const count = await ipcApplyAiRules(payload);
        messages = [...messages, {
          role: "assistant",
          text: `Applied ${count} security rule(s) to the rules engine.`,
        }];
        toast.success("Rules Applied", `${count} rule(s) sent to the security engine`);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        messages = [...messages, { role: "system", text: `Failed to apply rules: ${msg}` }];
        toast.error("Rules Error", msg);
      }
    }

    pendingChange = null;
    scrollToBottom();
  }

  function rejectPendingChange() {
    messages = [...messages, { role: "system", text: "Change rejected by user." }];
    pendingChange = null;
    scrollToBottom();
  }

  async function handleSubmit() {
    const trimmed = input.trim();
    if (!trimmed || loading) return;

    // Prompt injection detection
    if (detectPromptInjection(trimmed)) {
      error = "Input blocked: detected potential prompt injection.";
      toast.error("Security", "Prompt injection attempt detected and blocked.");
      return;
    }

    messages = [...messages, { role: "user", text: trimmed }];
    input = "";
    loading = true;
    error = null;
    pendingChange = null;

    try {
      const config = getCurrentConfig();
      const prompt = buildConfigPrompt(trimmed, config);
      const cfg = get(aiProviderConfig);

      const raw = await ipcAnalyzeContext(prompt, cfg.provider, cfg.model);

      // Try to parse JSON from AI response
      const jsonMatch = raw.match(/\{[\s\S]*\}/);
      if (!jsonMatch) {
        messages = [...messages, { role: "assistant", text: raw }];
        scrollToBottom();
        return;
      }

      let parsed: Record<string, unknown>;
      try {
        parsed = JSON.parse(jsonMatch[0]);
      } catch {
        messages = [...messages, { role: "assistant", text: raw }];
        scrollToBottom();
        return;
      }

      // Handle AI security rules (rules engine v1) - show preview
      if (Array.isArray(parsed.ai_rules)) {
        const validRules: AiRuleV1[] = [];
        for (const r of parsed.ai_rules) {
          try {
            validRules.push(validateAiRule(r));
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            messages = [...messages, { role: "system", text: `Invalid security rule: ${msg}` }];
          }
        }
        if (validRules.length > 0) {
          pendingChange = { kind: "ai_rules", aiRules: validRules, raw };
          messages = [...messages, { role: "assistant", text: "Proposed security rules (review below):" }];
        }
        scrollToBottom();
        return;
      }

      // Handle alert rules - show preview
      if (Array.isArray(parsed.alerts)) {
        const validRules: AlertRule[] = [];
        for (const r of parsed.alerts) {
          try {
            validRules.push(validateAlertRule(r));
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            messages = [...messages, { role: "system", text: `Invalid alert rule: ${msg}` }];
          }
        }
        if (validRules.length > 0) {
          pendingChange = { kind: "alerts", alerts: validRules, raw };
          messages = [...messages, { role: "assistant", text: "Proposed alert rules (review below):" }];
        }
        scrollToBottom();
        return;
      }

      // Handle config patches - show preview
      const validated = validateConfigPatch(parsed);
      if (Object.keys(validated).length === 0) {
        messages = [...messages, { role: "assistant", text: raw }];
        scrollToBottom();
        return;
      }

      pendingChange = { kind: "config", patch: validated, raw };
      messages = [...messages, { role: "assistant", text: "Proposed configuration change (review below):" }];
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes("Security violation")) {
        error = msg;
        toast.error("Security", msg);
        messages = [...messages, { role: "system", text: msg }];
      } else if (msg.includes("No matching entry") || msg.includes("keyring")) {
        error = t("processes.noApiKey");
      } else {
        error = msg;
        messages = [...messages, { role: "system", text: msg }];
      }
    } finally {
      loading = false;
      scrollToBottom();
    }
  }

  function clearChat() {
    messages = [];
    error = null;
    input = "";
    pendingChange = null;
  }
</script>

<div class="command-bar" role="region" aria-label="AI Command">
  {#if messages.length > 0}
    <div class="chat-messages" bind:this={chatContainer} transition:slide={{ duration: 200 }}>
      {#each messages as msg}
        <div class="chat-msg chat-{msg.role}">
          <span class="chat-role">
            {msg.role === "user" ? "You" : msg.role === "assistant" ? "AI" : "System"}
          </span>
          <span class="chat-text">{msg.text}</span>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Preview / Confirm panel (Human-in-the-loop) -->
  {#if pendingChange}
    <div class="preview-panel" transition:slide={{ duration: 200 }}>
      <div class="preview-header">
        <span class="preview-label">Preview</span>
        <span class="preview-kind">
          {pendingChange?.kind === "config" ? "Configuration Change" : pendingChange?.kind === "ai_rules" ? "Security Rules" : "Alert Rules"}
        </span>
      </div>

      <div class="preview-diff">
        {#if pendingChange?.kind === "config" && pendingChange.patch}
          <table class="diff-table">
            <thead>
              <tr>
                <th>Setting</th>
                <th>Current</th>
                <th>New</th>
              </tr>
            </thead>
            <tbody>
              {#each Object.entries(pendingChange?.patch ?? {}) as [key, newVal]}
                {@const current = getCurrentConfig()[key]}
                <tr>
                  <td class="diff-key">{key}</td>
                  <td class="diff-old">{JSON.stringify(current)}</td>
                  <td class="diff-new">{JSON.stringify(newVal)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}

        {#if pendingChange?.kind === "alerts" && pendingChange.alerts}
          {#each pendingChange?.alerts ?? [] as rule, i}
            <div class="alert-preview-row">
              <span class="alert-preview-num">#{i + 1}</span>
              <span class="alert-preview-rule">
                {rule.processName ? `${rule.processName}: ` : "System "}
                {rule.metric} {rule.operator} {rule.threshold}
              </span>
              <span class="alert-preview-action">{rule.action}</span>
            </div>
          {/each}
        {/if}

        {#if pendingChange?.kind === "ai_rules" && pendingChange.aiRules}
          {#each pendingChange?.aiRules ?? [] as rule, i}
            <div class="alert-preview-row">
              <span class="alert-preview-num">#{i + 1}</span>
              <span class="alert-preview-rule">
                <span class="rule-kind">{rule.kind}</span>
                {rule.name}
                {#if rule.process_contains}
                  <span class="rule-detail">process: {rule.process_contains}</span>
                {/if}
                {#if rule.country_code}
                  <span class="rule-detail">country: {rule.country_code}</span>
                {/if}
                {#if rule.destination_ip}
                  <span class="rule-detail">IP: {rule.destination_ip}</span>
                {/if}
                {#if rule.destination_port}
                  <span class="rule-detail">port: {rule.destination_port}</span>
                {/if}
                {#if rule.process_memory_mb_gt}
                  <span class="rule-detail">mem &gt; {rule.process_memory_mb_gt}MB</span>
                {/if}
              </span>
              <span class="alert-preview-action">{rule.enabled ? "ON" : "OFF"}</span>
            </div>
          {/each}
        {/if}
      </div>

      <div class="preview-actions">
        <button class="btn-confirm" onclick={confirmPendingChange}>
          Apply
        </button>
        <button class="btn-reject" onclick={rejectPendingChange}>
          Reject
        </button>
      </div>
    </div>
  {/if}

  <div class="command-row">
    <div class="command-input-wrap">
      <span class="command-prefix">&gt;</span>
      <input
        class="command-input"
        type="text"
        placeholder='Try: "Alert me if Chrome uses more than 2GB" or "Switch to cyberpunk theme"'
        bind:value={input}
        onkeydown={(e) => { if (e.key === "Enter") handleSubmit(); }}
        disabled={loading}
      />
      {#if input || messages.length > 0}
        <button class="clear-btn" onclick={clearChat} aria-label="Clear">&times;</button>
      {/if}
    </div>
    <button
      class="send-btn"
      onclick={handleSubmit}
      disabled={loading || !input.trim()}
    >
      {loading ? "..." : "Run"}
    </button>
  </div>
  {#if error}
    <div class="command-error">{error}</div>
  {/if}
</div>

<style>
  .command-bar {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-alt);
  }

  .chat-messages {
    max-height: 180px;
    overflow-y: auto;
    padding: 6px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.1));
  }

  .chat-msg {
    display: flex;
    gap: 6px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    line-height: 1.4;
    padding: 3px 0;
  }

  .chat-role {
    font-weight: 700;
    flex-shrink: 0;
    min-width: 36px;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding-top: 1px;
  }

  .chat-user .chat-role { color: var(--accent); }
  .chat-assistant .chat-role { color: var(--green); }
  .chat-system .chat-role { color: var(--yellow); }

  .chat-text {
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* --- Preview Panel --- */
  .preview-panel {
    border-bottom: 1px solid var(--border);
    background: var(--bg-surface, var(--bg));
    padding: 8px 10px;
  }

  .preview-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .preview-label {
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--yellow);
    padding: 1px 5px;
    border: 1px solid var(--yellow);
    border-radius: 3px;
  }

  .preview-kind {
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    font-weight: 600;
    color: var(--fg);
  }

  .preview-diff {
    margin-bottom: 8px;
  }

  .diff-table {
    width: 100%;
    border-collapse: collapse;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .diff-table th {
    text-align: left;
    font-weight: 600;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px 2px 0;
    border-bottom: 1px solid var(--border);
  }

  .diff-table td {
    padding: 3px 8px 3px 0;
    border-bottom: 1px solid var(--border-subtle, rgba(128,128,128,0.08));
  }

  .diff-key {
    color: var(--accent);
    font-weight: 600;
  }

  .diff-old {
    color: var(--fg-dim);
    text-decoration: line-through;
    opacity: 0.7;
  }

  .diff-new {
    color: var(--green);
    font-weight: 600;
  }

  .alert-preview-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .alert-preview-num {
    color: var(--fg-dim);
    font-weight: 600;
    min-width: 20px;
  }

  .alert-preview-rule {
    flex: 1;
    color: var(--fg);
  }

  .alert-preview-action {
    color: var(--accent);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-transform: uppercase;
  }

  .rule-kind {
    display: inline-block;
    padding: 0 4px;
    border-radius: 2px;
    background: var(--accent-dim, rgba(59,130,246,0.15));
    color: var(--accent);
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.3px;
    margin-right: 4px;
  }

  .rule-detail {
    display: inline-block;
    margin-left: 6px;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
  }

  .preview-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .btn-confirm {
    padding: 4px 14px;
    border: none;
    border-radius: var(--radius-sm, 4px);
    background: var(--green);
    color: white;
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: background 0.15s;
  }
  .btn-confirm:hover { filter: brightness(1.1); }

  .btn-reject {
    padding: 4px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: transparent;
    color: var(--fg-dim);
    font-weight: 600;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: background 0.15s, color 0.15s;
  }
  .btn-reject:hover {
    background: var(--danger);
    color: white;
    border-color: var(--danger);
  }

  /* --- Input Row --- */
  .command-row {
    display: flex;
    gap: 4px;
    padding: 6px 10px;
    align-items: center;
  }

  .command-input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    padding: 0 6px;
    height: calc(var(--base-font-size, 12px) * 2);
    transition: border-color 0.15s;
  }
  .command-input-wrap:focus-within {
    border-color: var(--accent);
  }

  .command-prefix {
    color: var(--accent);
    font-weight: 700;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    margin-right: 6px;
    user-select: none;
  }

  .command-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--fg);
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    outline: none;
    height: 100%;
  }
  .command-input::placeholder { color: var(--fg-dim); opacity: 0.6; }
  .command-input:disabled { opacity: 0.5; }

  .clear-btn {
    width: 16px;
    height: 16px;
    border: none;
    background: transparent;
    color: var(--fg-dim);
    font-size: 13px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border-radius: 2px;
  }
  .clear-btn:hover { color: var(--fg); background: var(--bg-hover); }

  .send-btn {
    padding: 0 12px;
    height: calc(var(--base-font-size, 12px) * 2);
    border: none;
    border-radius: var(--radius-sm, 4px);
    background: var(--accent);
    color: white;
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s;
  }
  .send-btn:hover:not(:disabled) { background: var(--accent-hover, #1d4ed8); }
  .send-btn:disabled { opacity: 0.4; cursor: default; }

  .command-error {
    padding: 2px 10px 4px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    color: var(--danger);
  }
</style>
