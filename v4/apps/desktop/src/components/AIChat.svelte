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
            {msg.text}
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
    white-space: pre-wrap;
    word-break: break-word;
    flex: 1;
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
