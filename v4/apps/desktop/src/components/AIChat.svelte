<script lang="ts">
  import { tick } from "svelte";
  import { get } from "svelte/store";

  import { slide } from "svelte/transition";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { ipcAiChat, ipcGetBrowserTabs, ipcCloseBrowserTab, ipcKillProcess, ipcKillProcesses } from "../lib/ipc";
  import { aiProviderConfig, aiCacheTtlMinutes, userMode } from "../stores/preferences";
  import { processes } from "../stores/processes";
  import { inspectProcessRequest, askAiRequest } from "../stores/uiActions";
  import { toast } from "../stores/toasts";
  import { detectPromptInjection } from "../lib/aiConfigBridge";
  import { t, resolvedLocale } from "../lib/i18n";
  import { formatToolResultDetails, localizeBackendError } from "../lib/localizedUi";
  import { renderMarkdown } from "../lib/markdown";
  import { scrollToBottom as scrollContainerToBottom, resizeInput as resizeTextarea } from "../lib/chatUtils";
  import type { ChatMessage } from "../lib/chatUtils";
  import type { ToolResult } from "../lib/types";
  import { AI_PRESETS, AI_PRESET_CATEGORY_LABELS } from "../lib/aiPresets";
  import InfoPopover from "./InfoPopover.svelte";
  import Button from "./Button.svelte";
  import EmptyState from "./EmptyState.svelte";
  import { AI_CHAT_TIMEOUT_MS } from "../lib/constants";
  import { AlertTriangle, Sparkles } from "lucide-svelte";

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
  let isAutoScroll = $state(true);
  let streamingMessage = $state("");
  let activePresetCategory = $state<(typeof AI_PRESETS)[number]["category"] | null>(null);

  // Tab selection for close_tabs actions
  interface SelectableTab { id: string; title: string; url: string; browser: string; selected: boolean; }
  let pendingTabs = $state<SelectableTab[]>([]);
  let pendingTabsLoading = $state(false);

  const MAX_CHAT_INPUT_CHARS = 4000;
  const presetGroups = Object.entries(
    AI_PRESETS.reduce((acc, preset) => {
      (acc[preset.category] ??= []).push(preset);
      return acc;
    }, {} as Record<(typeof AI_PRESETS)[number]["category"], typeof AI_PRESETS[number][]>),
  ) as Array<[(typeof AI_PRESETS)[number]["category"], typeof AI_PRESETS[number][]]>;

  const showPresetChips = $derived(messages.length === 0 || input.trim().length === 0);

  $effect(() => {
    if (activePresetCategory === null && presetGroups.length > 0) {
      activePresetCategory = presetGroups[0][0];
    }
  });

  function sanitizeUserInput(value: string): string {
    return value.normalize("NFKC").replace(/[\u0000-\u001f\u007f]/g, " ").trim();
  }

  $effect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;
    listen<string>("ai-stream-token", (event: { payload: string }) => {
      if (cancelled) return;
      streamingMessage += event.payload;
      if (isAutoScroll) {
        scrollToBottom();
      }
    }).then((fn: UnlistenFn) => {
      if (cancelled) { fn(); return; }
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  function handleScroll() {
    if (!chatContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = chatContainer;
    isAutoScroll = scrollHeight - scrollTop - clientHeight < 50;
  }

  $effect(() => {
    const _len = messages.length; // trigger reactivo
    if (isAutoScroll && chatContainer) {
      tick().then(() => {
        chatContainer!.scrollTop = chatContainer!.scrollHeight;
      });
    }
  });

  $effect(() => {
    const request = $askAiRequest;
    if (request) {
      input = request;
      askAiRequest.set(null);
      tick().then(() => handleSubmit());
    }
  });

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

  function applyPreset(prompt: string) {
    input = prompt;
    tick().then(() => inputRef?.focus());
  }

  function formatPayload(result: ToolResult): string {
    const payload = result.payload;
    if (!result.success) return localizeBackendError(result.details);
    if (!payload) return formatToolResultDetails(result);

    if (result.tool === "get_process_details") {
      const pid = payload.pid ?? "-";
      const name = payload.name ?? t("common.untitled");
      const cpu = payload.cpu_pct ?? "-";
      const ram = payload.ram_mb ?? "-";
      const state = payload.state ?? "-";
      return `${formatToolResultDetails(result)}\n\n${t("process.pid")}: ${pid}\n${t("process.name")}: ${name}\n${t("process.cpu")}: ${cpu}%\n${t("process.ram")}: ${ram} MB\n${t("process.state")}: ${state}`;
    }

    if (result.tool === "run_security_scan") {
      const findings = Array.isArray(payload.findings) ? payload.findings : [];
      if (findings.length === 0) return `${formatToolResultDetails(result)}\n\n${t("aiChat.noFindings")}`;
      const lines = findings.map((finding: unknown) => {
        const item = finding as Record<string, unknown>;
        return `- [${String(item.severity ?? "info").toUpperCase()}] ${String(item.process_name ?? "unknown")} (PID ${String(item.pid ?? "?")})`;
      });
      return `${formatToolResultDetails(result)}\n\n${lines.join("\n")}`;
    }

    if (result.tool === "get_network_details") {
      const connections = Array.isArray(payload.connections) ? payload.connections : [];
      if (connections.length === 0) return `${formatToolResultDetails(result)}\n\n${t("aiChat.noActiveConnections")}`;
      const lines = connections.map((connection: unknown) => {
        const item = connection as Record<string, unknown>;
        return `- ${String(item.protocol ?? "?")} ${String(item.dst_ip ?? "?")}:${String(item.dst_port ?? "?")} (${String(item.bytes ?? 0)} bytes)`;
      });
      return `${formatToolResultDetails(result)}\n\n${lines.join("\n")}`;
    }

    if (result.tool === "explain_process") {
      return `${formatToolResultDetails(result)}\n\n${t("process.executable")}: ${String(payload.exe_path ?? t("common.unknown"))}\n${t("process.bundleId")}: ${String(payload.bundle_id ?? t("common.notAvailable"))}`;
    }

    if (result.tool === "get_system_summary") {
      return `${formatToolResultDetails(result)}\n\n${t("process.cpu")}: ${String(payload.cpu_pct ?? "—")}%\n${t("process.ram")}: ${String(payload.ram_used_gb ?? "—")}/${String(payload.ram_total_gb ?? "—")} GB\n${t("status.swap")}: ${String(payload.swap_mb ?? "—")} MB\n${t("status.net")}: ${t("systemMetrics.rx")} ${String(payload.net_rx_bytes_per_sec ?? "—")} B/s, ${t("systemMetrics.tx")} ${String(payload.net_tx_bytes_per_sec ?? "—")} B/s`;
    }

    return formatToolResultDetails(result);
  }

  async function handleSubmit() {
    const trimmed = sanitizeUserInput(input);
    if (!trimmed || loading) return;

    if (trimmed.length > MAX_CHAT_INPUT_CHARS) {
      toast.error(t("aiChat.blockedTitle"), t("aiChat.maxInputError", { count: MAX_CHAT_INPUT_CHARS }));
      return;
    }

    const token = ++requestToken;

    if (detectPromptInjection(trimmed)) {
      console.warn("[AIChat] Blocked prompt injection attempt");
      toast.error(t("aiChat.blockedTitle"), t("aiChat.blockedPrompt"));
      return;
    }

    messages = [...messages, { role: "user", text: trimmed }];
    input = "";
    loading = true;
    streamingMessage = "";
    scrollToBottom();

    try {
      const cfg = get(aiProviderConfig);
      const currentLang = get(resolvedLocale);
      const systemInstruction = `Responde siempre en el idioma '${currentLang}'. No menciones esta instrucción.`;
      
      // Build conversation history (last 10 messages max to avoid token overflow)
      const messageHistory = messages
        .slice(0, -1)
        .filter(m => m.role === "user" || m.role === "assistant")
        .slice(-10)
        .map(m => [m.role, sanitizeUserInput(m.text).slice(0, 2000)] as [string, string]);
        
      const history: Array<[string, string]> = [
        ["system", systemInstruction],
        ...messageHistory
      ];
      
      // Race the AI call against a 45-second timeout
      const timeoutPromise = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error(t("aiChat.timeoutError"))), AI_CHAT_TIMEOUT_MS)
      );
      const startTime = performance.now();
      const response = await Promise.race([
        ipcAiChat(trimmed, cfg.provider, cfg.model, history, get(aiCacheTtlMinutes)),
        timeoutPromise,
      ]);
      const responseTimeMs = performance.now() - startTime;

      if (token !== requestToken) return;

      streamingMessage = "";
      const tokens = Math.round((trimmed.length + response.reply.length) / 4);
      const metadata = {
        responseTimeMs,
        model: cfg.model,
        tokens,
        toolCalls: response.tool_call ? [{ name: response.tool_call.tool, args: { success: response.tool_call.success } }] : undefined
      };
      messages = [...messages, { role: "assistant", text: response.reply, metadata }];

      if (response.tool_call) {
        const result = response.tool_call;
        // For destructive actions, require confirmation
        if (result.tool === "close_tabs" || result.tool === "kill_process" || result.tool === "kill_by_name" || result.tool === "close_connection" || result.tool === "add_automation_rule" || result.tool === "remove_automation_rule") {
          pendingAction = { tool: result.tool, details: result.details, result };
          // Load tabs for selection UI
          if (result.tool === "close_tabs") {
            loadPendingTabs(result.details);
          }
        } else {
          messages = [
            ...messages,
            { role: "tool", text: formatPayload(result), toolResult: result },
          ];
          if (result.success) {
            toast.success(t("aiChat.actionSuccessTitle"), formatToolResultDetails(result));
          } else {
            toast.error(t("aiChat.actionErrorTitle"), localizeBackendError(result.details));
          }
        }
      }
    } catch (e) {
      if (token !== requestToken) return;
      const msg = e instanceof Error ? e.message : String(e);
      console.warn("[AIChat] Chat request failed:", msg);
      
      let errorText = msg;
      const lowerMsg = msg.toLowerCase();
      
      if (lowerMsg.includes("timeout") || lowerMsg.includes("time out")) {
          errorText = t("aiChat.errorTimeout");
      } else if (lowerMsg.includes("fetch failed") || lowerMsg.includes("network error") || lowerMsg.includes("connection refused") || lowerMsg.includes("ollama is not running") || lowerMsg.includes("ai request failed")) {
          errorText = t("aiChat.errorApi");
      } else {
          errorText = t("aiChat.errorGeneric", { msg });
      }

      messages = [...messages, { role: "system", text: errorText, isError: true, canRetry: true, retryText: trimmed }];

      if (msg.includes("No API key") || msg.includes("keyring")) {
        toast.error(t("aiChat.configErrorTitle"), t("aiChat.providerSetupFirst"));
      }
    } finally {
      if (token === requestToken) {
        loading = false;
        streamingMessage = "";
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
    if (details.startsWith("kill_process:")) {
      const parts = details.replace("kill_process:", "").split(":");
      return t("aiChat.killProcessDesc", { name: parts[1] ?? "unknown", pid: parts[0] });
    }
    if (details.startsWith("close_connection:")) {
      const parts = details.replace("close_connection:", "").split(":");
      return t("aiChat.closeConnectionDesc", { pid: parts[0], ip: parts[1], port: parts[2] });
    }
    if (details.startsWith("kill_by_name:")) {
      const parts = details.replace("kill_by_name:", "").split(":");
      const pids = parts[1]?.split(",") ?? [];
      return t("aiChat.killByNameDesc", { count: pids.length, name: parts[0], pids: pids.join(", ") });
    }
    if (details.startsWith("add_automation_rule:")) {
      const parts = details.replace("add_automation_rule:", "").split(":");
      return t("aiChat.addRuleDesc", {
        pattern: parts[0] ?? "?",
        metric: parts[1] ?? "?",
        action: parts[2] ?? "?",
      });
    }
    if (details.startsWith("remove_automation_rule:")) {
      const id = details.replace("remove_automation_rule:", "");
      return t("aiChat.removeRuleDesc", { id });
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

  async function executeKillProcess(details: string): Promise<{ success: boolean; message: string }> {
    const parts = details.replace("kill_process:", "").split(":");
    const pid = parseInt(parts[0], 10);
    const name = parts[1] ?? "unknown";
    if (!pid || pid <= 0) {
      return { success: false, message: t("aiChat.invalidPid", { pid: parts[0] }) };
    }
    try {
      const ok = await ipcKillProcess(pid);
      return ok
        ? { success: true, message: t("aiChat.killedProcess", { name, pid }) }
        : { success: false, message: t("aiChat.processNotFound", { pid }) };
    } catch (e) {
      return { success: false, message: t("aiChat.failedKill", { pid, error: e instanceof Error ? e.message : String(e) }) };
    }
  }

  async function executeCloseConnection(details: string): Promise<{ success: boolean; message: string }> {
    const parts = details.replace("close_connection:", "").split(":");
    const pid = parseInt(parts[0], 10);
    const ip = parts[1];
    const port = parts[2];
    if (!pid || pid <= 0) {
      return { success: false, message: t("aiChat.invalidPid", { pid: parts[0] }) };
    }
    try {
      // Direct connection closing is not fully supported without elevated privileges, so we fallback to process killing
      const ok = await ipcKillProcess(pid);
      return ok
        ? { success: true, message: t("aiChat.killedProcessForConnection", { pid, ip, port }) }
        : { success: false, message: t("aiChat.processNotFound", { pid }) };
    } catch (e) {
      return { success: false, message: t("aiChat.failedKill", { pid, error: e instanceof Error ? e.message : String(e) }) };
    }
  }

  async function executeKillByName(details: string): Promise<{ success: boolean; message: string }> {
    const parts = details.replace("kill_by_name:", "").split(":");
    const name = parts[0] ?? "";
    const pids = (parts[1] ?? "").split(",").map(p => parseInt(p, 10)).filter(p => p > 0);
    if (pids.length === 0) {
      return { success: false, message: t("aiChat.noValidPids", { name }) };
    }
    try {
      const result = await ipcKillProcesses(pids);
      const killed = result.killed.length;
      const failed = result.failed.length;
      if (killed > 0) {
        return {
          success: true,
          message: t("aiChat.killedProcesses", { killed, total: pids.length, name, failedText: failed > 0 ? ` (${failed} failed)` : "" }),
        };
      }
      return { success: false, message: t("aiChat.failedKillAny", { name }) };
    } catch (e) {
      return { success: false, message: t("aiChat.failedKillMultiple", { error: e instanceof Error ? e.message : String(e) }) };
    }
  }

  async function loadPendingTabs(details: string) {
    pendingTabsLoading = true;
    pendingTabs = [];
    try {
      const allTabs = await ipcGetBrowserTabs();
      const isExcept = details.startsWith("close_tabs_except:");
      const raw = details.replace(/^close_tabs(_except)?:/, "").trim();
      const patterns = raw.split("|").map(p => p.trim().toLowerCase());

      const matched = allTabs.filter(tab => {
        const url = tab.url.toLowerCase();
        const title = tab.title.toLowerCase();
        const matches = patterns.some(p => url.includes(p) || title.includes(p));
        return isExcept ? !matches : matches;
      });

      if (matched.length === 0) {
        // No matching tabs — auto-dismiss with error
        if (pendingAction) {
          const errorMsg = t("aiChat.noTabsMatched", { patterns: raw });
          pendingAction.result.details = errorMsg;
          pendingAction.result.success = false;
          messages = [...messages, { role: "tool", text: errorMsg, toolResult: pendingAction.result }];
          toast.error(t("aiChat.actionErrorTitle"), errorMsg);
          pendingAction = null;
        }
        return;
      }

      pendingTabs = matched.map(tab => ({
        id: tab.id,
        title: tab.title || tab.url,
        url: tab.url,
        browser: tab.browser,
        selected: true,
      }));
    } catch {
      pendingTabs = [];
    } finally {
      pendingTabsLoading = false;
    }
  }

  function toggleTab(id: string) {
    pendingTabs = pendingTabs.map(t => t.id === id ? { ...t, selected: !t.selected } : t);
  }

  function toggleAllTabs() {
    const allSelected = pendingTabs.every(t => t.selected);
    pendingTabs = pendingTabs.map(t => ({ ...t, selected: !allSelected }));
  }

  async function confirmAction() {
    if (!pendingAction) return;
    loading = true;
    const { result } = pendingAction;
    if (result.tool === "close_tabs" && result.success) {
      // Use selected tabs if available
      const selectedTabs = pendingTabs.filter(t => t.selected);
      if (selectedTabs.length > 0) {
        let closed = 0;
        const failed: string[] = [];
        for (const tab of selectedTabs) {
          try {
             await ipcCloseBrowserTab(tab.id, tab.url, tab.browser);
            closed++;
          } catch {
            failed.push(tab.title);
          }
        }
        result.details = closed > 0
          ? t("aiChat.closedTabs", { count: closed, suffix: failed.length > 0 ? `, ${failed.length} failed` : "" })
          : t("aiChat.failedCloseTabs", { count: failed.length });
        result.success = closed > 0;
      } else {
        result.details = t("aiChat.noTabsSelected");
        result.success = false;
      }
      pendingTabs = [];
    } else if (result.tool === "kill_process" && result.success) {
      const executed = await executeKillProcess(result.details);
      result.details = executed.message;
      result.success = executed.success;
    } else if (result.tool === "close_connection" && result.success) {
      const executed = await executeCloseConnection(result.details);
      result.details = executed.message;
      result.success = executed.success;
    } else if (result.tool === "kill_by_name" && result.success) {
      const executed = await executeKillByName(result.details);
      result.details = executed.message;
      result.success = executed.success;
    } else if (result.tool === "add_automation_rule" && result.success) {
      const rule = (result.payload ?? {}) as {
        id?: string;
        process_pattern?: string;
        metric?: string;
        threshold?: number;
        duration_secs?: number;
        action?: string;
      };
      if (!rule.process_pattern || !rule.metric || !rule.action) {
        result.details = t("aiChat.addRuleInvalid");
        result.success = false;
      } else {
        try {
          await invoke("add_automation_rule", { rule });
          result.details = t("aiChat.addRuleOk", { pattern: rule.process_pattern });
          result.success = true;
        } catch (e) {
          result.details = t("aiChat.addRuleFailed", { error: String(e) });
          result.success = false;
        }
      }
    } else if (result.tool === "remove_automation_rule" && result.success) {
      const id = ((result.payload ?? {}) as { id?: string }).id;
      if (!id) {
        result.details = t("aiChat.removeRuleInvalid");
        result.success = false;
      } else {
        try {
          await invoke("remove_automation_rule", { id });
          result.details = t("aiChat.removeRuleOk");
          result.success = true;
        } catch (e) {
          result.details = t("aiChat.removeRuleFailed", { error: String(e) });
          result.success = false;
        }
      }
    }

    const finalText = result.success ? result.details : formatToolResultDetails(result);

    messages = [
      ...messages,
      { role: "tool", text: finalText, toolResult: result },
    ];

    if (result.success) {
      toast.success(t("aiChat.actionSuccessTitle"), finalText);
    } else {
      toast.error(t("aiChat.actionErrorTitle"), finalText);
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
    pendingTabs = [];
    scrollToBottom();
  }

  function cancelRequest() {
    requestToken++;
    loading = false;
    streamingMessage = "";
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
    <div class="chat-messages" bind:this={chatContainer} onclick={handleChatClick} onscroll={handleScroll} transition:slide={{ duration: 200 }}>
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
              
              {#if msg.metadata && $userMode === "pro"}
                <details class="pro-metadata">
                  <summary>{t("aiChat.metadataSummary")}</summary>
                  <div class="pro-metadata-content">
                    <div><strong>{t("aiChat.metadataModel")}</strong> {msg.metadata.model}</div>
                    <div><strong>{t("aiChat.metadataResponseTime")}</strong> {Math.round(msg.metadata.responseTimeMs ?? 0)}ms</div>
                    <div><strong>{t("aiChat.metadataTokens")}</strong> {msg.metadata.tokens ?? 0}</div>
                    {#if msg.metadata.toolCalls}
                      <div><strong>{t("aiChat.metadataTools")}</strong> {JSON.stringify(msg.metadata.toolCalls)}</div>
                    {/if}
                    {#if msg.metadata.thought}
                      <div><strong>{t("aiChat.metadataThought")}</strong> {msg.metadata.thought}</div>
                    {/if}
                  </div>
                </details>
              {/if}
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
                  ↻ {t("common.retry")}
                </Button>
              </div>
            {/if}
          </span>
        </div>
      {/each}
      {#if loading}
        <div class="chat-msg chat-assistant">
          <span class="chat-role">{t("aiChat.assistantLabel")}</span>
          {#if streamingMessage}
            <span class="chat-text">{@html renderMessage(streamingMessage)}</span>
          {:else}
            <span class="chat-text typing">{t("aiChat.thinking")}<span class="dots"><span>.</span><span>.</span><span>.</span></span></span>
          {/if}
        </div>
        <div class="cancel-row">
          <Button class="cancel-btn" variant="danger" size="sm" onclick={cancelRequest}>{t("aiChat.cancel")}</Button>
        </div>
      {/if}
      {#if pendingAction}
        <div class="action-preview">
          <div class="action-header">
            <span class="action-icon"><AlertTriangle size={14} /></span>
            <strong>{t("aiChat.pendingAction")}: {pendingAction.tool}</strong>
          </div>
          <div class="action-details">{formatActionDetails(pendingAction.tool, pendingAction.details)}</div>

          {#if pendingAction.tool === "close_tabs" && pendingTabs.length > 0}
            <div class="tab-select-list">
              <label class="tab-select-all">
                <input type="checkbox" checked={pendingTabs.every(t => t.selected)} onchange={toggleAllTabs} />
                <strong>{t("aiChat.selectAll")} ({pendingTabs.filter(t => t.selected).length}/{pendingTabs.length})</strong>
              </label>
              {#each pendingTabs as tab (tab.id)}
                <label class="tab-select-item" class:selected={tab.selected}>
                  <input type="checkbox" checked={tab.selected} onchange={() => toggleTab(tab.id)} />
                  <span class="tab-select-info">
                    <span class="tab-select-title">{tab.title}</span>
                    <span class="tab-select-url">{tab.url}</span>
                  </span>
                </label>
              {/each}
            </div>
          {:else if pendingAction.tool === "close_tabs" && pendingTabsLoading}
            <div class="tab-select-loading">{t("common.loading")}...</div>
          {/if}

          <div class="action-buttons">
            <Button class="confirm-btn" variant="primary" size="sm" onclick={confirmAction} disabled={pendingAction.tool === "close_tabs" && pendingTabs.filter(t => t.selected).length === 0}>
              {t("aiChat.confirm")}{pendingAction.tool === "close_tabs" && pendingTabs.length > 0 ? ` (${pendingTabs.filter(t => t.selected).length})` : ""}
            </Button>
            <Button class="reject-btn" variant="ghost" size="sm" onclick={rejectAction}>{t("aiChat.cancel")}</Button>
          </div>
        </div>
      {/if}
    </div>
    {#if !isAutoScroll}
      <button class="scroll-to-bottom" onclick={() => {
        if (chatContainer) {
          chatContainer.scrollTop = chatContainer.scrollHeight;
          isAutoScroll = true;
        }
      }}>↓</button>
    {/if}
  {:else}
    <div class="chat-empty">
      <EmptyState
        icon={Sparkles}
        title={t("aiChat.title")}
        description={t("aiChat.emptyState")}
      >
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
      </EmptyState>
    </div>
  {/if}

  <div class="chat-input-row">
    {#if showPresetChips}
      <div class="preset-strip" aria-label={t("aiChat.promptPresets")}>
        <div class="preset-categories">
          {#each presetGroups as [category]}
            <button
              class="preset-category"
              class:active={activePresetCategory === category}
              type="button"
              onclick={() => activePresetCategory = category}
            >
              {t(AI_PRESET_CATEGORY_LABELS[category])}
            </button>
          {/each}
        </div>
        <div class="preset-chips">
          {#each presetGroups.filter(([category]) => category === activePresetCategory) as [, presets]}
            {#each presets as preset}
              <button
                class="preset-chip"
                type="button"
                onclick={() => applyPreset(t(preset.prompt))}
                aria-label={t("aiChat.presetLabel", { label: t(preset.label) })}
              >
                <span class="preset-icon"><preset.icon size={14} /></span>
                <span>{t(preset.label)}</span>
              </button>
            {/each}
          {/each}
        </div>
      </div>
    {/if}
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
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius, 6px);
    background: var(--bg-secondary);
    overflow: hidden;
    position: relative;
  }

  .chat-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-primary);
  }

  .chat-title {
    font-weight: 700;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    color: var(--text-primary);
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
  .chat-assistant .chat-role { color: var(--success); }
  .chat-system .chat-role { color: var(--warning); }
  .chat-tool .chat-role { color: var(--cyan, #06b6d4); }

  .chat-text {
    color: var(--text-primary);
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
  .chat-text :global(strong) { color: var(--text-primary); font-weight: 700; }
  .chat-text :global(em) { font-style: italic; color: var(--text-secondary); }
  .chat-text :global(ul) {
    margin: 4px 0;
    padding-left: 18px;
    list-style: disc;
  }
  .chat-text :global(li) {
    margin: 2px 0;
  }
  .chat-text :global(code) {
    background: var(--bg-secondary);
    padding: 1px 5px;
    border-radius: 3px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: 0.9em;
  }
  .chat-text :global(pre) {
    background: var(--bg-primary);
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

  .cancel-row {
    display: flex;
    justify-content: flex-end;
    padding: 2px 0;
    position: sticky;
    bottom: 0;
  }

  .cancel-btn {
    text-transform: uppercase;
    letter-spacing: 0.3px;
    flex-shrink: 0;
  }

  .typing {
    color: var(--text-secondary);
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
  .tool-badge.success { background: color-mix(in srgb, var(--success) 28%, var(--bg)); color: var(--success); }
  .tool-badge.fail { background: color-mix(in srgb, var(--danger) 28%, var(--bg)); color: var(--danger); }

  .action-preview {
    margin: 8px 0;
    padding: 10px 12px;
    border: 1px solid var(--warning, #eab308);
    border-radius: var(--radius, 6px);
    background: color-mix(in srgb, var(--warning) 25%, var(--bg));
  }

  .action-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: calc(var(--base-font-size, 12px) * 0.917);
    margin-bottom: 6px;
    color: var(--warning, #eab308);
  }

  .action-icon {
    font-size: 1.1em;
  }

  .action-details {
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    color: var(--text-primary);
    margin-bottom: 8px;
    white-space: pre-wrap;
    line-height: 1.5;
    padding: 6px 8px;
    background: var(--bg-primary);
    border-radius: 4px;
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
  }

  .action-buttons {
    display: flex;
    gap: 8px;
  }

  .tab-select-list {
    max-height: 220px;
    overflow-y: auto;
    margin-bottom: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-primary);
  }

  .tab-select-all {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
  }

  .tab-select-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 5px 10px;
    border-bottom: 1px solid var(--border-subtle, #2a2a3a);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
    cursor: pointer;
    transition: background 0.1s;
  }

  .tab-select-item:last-child {
    border-bottom: none;
  }

  .tab-select-item:hover {
    background: var(--bg-hover);
  }

  .tab-select-item.selected {
    background: color-mix(in srgb, var(--accent) 10%, var(--bg-primary));
  }

  .tab-select-item input[type="checkbox"] {
    margin-top: 2px;
    flex-shrink: 0;
  }

  .tab-select-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .tab-select-title {
    font-weight: 600;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-select-url {
    font-family: "SF Mono", "Menlo", "Consolas", monospace;
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    color: var(--fg-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-select-loading {
    padding: 8px 10px;
    color: var(--fg-dim);
    font-size: calc(var(--base-font-size, 12px) * 0.833);
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
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 12px;
    text-align: center;
    color: var(--text-secondary);
    font-size: calc(var(--base-font-size, 12px) * 0.917);
  }


  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: center;
  }

  .chat-input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--bg-primary);
  }

  .preset-strip {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preset-categories,
  .preset-chips {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    scrollbar-width: thin;
    padding-bottom: 2px;
  }

  .preset-category,
  .preset-chip {
    border: 1px solid var(--border);
    background: var(--bg-alt);
    color: var(--fg);
    border-radius: 999px;
    white-space: nowrap;
    cursor: pointer;
  }

  .preset-category {
    padding: 5px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.72);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--fg-dim);
  }

  .preset-category.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 28%, var(--bg));
  }

  .preset-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    font-size: calc(var(--base-font-size, 12px) * 0.83);
  }

  .preset-chip:hover,
  .preset-chip:focus-visible {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 25%, var(--bg));
  }

  .preset-icon {
    font-size: 0.95rem;
  }

  @media (min-width: 720px) {
    .chat-input-row {
      gap: 8px;
    }
  }

  .chat-input {
    flex: 1;
    min-height: 40px;
    max-height: 180px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm, 4px);
    background: var(--bg-secondary);
    color: var(--text-primary);
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

  .scroll-to-bottom {
    position: absolute;
    bottom: 70px;
    right: 20px;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
    z-index: 10;
  }
  .scroll-to-bottom:hover {
    background: var(--bg-primary);
  }

  .chat-input::placeholder { color: var(--fg-muted); }
  .chat-input:disabled { color: var(--fg-muted); }

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


  /* existing styles should not be affected, appending new styles for pro-metadata */
  .pro-metadata {
    margin-top: 8px;
    font-size: calc(var(--base-font-size, 12px) * 0.85);
    background: var(--bg-alt);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
  }
  .pro-metadata summary {
    cursor: pointer;
    font-weight: 600;
    color: var(--text-secondary);
    user-select: none;
  }
  .pro-metadata-content {
    margin-top: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--text-primary);
  }

</style>
