import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import AIChat from "../AIChat.svelte";
import type { ChatResponse, ProcessEntry } from "../../lib/types";

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 101,
    name: "Chrome",
    exec_name: "/Applications/Chrome",
    exe_path: "/Applications/Chrome",
    bundle_id: null,
    icon_data_url: null,
    ram_mb: 512,
    cpu_pct: 12,
    disk_read_mb: 0,
    disk_write_mb: 0,
    net_rx_bytes_per_sec: 0,
    net_tx_bytes_per_sec: 0,
    energy_impact_score: 3,
    uptime: "1h",
    group: "Browser",
    group_key: "browser:chrome",
    group_identity_type: "normalized_name",
    grouped_name: "Chrome",
    process_count: 1,
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

const {
  mockAiChat,
  mockGetBrowserTabs,
  mockCloseBrowserTab,
  mockKillProcess,
  mockKillProcesses,
  mockToast,
  mockInspectSet,
  mockDetectPromptInjection,
  mockProcesses,
  mockAiProviderConfig,
} = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockAiChat: vi.fn<(...args: unknown[]) => Promise<ChatResponse>>(),
    mockGetBrowserTabs: vi.fn(),
    mockCloseBrowserTab: vi.fn(),
    mockKillProcess: vi.fn(),
    mockKillProcesses: vi.fn(),
    mockToast: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    },
    mockInspectSet: vi.fn(),
    mockDetectPromptInjection: vi.fn(() => false),
    mockProcesses: writable<ProcessEntry[]>([]),
    mockAiProviderConfig: writable({ provider: "openrouter", model: "test-model" }),
  };
});

vi.mock("../../lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...actual,
    ipcAiChat: mockAiChat,
    ipcGetBrowserTabs: mockGetBrowserTabs,
    ipcCloseBrowserTab: mockCloseBrowserTab,
    ipcKillProcess: mockKillProcess,
    ipcKillProcesses: mockKillProcesses,
  };
});

vi.mock("../../stores/preferences", () => ({
  aiProviderConfig: mockAiProviderConfig,
}));

vi.mock("../../stores/processes", () => ({
  processes: mockProcesses,
}));

vi.mock("../../stores/uiActions", () => ({
  inspectProcessRequest: {
    set: mockInspectSet,
  },
}));

vi.mock("../../stores/toasts", () => ({
  toast: mockToast,
}));

vi.mock("../../lib/aiConfigBridge", () => ({
  detectPromptInjection: mockDetectPromptInjection,
}));

vi.mock("../../lib/markdown", () => ({
  renderMarkdown: (value: string) => value,
}));

vi.mock("../../lib/chatUtils", () => ({
  scrollToBottom: vi.fn(),
  resizeInput: vi.fn(),
}));

describe("AIChat", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockProcesses.set([makeProc()]);
    mockAiProviderConfig.set({ provider: "openrouter", model: "test-model" });
    mockAiChat.mockReset();
    mockAiChat.mockResolvedValue({ reply: "All good", tool_call: null });
    mockGetBrowserTabs.mockReset();
    mockCloseBrowserTab.mockReset();
    mockKillProcess.mockReset();
    mockKillProcesses.mockReset();
    mockToast.success.mockClear();
    mockToast.error.mockClear();
    mockToast.warning.mockClear();
    mockToast.info.mockClear();
    mockInspectSet.mockClear();
    mockDetectPromptInjection.mockClear();
    mockDetectPromptInjection.mockReturnValue(false);
  });

  it("renderiza sin errores", () => {
    render(AIChat);

    expect(screen.getByRole("region", { name: "AI Chat" })).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/ask ai to act/i)).toBeInTheDocument();
    expect(screen.getByText("Send")).toBeInTheDocument();
  });

  it("muestra mensajes en la lista", async () => {
    render(AIChat);

    const input = screen.getByPlaceholderText(/ask ai to act/i);
    await fireEvent.input(input, { target: { value: "What uses most RAM?" } });
    await fireEvent.click(screen.getByText("Send"));

    await waitFor(() => {
      expect(screen.getByText("What uses most RAM?")).toBeInTheDocument();
      expect(screen.getByText("All good")).toBeInTheDocument();
      expect(screen.getByText("You")).toBeInTheDocument();
      expect(screen.getByText("AI")).toBeInTheDocument();
    });
  });

  it("envia mensaje al presionar Enter", async () => {
    render(AIChat);

    const input = screen.getByPlaceholderText(/ask ai to act/i);
    await fireEvent.input(input, { target: { value: "Kill Chrome" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(mockAiChat).toHaveBeenCalledWith(
        "Kill Chrome",
        "openrouter",
        "test-model",
        expect.arrayContaining([expect.arrayContaining(["system", expect.any(String)])]),
      );
    });
  });

  it("muestra indicador de loading mientras espera respuesta", async () => {
    const pending = deferred<ChatResponse>();
    mockAiChat.mockReturnValueOnce(pending.promise);

    render(AIChat);

    const input = screen.getByPlaceholderText(/ask ai to act/i);
    await fireEvent.input(input, { target: { value: "Analyze network traffic" } });
    await fireEvent.click(screen.getByText("Send"));

    expect(screen.getByText("Thinking")).toBeInTheDocument();
    expect(screen.getByText("Cancel")).toBeInTheDocument();

    pending.resolve({ reply: "Done", tool_call: null });

    await waitFor(() => {
      expect(screen.queryByText("Thinking")).not.toBeInTheDocument();
      expect(screen.getByText("Done")).toBeInTheDocument();
    });
  });

  it("maneja error de API gracefulmente", async () => {
    mockAiChat.mockRejectedValueOnce(new Error("fetch failed"));

    render(AIChat);

    const input = screen.getByPlaceholderText(/ask ai to act/i);
    await fireEvent.input(input, { target: { value: "Hello" } });
    await fireEvent.click(screen.getByText("Send"));

    await waitFor(() => {
      // After i18n integration, error messages use translated keys
      const errorArea = screen.getByRole("region", { name: "AI Chat" });
      expect(errorArea.textContent).toMatch(/error|Error|API|ollama/i);
    });
  });
});
