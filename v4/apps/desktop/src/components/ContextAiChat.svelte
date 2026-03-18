<script lang="ts">
  import { get } from "svelte/store";
  import { slide } from "svelte/transition";
  import { ipcAnalyzeContext } from "../lib/ipc";
  import { aiProviderConfig } from "../stores/preferences";
  import { t } from "../lib/i18n";
  import { renderMarkdown } from "../lib/markdown";
  import { scrollToBottom as scrollContainerToBottom, resizeInput as resizeTextarea } from "../lib/chatUtils";
  import type { ChatMessage } from "../lib/chatUtils";
  import { AI_CHAT_TIMEOUT_MS } from "../lib/constants";
  import InfoPopover from "./InfoPopover.svelte";
  import Button from "./Button.svelte";

  interface Props {
    title: string;
    placeholder: string;
    emptyState: string;
    buildContext: (question: string) => string;
    helpTooltip?: string;
    sendLabel?: string;
    inputAriaLabel?: string;
    maxHeight?: number;
  }

  let {
    title,
    placeholder,
    emptyState,
    buildContext,
    helpTooltip = "",
    sendLabel = t("common.askAi"),
    inputAriaLabel = title,
    maxHeight = 220,
  }: Props = $props();

  let input = $state("");
  let loading = $state(false);
  let messages = $state<ChatMessage[]>([]);
  let chatContainer: HTMLDivElement | undefined = $state();
  let inputRef: HTMLTextAreaElement | undefined = $state();
  let requestToken = 0;

  function doResizeInput() {
    resizeTextarea(inputRef);
  }

  $effect(() => {
    void input;
    doResizeInput();
  });

  function scrollToBottom() {
    scrollContainerToBottom(chatContainer);
  }

  function clearConversation() {
    messages = [];
    input = "";
  }

  export async function ask(question: string) {
    if (!question || loading) return;
    input = question;
    await handleSubmit();
  }

  async function handleSubmit() {
    const trimmed = input.trim();
    if (!trimmed || loading) return;

    const token = ++requestToken;
    messages = [...messages, { role: "user", text: trimmed }];
    input = "";
    loading = true;
    scrollToBottom();

    try {
      const cfg = get(aiProviderConfig);
      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error(t("aiChat.timeoutError"))), AI_CHAT_TIMEOUT_MS)
      );
      const response = await Promise.race([
        ipcAnalyzeContext(buildContext(trimmed), cfg.provider, cfg.model),
        timeoutPromise,
      ]);

      if (token !== requestToken) return;
      messages = [...messages, { role: "assistant", text: response }];
    } catch (e) {
      if (token !== requestToken) return;
      const msg = e instanceof Error ? e.message : String(e);
      messages = [...messages, { role: "system", text: msg }];
    } finally {
      if (token === requestToken) {
        loading = false;
        scrollToBottom();
      }
    }
  }
</script>

<div class="context-chat" role="region" aria-label={title}>
  <div class="context-chat-header">
    <span class="context-chat-title">{title}</span>
    {#if helpTooltip}
      <InfoPopover label={title} content={helpTooltip} />
    {/if}
    {#if messages.length > 0}
      <button class="context-chat-clear" onclick={clearConversation}>{t("common.clear")}</button>
    {/if}
  </div>

  {#if messages.length > 0}
    <div class="context-chat-messages" bind:this={chatContainer} style={`max-height:${maxHeight}px`} transition:slide={{ duration: 180 }}>
      {#each messages as msg}
        <div class="context-chat-msg context-chat-{msg.role}">
          <span class="context-chat-role">
            {msg.role === "user"
              ? t("aiChat.userLabel")
              : msg.role === "assistant"
                ? t("aiChat.assistantLabel")
                : t("aiChat.systemLabel")}
          </span>
          <span class="context-chat-text">
            {#if msg.role === "assistant"}
              {@html renderMarkdown(msg.text)}
            {:else}
              {msg.text}
            {/if}
          </span>
        </div>
      {/each}
      {#if loading}
        <div class="context-chat-msg context-chat-assistant">
          <span class="context-chat-role">{t("aiChat.assistantLabel")}</span>
          <span class="context-chat-text typing">{t("aiChat.thinking")}<span class="dots"><span>.</span><span>.</span><span>.</span></span></span>
        </div>
      {/if}
    </div>
  {:else}
    <div class="context-chat-empty">{emptyState}</div>
  {/if}

  <div class="context-chat-input-row">
    <textarea
      class="context-chat-input"
      rows="1"
      bind:value={input}
      bind:this={inputRef}
      placeholder={placeholder}
      aria-label={inputAriaLabel}
      disabled={loading}
      onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSubmit(); } }}
      style="resize: none;"
    ></textarea>
    <Button class="context-chat-send" variant="primary" size="sm" onclick={handleSubmit} disabled={loading || !input.trim()}>
      {loading ? t("common.loadingShort") : sendLabel}
    </Button>
  </div>
</div>

<style>
  .context-chat {
    display: flex;
    flex-direction: column;
    gap: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg);
    padding: 8px;
  }

  .context-chat-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .context-chat-title {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--accent);
  }

  .context-chat-clear {
    margin-left: auto;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: transparent;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    padding: 2px 8px;
    cursor: pointer;
  }

  .context-chat-clear:hover {
    color: var(--fg);
    border-color: var(--accent);
  }

  .context-chat-empty {
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    line-height: 1.5;
  }

  .context-chat-messages {
    overflow-y: auto;
    border: 1px solid var(--border-subtle, #2a2a3a);
    border-radius: 4px;
    background: var(--bg-alt);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .context-chat-msg {
    display: flex;
    gap: 8px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    line-height: 1.5;
  }

  .context-chat-role {
    min-width: 48px;
    flex-shrink: 0;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .context-chat-user .context-chat-role { color: var(--accent); }
  .context-chat-assistant .context-chat-role { color: var(--green); }
  .context-chat-system .context-chat-role { color: var(--yellow); }

  .context-chat-text {
    flex: 1;
    word-break: break-word;
  }

  .context-chat-text :global(p) { margin: 0 0 4px; }
  .context-chat-text :global(p:last-child) { margin-bottom: 0; }
  .context-chat-text :global(strong) { color: var(--fg); }
  .context-chat-text :global(ul) { margin: 4px 0; padding-left: 18px; }
  .context-chat-text :global(code) {
    background: var(--bg-secondary);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }
  .context-chat-text :global(pre) {
    margin: 6px 0;
    padding: 8px;
    border-radius: 4px;
    overflow-x: auto;
    background: var(--bg-primary);
  }

  .typing {
    color: var(--fg-dim);
    font-style: italic;
  }

  .typing .dots span {
    display: inline-block;
    animation: blink 1.4s infinite both;
  }

  .typing .dots span:nth-child(2) { animation-delay: 0.2s; }
  .typing .dots span:nth-child(3) { animation-delay: 0.4s; }

  .context-chat-input-row {
    display: flex;
    gap: 6px;
  }

  .context-chat-input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-alt);
    color: var(--fg);
    padding: 6px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.833);
  }

  .context-chat-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  :global(.context-chat-send) {
    flex-shrink: 0;
  }

  @keyframes blink {
    0%, 80%, 100% { opacity: 0; }
    40% { opacity: 1; }
  }
</style>
