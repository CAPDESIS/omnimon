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
  import { renderMarkdown } from "../lib/markdown";
  import { scrollToBottom as scrollContainerToBottom, resizeInput as resizeTextarea } from "../lib/chatUtils";
  import type { ChatMessage } from "../lib/chatUtils";
  import type { ToolResult } from "../lib/types";
  import InfoPopover from "./InfoPopover.svelte";
  import Button from "./Button.svelte";
  import { AI_CHAT_TIMEOUT_MS } from "../lib/constants";

  interface ChatMessageWithTool extends ChatMessage {
    toolResult?: ToolResult;
    isError?: boolean;
    canRetry?: boolean;
    retryText?: string;
  }

  let input = $state("");
  let inputRef: HTMLTextAreaElement | undefined = $state();
  let loading = $state(false);
  let messages = $state<ChatMessageWithTool[]>([]);
  let chatContainer: HTMLDivElement | undefined = $state();
  let pendingAction = $state<{ tool: string; details: string; result: ToolResult } | null>(null);
  let requestToken = 0;

  function scrollToBottom() {
    scrollContainerToBottom(chatContainer);
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
      toast.error(t("aiChat.blockedTitle"), t("aiChat.blockedPrompt"));
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
        setTimeout(() => reject(new Error(t("aiChat.timeoutError"))), AI_CHAT_TIMEOUT_MS)
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
            toast.success(t("aiChat.actionSuccessTitle"), result.details);
          } else {
            toast.error(t("aiChat.actionErrorTitle"), result.details);
          }
        }
      }
    } catch (e) {
      if (token !== requestToken) return;
      const msg = e instanceof Error ? e.message : String(e);
      
      let errorText = msg;
      const lowerMsg = msg.toLowerCase();
      
      if (lowerMsg.includes("timeout") || lowerMsg.includes("time out")) {
          errorText = "Error de conexión: Timeout agotado. ¿Deseas reintentar o usar un modelo local (Ollama)?";
      } else if (lowerMsg.includes("fetch failed") || lowerMsg.includes("network error") || lowerMsg.includes("connection refused") || lowerMsg.includes("ollama is not running") || lowerMsg.includes("ai request failed")) {
          errorText = "Error de API: No se pudo contactar al proveedor. Si usas Ollama, verifica que esté corriendo (`ollama serve`).";
      } else {
          errorText = `Error al procesar la solicitud: ${msg}`;
      }

      messages = [...messages, { role: "system", text: errorText, isError: true, canRetry: true, retryText: trimmed }];

      if (msg.includes("No API key") || msg.includes("keyring")) {
        toast.error(t("aiChat.configErrorTitle"), t("aiChat.providerSetupFirst"));
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
      return t("aiChat.closeTabsExcept", { patterns });
    }
    if (details.startsWith("close_tabs:")) {
      const patterns = details.replace("close_tabs:", "").split("|").join(", ");
      return t("aiChat.closeTabsMatching", { patterns });
    }
    return details;
  }

  async function executeCloseTabs(details: string): Promise<{ closed: number; message: string }> {
    const isExcept = details.startsWith("close_tabs_except:");
    const raw = details.replace(/^close_tabs(_except)?:/, "").trim();
    if (!raw) return { closed: 0, message: t("aiChat.noPatternProvided") };

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
          return { closed: 0, message: t("aiChat.noTabsMatched", { patterns: patterns.join(", ") }) };
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
        ? t("aiChat.closedTabs", { count: closed, suffix: failed.length > 0 ? `, ${failed.length} failed` : "" })
        : t("aiChat.failedCloseTabs", { count: failed.length });
      return { closed, message: msg };
    } catch (e) {
      return { closed: 0, message: t("aiChat.errorPrefix", { message: e instanceof Error ? e.message : String(e) }) };
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
      toast.success(t("aiChat.actionSuccessTitle"), result.details);
    } else {
      toast.error(t("aiChat.actionErrorTitle"), result.details);
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

  function retryMessage(text: string) {
    if (loading) return;
    input = text;
    // Remove the error message and the immediately preceding user message
    let lastErrorIndex = -1;
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].isError) {
        lastErrorIndex = i;
        break;
      }
    }
    if (lastErrorIndex !== -1) {
      messages = messages.filter((_, i) => i !== lastErrorIndex && i !== lastErrorIndex - 1);
    }
    handleSubmit();
  }

  function doResizeInput() {
    resizeTextarea(inputRef);
  }

  $effect(() => {
    input;
    doResizeInput();
  });

  function renderMessage(text: string): string {
    return renderWithClickablePids(renderMarkdown(text));
  }
