<script lang="ts">
  import { get } from "svelte/store";
  import { slide } from "svelte/transition";
  import { ipcAiChat, ipcGetBrowserTabs, ipcCloseBrowserTab } from "../lib/ipc";
  import { aiProviderConfig } from "../stores/preferences";
  import { processes } from "../stores/processes";
  import { inspectProcessRequest } from "../stores/uiActions";
  import { toast } from "../stores/toasts";
  import { detectPromptInjection } from "../lib/aiConfigBridge";
  import { t } from "../lib/i18n";
  import type { ToolResult } from "../lib/types";

  interface ChatMessage {
    role: "user" | "assistant" | "system" | "tool";
    text: string;
    toolResult?: ToolResult;
  }

  let input = $state("");
  let loading = $state(false);
  let messages = $state<ChatMessage[]>([]);
  let chatContainer: HTMLDivElement | undefined = $state();
  let pendingAction = $state<{ tool: string; details: string; result: ToolResult } | null>(null);
  let requestToken = 0;

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (chatContainer) {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }
    });
  }

  function showProcessDetail(pid: number) {
    const proc = get(processes).find(p => p.pid === pid);
    if (proc) {
      inspectProcessRequest.set(proc);
    }
  }

  function renderWithClickablePids(text: string): string {
    // Match "PID XXXXX" patterns and make them clickable
    return text.replace(/PID\s+(\d+)/g, (match, pid) =>
      `<button class="pid-link" data-pid="${pid}">${match}</button>`
    );
  }

  function handleChatClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.classList.contains('pid-link')) {
      const pid = parseInt(target.dataset.pid || '0', 10);
      if (pid > 0) showProcessDetail(pid);
    }
  }

  async function handleSubmit() {
    const trimmed = input.trim();
    if (!trimmed || loading) return;

    const token = ++requestToken;

    if (detectPromptInjection(trimmed)) {
      toast.error("Security", "Prompt injection attempt blocked.");
      return;
    }

    messages = [...messages, { role: "user", text: trimmed }];
    input = "";
    loading = true;
    scrollToBottom();

    try {
      const cfg = get(aiProviderConfig);
      // Build conversation history (last 10 messages max to avoid token overflow)
      const history: Array<[string, string]> = messages
        .slice(0, -1)
        .filter(m => m.role === "user" || m.role === "assistant")
        .slice(-10)
        .map(m => [m.role, m.text.slice(0, 2000)] as [string, string]);
      // Race the AI call against a 45-second timeout
      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error(t("aiChat.timeoutError"))), 45000)
      );
      const response = await Promise.race([
        ipcAiChat(trimmed, cfg.provider, cfg.model, history),
        timeoutPromise,
      ]);

      if (token !== requestToken) return;

      messages = [...messages, { role: "assistant", text: response.reply }];

      if (response.tool_call) {
        const result = response.tool_call;

        // For destructive actions, require confirmation
        if (result.tool === "close_tabs" || result.tool === "kill_process" || result.tool === "kill_by_name") {
          pendingAction = { tool: result.tool, details: result.details, result };
          // Don't execute yet - wait for user confirmation
        } else {
          messages = [
            ...messages,
            { role: "tool", text: result.details, toolResult: result },
          ];
          if (result.success) {
            toast.success("Action", result.details);
          } else {
            toast.error("Action Failed", result.details);
          }
        }
      }
    } catch (e) {
      if (token !== requestToken) return;
      const msg = e instanceof Error ? e.message : String(e);
      messages = [...messages, { role: "system", text: msg }];

      if (msg.includes("No API key") || msg.includes("keyring")) {
        toast.error("Config", "Set up an AI provider in Settings first.");
      }
    } finally {
      if (token === requestToken) {
        loading = false;
        scrollToBottom();
      }
    }
  }

  function formatActionDetails(tool: string, details: string): string {
    if (details.startsWith("close_tabs_except:")) {
      const patterns = details.replace("close_tabs_except:", "").split("|").join(", ");
      return `Close ALL tabs EXCEPT those matching: ${patterns}`;
    }
    if (details.startsWith("close_tabs:")) {
      const patterns = details.replace("close_tabs:", "").split("|").join(", ");
      return `Close tabs matching: ${patterns}`;
    }
    return details;
  }

  async function executeCloseTabs(details: string): Promise<{ closed: number; message: string }> {
    const isExcept = details.startsWith("close_tabs_except:");
    const raw = details.replace(/^close_tabs(_except)?:/, "").trim();
    if (!raw) return { closed: 0, message: "No pattern provided" };

    try {
      const allTabs = await ipcGetBrowserTabs();
      const patterns = raw.split("|").map(p => p.trim().toLowerCase());

      let toClose;
      if (isExcept) {
        // Close everything EXCEPT tabs matching the patterns
        toClose = allTabs.filter(tab => {
          const url = tab.url.toLowerCase();
          const title = tab.title.toLowerCase();
          return !patterns.some(p => url.includes(p) || title.includes(p));
        });
      } else {
        // Close tabs that MATCH the patterns
        toClose = allTabs.filter(tab => {
          const url = tab.url.toLowerCase();
          const title = tab.title.toLowerCase();
          return patterns.some(p => url.includes(p) || title.includes(p));
        });
      }

       if (toClose.length === 0) {
         return { closed: 0, message: `No tabs matched: ${patterns.join(", ")}` };
       }

      let closed = 0;
      const failed: string[] = [];
      for (const tab of toClose) {
        try {
          await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
          closed++;
        } catch {
          failed.push(tab.title || tab.url);
        }
      }

      const msg = closed > 0
        ? `Closed ${closed} tab(s)${failed.length > 0 ? `, ${failed.length} failed` : ""}`
        : `Failed to close ${failed.length} tab(s)`;
      return { closed, message: msg };
    } catch (e) {
      return { closed: 0, message: `Error: ${e instanceof Error ? e.message : String(e)}` };
    }
  }

  async function confirmAction() {
    if (!pendingAction) return;
    loading = true;
    const { result } = pendingAction;

    if (result.tool === "close_tabs" && result.success) {
      const executed = await executeCloseTabs(result.details);
      result.details = executed.message;
      result.success = executed.closed > 0;
    }
    // kill_process and kill_by_name are already executed by backend

    messages = [
      ...messages,
      { role: "tool", text: result.details, toolResult: result },
    ];

    if (result.success) {
      toast.success("Action", result.details);
    } else {
      toast.error("Action Failed", result.details);
    }

    pendingAction = null;
    loading = false;
    scrollToBottom();
  }

  function rejectAction() {
    if (!pendingAction) return;
    messages = [
      ...messages,
      { role: "system", text: t("aiChat.cancelled") },
    ];
    pendingAction = null;
    scrollToBottom();
  }

  function cancelRequest() {
    requestToken++;
    loading = false;
    messages = [...messages, { role: "system", text: t("aiChat.requestCancelled") }];
    scrollToBottom();
  }

  function clearChat() {
    messages = [];
    input = "";
  }

  function renderMarkdown(text: string): string {
    let html = text
      // Escape HTML first
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      // Code blocks (``` ... ```)
      .replace(/```(\w*)\n([\s\S]*?)```/g, (_m, _lang, code) =>
        `<pre><code>${code.trim()}</code></pre>`)
      // Inline code
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      // Headers
      .replace(/^### (.+)$/gm, "<strong style='font-size:1.05em'>$1</strong>")
      .replace(/^## (.+)$/gm, "<strong style='font-size:1.1em;display:block;margin:6px 0 2px'>$1</strong>")
      .replace(/^# (.+)$/gm, "<strong style='font-size:1.2em;display:block;margin:8px 0 4px'>$1</strong>")
      // Bold
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      // Italic
      .replace(/\*(.+?)\*/g, "<em>$1</em>")
      // Unordered lists
      .replace(/^- (.+)$/gm, "<li>$1</li>")
      // Ordered lists
      .replace(/^\d+\. (.+)$/gm, "<li>$1</li>")
      // Line breaks (double newline = paragraph, single = br)
      .replace(/\n\n/g, "</p><p>")
      .replace(/\n/g, "<br>");

    // Wrap consecutive <li> in <ul>
    html = html.replace(/((?:<li>.*?<\/li>(?:<br>)?)+)/g, "<ul>$1</ul>");
    html = html.replace(/<ul>([\s\S]*?)<\/ul>/g, (_m, inner) =>
      "<ul>" + inner.replace(/<br>/g, "") + "</ul>");

    // Make PID references clickable
    html = renderWithClickablePids(html);

    return `<p>${html}</p>`;
  }
</script>

<div class="ai-chat" role="region" aria-label="AI Chat">
  <div class="chat-header">
    <span class="chat-title">{t("aiChat.title")}</span>
    <span class="chat-help" title={t("aiChat.helpTooltip")}>&#9432;</span>
    <span class="chat-provider">{get(aiProviderConfig).provider}</span>
    {#if messages.length > 0}
      <button class="clear-btn" onclick={clearChat}>{t("common.clear")}</button>
    {/if}
  </div>

  {#if messages.length > 0}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="chat-messages" bind:this={chatContainer} onclick={handleChatClick} transition:slide={{ duration: 200 }}>
      {#each messages as msg}
        <div class="chat-msg chat-{msg.role}">
          <span class="chat-role">
              {msg.role === "user"
              ? t("aiChat.userLabel")
              : msg.role === "assistant"
                ? t("aiChat.assistantLabel")
                : msg.role === "tool"
                  ? t("aiChat.actionLabel")
                  : t("aiChat.systemLabel")}
          </span>
          <span class="chat-text">
            {#if msg.role === "assistant"}
              {@html renderMarkdown(msg.text)}
            {:else}
              {msg.text}
            {/if}
            {#if msg.toolResult}
              <span class="tool-badge" class:success={msg.toolResult.success} class:fail={!msg.toolResult.success}>
                {msg.toolResult.tool}
              </span>
            {/if}
          </span>
        </div>
      {/each}
      {#if loading}
        <div class="chat-msg chat-assistant">
          <span class="chat-role">{t("aiChat.assistantLabel")}</span>
          <span class="chat-text typing">{t("aiChat.thinking")}<span class="dots"><span>.</span><span>.</span><span>.</span></span></span>
          <button class="cancel-btn" onclick={cancelRequest}>{t("aiChat.cancel")}</button>
        </div>
      {/if}
      {#if pendingAction}
        <div class="action-preview">
          <div class="action-header">
            <span class="action-icon">⚠</span>
            <strong>{t("aiChat.pendingAction")}: {pendingAction.tool}</strong>
          </div>
          <div class="action-details">{formatActionDetails(pendingAction.tool, pendingAction.details)}</div>
          <div class="action-buttons">
            <button class="confirm-btn" onclick={confirmAction}>{t("aiChat.confirm")}</button>
            <button class="reject-btn" onclick={rejectAction}>{t("aiChat.cancel")}</button>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <div class="chat-empty">
      <p>{t("aiChat.emptyState")}</p>
      <div class="suggestions">
        <button onclick={() => { input = t("aiChat.suggestion1"); handleSubmit(); }}>
          {t("aiChat.suggestion1")}
        </button>
        <button onclick={() => { input = t("aiChat.suggestion2"); handleSubmit(); }}>
          {t("aiChat.suggestion2")}
        </button>
        <button onclick={() => { input = t("aiChat.suggestion3"); handleSubmit(); }}>
          {t("aiChat.suggestion3")}
        </button>
        <button onclick={() => { input = t("aiChat.suggestion4"); handleSubmit(); }}>
          {t("aiChat.suggestion4")}
        </button>
      </div>
    </div>
  {/if}

  <div class="chat-input-row">
    <input
      class="chat-input"
      type="text"
      placeholder={t("aiChat.placeholder")}
      bind:value={input}
      onkeydown={(e) => { if (e.key === "Enter") handleSubmit(); }}
      disabled={loading}
    />
    <button
      class="send-btn"
      onclick={handleSubmit}
      disabled={loading || !input.trim()}
    >
      {loading ? "..." : t("aiChat.send")}
    </button>
  </div>
</div>

<style>
  .ai-chat {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius, 6px);
    background: var(--bg-alt);
    overflow: hidden;
  }

  .chat-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  .chat-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    color: var(--fg);
  }

  .chat-provider {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 1px 6px;
    border: 1px solid var(--accent);
    border-radius: 3px;
  }

  .clear-btn {
    margin-left: auto;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    cursor: pointer;
  }
  .clear-btn:hover { color: var(--danger); border-color: var(--danger); }

  .chat-messages {
    flex: 1;
    min-height: 120px;
    max-height: 300px;
    overflow-y: auto;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .chat-msg {
    display: flex;
    gap: 8px;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    line-height: 1.5;
    padding: 4px 0;
  }

  .chat-role {
    font-weight: 700;
    flex-shrink: 0;
    min-width: 48px;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    text-transform: uppercase;
    letter-spacing: 0.3px;
    padding-top: 2px;
  }

  .chat-user .chat-role { color: var(--accent); }
  .chat-assistant .chat-role { color: var(--green); }
  .chat-system .chat-role { color: var(--yellow); }
  .chat-tool .chat-role { color: var(--cyan, #06b6d4); }

  .chat-text {
    color: var(--fg);
    word-break: break-word;
    flex: 1;
    line-height: 1.6;
  }

  .chat-user .chat-text {
    white-space: pre-wrap;
  }

  /* Markdown rendered content */
  .chat-text :global(p) { margin: 0 0 4px; }
  .chat-text :global(p:last-child) { margin-bottom: 0; }
  .chat-text :global(strong) { color: var(--fg); font-weight: 700; }
  .chat-text :global(em) { font-style: italic; color: var(--fg-dim); }
  .chat-text :global(ul) {
    margin: 4px 0;
    padding-left: 18px;
    list-style: disc;
  }
  .chat-text :global(li) {
    margin: 2px 0;
  }
  .chat-text :global(code) {
    background: rgba(0, 0, 0, 0.15);
    padding: 1px 5px;
    border-radius: 3px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 0.9em;
  }
  .chat-text :global(pre) {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
    padding: 8px 10px;
    margin: 6px 0;
    overflow-x: auto;
  }
  .chat-text :global(pre code) {
    background: none;
    padding: 0;
    font-size: 0.85em;
    white-space: pre;
  }

  .chat-help {
    cursor: help;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 1.1);
    opacity: 0.7;
    transition: opacity 0.15s;
  }
  .chat-help:hover { opacity: 1; color: var(--accent); }

  .cancel-btn {
    margin-left: auto;
    padding: 2px 10px;
    border: 1px solid var(--danger, #ef4444);
    border-radius: 3px;
    background: transparent;
    color: var(--danger, #ef4444);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
  }
  .cancel-btn:hover { background: var(--danger, #ef4444); color: white; }

  .typing {
    color: var(--fg-dim);
    font-style: italic;
  }

  .typing .dots span {
    animation: blink 1.4s infinite both;
    display: inline-block;
  }
  .typing .dots span:nth-child(2) { animation-delay: 0.2s; }
  .typing .dots span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes blink {
    0%, 80%, 100% { opacity: 0; }
    40% { opacity: 1; }
  }

  .tool-badge {
    display: inline-block;
    margin-left: 6px;
    padding: 0 5px;
    border-radius: 3px;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    vertical-align: middle;
  }
  .tool-badge.success { background: rgba(34, 197, 94, 0.15); color: var(--green); }
  .tool-badge.fail { background: rgba(239, 68, 68, 0.15); color: var(--danger); }

  .action-preview {
    margin: 8px 0;
    padding: 10px 12px;
    border: 1px solid var(--yellow, #eab308);
    border-radius: var(--radius, 6px);
    background: rgba(234, 179, 8, 0.08);
  }

  .action-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    margin-bottom: 6px;
    color: var(--yellow, #eab308);
  }

  .action-icon {
    font-size: 1.1em;
  }

  .action-details {
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    color: var(--fg);
    margin-bottom: 8px;
    white-space: pre-wrap;
    line-height: 1.5;
    padding: 6px 8px;
    background: rgba(0, 0, 0, 0.1);
    border-radius: 4px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .action-buttons {
    display: flex;
    gap: 8px;
  }

  .confirm-btn {
    padding: 4px 16px;
    border: none;
    border-radius: 4px;
    background: var(--green, #22c55e);
    color: white;
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .confirm-btn:hover { opacity: 0.85; }

  .reject-btn {
    padding: 4px 16px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--fg-dim);
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .reject-btn:hover { color: var(--danger); border-color: var(--danger); }

  .chat-empty {
    padding: 20px 12px;
    text-align: center;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.917);
  }

  .chat-empty p {
    margin: 0 0 12px;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: center;
  }

  .suggestions button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    color: var(--fg);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .suggestions button:hover {
    border-color: var(--accent);
    background: var(--bg-hover, rgba(59, 130, 246, 0.08));
  }

  .chat-input-row {
    display: flex;
    gap: 4px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .chat-input {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg-alt);
    color: var(--fg);
    padding: 6px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    outline: none;
    transition: border-color 0.15s;
  }
  .chat-input:focus { border-color: var(--accent); }
  .chat-input::placeholder { color: var(--fg-dim); opacity: 0.6; }
  .chat-input:disabled { opacity: 0.5; }

  .send-btn {
    padding: 0 14px;
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

  :global(.pid-link) {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-weight: 700;
    padding: 0;
    font-size: inherit;
    font-family: inherit;
    text-decoration: underline;
    text-decoration-style: dotted;
  }
  :global(.pid-link:hover) {
    color: var(--accent-hover, #1d4ed8);
    text-decoration-style: solid;
  }
</style>
