<script lang="ts">
  import { get } from "svelte/store";
  import { slide } from "svelte/transition";
  import { ipcAiChat } from "../lib/ipc";
  import { aiProviderConfig } from "../stores/preferences";
  import { toast } from "../stores/toasts";
  import { detectPromptInjection } from "../lib/aiConfigBridge";
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

  function scrollToBottom() {
    requestAnimationFrame(() => {
      if (chatContainer) {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }
    });
  }

  async function handleSubmit() {
    const trimmed = input.trim();
    if (!trimmed || loading) return;

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
      const response = await ipcAiChat(trimmed, cfg.provider, cfg.model);

      messages = [...messages, { role: "assistant", text: response.reply }];

      if (response.tool_call) {
        const result = response.tool_call;
        messages = [
          ...messages,
          {
            role: "tool",
            text: result.details,
            toolResult: result,
          },
        ];

        if (result.success) {
          toast.success("Action", result.details);
        } else {
          toast.error("Action Failed", result.details);
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      messages = [...messages, { role: "system", text: msg }];

      if (msg.includes("No API key") || msg.includes("keyring")) {
        toast.error("Config", "Set up an AI provider in Settings first.");
      }
    } finally {
      loading = false;
      scrollToBottom();
    }
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

    return `<p>${html}</p>`;
  }
</script>

<div class="ai-chat" role="region" aria-label="AI Chat">
  <div class="chat-header">
    <span class="chat-title">AI Assistant</span>
    <span class="chat-provider">{get(aiProviderConfig).provider}</span>
    {#if messages.length > 0}
      <button class="clear-btn" onclick={clearChat}>Clear</button>
    {/if}
  </div>

  {#if messages.length > 0}
    <div class="chat-messages" bind:this={chatContainer} transition:slide={{ duration: 200 }}>
      {#each messages as msg}
        <div class="chat-msg chat-{msg.role}">
          <span class="chat-role">
            {msg.role === "user"
              ? "You"
              : msg.role === "assistant"
                ? "AI"
                : msg.role === "tool"
                  ? "Action"
                  : "System"}
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
          <span class="chat-role">AI</span>
          <span class="chat-text typing">Thinking...</span>
        </div>
      {/if}
    </div>
  {:else}
    <div class="chat-empty">
      <p>Ask me anything about your system:</p>
      <div class="suggestions">
        <button onclick={() => { input = "Close all YouTube tabs"; handleSubmit(); }}>
          Close all YouTube tabs
        </button>
        <button onclick={() => { input = "What's using the most memory?"; handleSubmit(); }}>
          What's using the most memory?
        </button>
        <button onclick={() => { input = "Kill Chrome"; handleSubmit(); }}>
          Kill Chrome
        </button>
        <button onclick={() => { input = "Analyze my network traffic"; handleSubmit(); }}>
          Analyze network traffic
        </button>
      </div>
    </div>
  {/if}

  <div class="chat-input-row">
    <input
      class="chat-input"
      type="text"
      placeholder="Type a command... (e.g. 'Kill all Chrome processes')"
      bind:value={input}
      onkeydown={(e) => { if (e.key === "Enter") handleSubmit(); }}
      disabled={loading}
    />
    <button
      class="send-btn"
      onclick={handleSubmit}
      disabled={loading || !input.trim()}
    >
      {loading ? "..." : "Send"}
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

  .typing {
    color: var(--fg-dim);
    font-style: italic;
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
</style>