</script>

<div class="ai-chat" role="region" aria-label={t("aiChat.regionLabel")}>
  <div class="chat-header">
    <span class="chat-title">{t("aiChat.title")}</span>
    <InfoPopover label={t("aiChat.title")} content={t("aiChat.helpTooltip")} />
    <span class="chat-provider">{get(aiProviderConfig).provider}</span>
    {#if messages.length > 0}
      <Button class="clear-btn" variant="ghost" size="sm" onclick={clearChat}>{t("common.clear")}</Button>
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
              {@html renderMessage(msg.text)}
            {:else}
              {msg.text}
            {/if}
            {#if msg.toolResult}
              <span class="tool-badge" class:success={msg.toolResult.success} class:fail={!msg.toolResult.success}>
                {msg.toolResult.tool}
              </span>
            {/if}
            {#if msg.isError && msg.canRetry && msg.retryText}
              <div class="error-actions">
                <Button class="retry-btn" variant="secondary" size="sm" onclick={() => retryMessage(msg.retryText!)}>
                  ↻ {t("common.retry") || "Reintentar"}
                </Button>
              </div>
            {/if}
          </span>
        </div>
      {/each}
      {#if loading}
        <div class="chat-msg chat-assistant">
          <span class="chat-role">{t("aiChat.assistantLabel")}</span>
          <span class="chat-text typing">{t("aiChat.thinking")}<span class="dots"><span>.</span><span>.</span><span>.</span></span></span>
          <Button class="cancel-btn" variant="danger" size="sm" onclick={cancelRequest}>{t("aiChat.cancel")}</Button>
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
            <Button class="confirm-btn" variant="primary" size="sm" onclick={confirmAction}>{t("aiChat.confirm")}</Button>
            <Button class="reject-btn" variant="ghost" size="sm" onclick={rejectAction}>{t("aiChat.cancel")}</Button>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <div class="chat-empty">
      <p>{t("aiChat.emptyState")}</p>
      <div class="suggestions">
        <Button variant="secondary" size="sm" onclick={() => { input = t("aiChat.suggestion1"); handleSubmit(); }}>
          {t("aiChat.suggestion1")}
        </Button>
        <Button variant="secondary" size="sm" onclick={() => { input = t("aiChat.suggestion2"); handleSubmit(); }}>
          {t("aiChat.suggestion2")}
        </Button>
        <Button variant="secondary" size="sm" onclick={() => { input = t("aiChat.suggestion3"); handleSubmit(); }}>
          {t("aiChat.suggestion3")}
        </Button>
        <Button variant="secondary" size="sm" onclick={() => { input = t("aiChat.suggestion4"); handleSubmit(); }}>
          {t("aiChat.suggestion4")}
        </Button>
      </div>
    </div>
  {/if}

  <div class="chat-input-row">
    <textarea
      class="chat-input"
      placeholder={t("aiChat.placeholder")}
      bind:value={input}
      bind:this={inputRef}
      rows="1"
      onkeydown={(e: KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          handleSubmit();
        }
      }}
      disabled={loading}
    ></textarea>
    <Button
      class="send-btn"
      variant="primary"
      onclick={handleSubmit}
      disabled={loading || !input.trim()}
    >
      {loading ? t("common.loadingShort") : t("aiChat.send")}
    </Button>
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
  }

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

  .cancel-btn {
    margin-left: auto;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    flex-shrink: 0;
  }

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

  .error-actions {
    margin-top: 8px;
  }

  .retry-btn {
    min-height: 30px;
  }

  .confirm-btn {
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .reject-btn {
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

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

  .chat-input-row {
    display: flex;
    gap: 4px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .chat-input {
    flex: 1;
    min-height: 40px;
    max-height: 180px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg-alt);
    color: var(--fg);
    padding: 6px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    line-height: 1.45;
    outline: none;
    transition: border-color 0.15s;
    resize: none;
    overflow-y: auto;
  }
  .chat-input:focus { border-color: var(--accent); }
  .chat-input::placeholder { color: var(--fg-dim); opacity: 0.6; }
  .chat-input:disabled { opacity: 0.5; }

  .send-btn {
    text-transform: uppercase;
    letter-spacing: 0.5px;
    white-space: nowrap;
  }

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
