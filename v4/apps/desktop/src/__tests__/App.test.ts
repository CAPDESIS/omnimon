import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { writable, derived } from "svelte/store";
import App from "../App.svelte";
import { ipcAnalyzeContext } from "../lib/ipc";

vi.mock("../lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/ipc")>();
  return {
    ...actual,
    ipcAnalyzeContext: vi.fn(async () => ""),
    ipcValidateApiKey: vi.fn(async () => true),
  };
});

vi.mock("../stores/processes", () => {
  const processes = writable([
    {
      pid: 101,
      name: "Chrome",
      exec_name: "Google Chrome Helper",
      ram_mb: 512,
      cpu_pct: 12.3,
      uptime: "1h",
      group: "Browser",
      is_system: false,
      idle: false,
      state: "R",
    },
  ]);
  const filtered = writable([
    {
      pid: 101,
      name: "Chrome",
      exec_name: "Google Chrome Helper",
      ram_mb: 512,
      cpu_pct: 12.3,
      uptime: "1h",
      group: "Browser",
      is_system: false,
      idle: false,
      state: "R",
    },
  ]);
  const loading = writable(false);
  const search = writable("");
  const selectedPids = writable(new Set<number>());
  const selectedCount = derived(selectedPids, ($s) => $s.size);
  const selectedRamMB = writable(0);
  const focusedPid = writable<number | null>(null);
  const grouping = writable(false);
  const aiSuggestions = writable([]);
  const aiLoading = writable(false);
  const aiError = writable<string | null>(null);
  const aiProfile = writable("general");
  const stats = writable({
    ram_total_gb: 16,
    ram_used_pct: 40,
    swap_used_mb: 0,
    total_processes: 1,
    net_rx_bytes_per_sec: 1200,
    net_tx_bytes_per_sec: 800,
  });
  const browserTabs = writable([]);
  const chromeProcesses = derived(processes, ($p) => $p.filter((x) => x.group === "Browser"));

  return {
    processes,
    filtered,
    loading,
    search,
    selectedPids,
    selectedCount,
    selectedRamMB,
    focusedPid,
    grouping,
    startPolling: vi.fn(),
    stopPolling: vi.fn(),
    killSelected: vi.fn(async () => []),
    killSingle: vi.fn(async () => true),
    selectAllVisible: vi.fn(),
    selectNone: vi.fn(),
    aiSuggestions,
    aiLoading,
    aiError,
    aiProfile,
    analyzeWithAi: vi.fn(async () => {}),
    saveAiConfigAction: vi.fn(async () => {}),
    dismissAiSuggestions: vi.fn(),
    stats,
    browserTabs,
    chromeProcesses,
  };
});

vi.mock("../stores/preferences", () => {
  return {
    fontSize: writable(12),
    columns: writable({
      name: true,
      detail: true,
      group: true,
      ram: true,
      cpu: true,
      energy: true,
      network: true,
      uptime: true,
      pid: true,
      state: true,
    }),
    columnOrder: writable(["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"]),
    aiProviderConfig: writable({ provider: "openrouter", model: "meta-llama/llama-3.2-3b-instruct:free" }),
    idleThreshold: writable(1),
    theme: writable("auto"),
    tabPanelHeight: writable(160),
    aiChatPanelHeight: writable(220),
    networkPanelHeight: writable(280),
    localePreference: writable("en"),
    loadPreferences: vi.fn(async () => {}),
    savePreferences: vi.fn(async () => {}),
    initPreferenceSubscriptions: vi.fn(() => () => {}),
    increaseFontSize: vi.fn(),
    decreaseFontSize: vi.fn(),
    moveColumnUp: vi.fn(),
    moveColumnDown: vi.fn(),
    MIN_IDLE_THRESHOLD: 0.1,
    MAX_IDLE_THRESHOLD: 10,
  };
});

vi.mock("../stores/metricsHistory", () => ({
  metricsHistory: writable([]),
  pushMetrics: vi.fn(),
  cpuSeries: writable([]),
  ramSeries: writable([]),
  netRxSeries: writable([]),
  netTxSeries: writable([]),
  swapSeries: writable([]),
  _resetMetricsHistory: vi.fn(),
}));

vi.mock("../stores/alerts", () => ({
  alertRules: writable([]),
  firedAlerts: writable([]),
  dynamicAlerts: writable([]),
  smartAlerts: writable([]),
  evaluateAlerts: vi.fn(),
  addAlertRule: vi.fn(),
  removeAlertRule: vi.fn(),
  clearFiredAlerts: vi.fn(),
  clearDynamicAlerts: vi.fn(),
  dismissSmartAlert: vi.fn(),
  initSecurityAlertListener: vi.fn().mockResolvedValue(() => {}),
  _resetAlerts: vi.fn(),
}));

vi.mock("../stores/toasts", () => ({
  toasts: writable([]),
  addToast: vi.fn(() => "toast-1"),
  dismissToast: vi.fn(),
  toast: {
    info: vi.fn(),
    success: vi.fn(),
    warning: vi.fn(),
    error: vi.fn(),
  },
  _resetToasts: vi.fn(),
}));

const mockAnalyzeContext = vi.mocked(ipcAnalyzeContext);

describe("App AI Command Bar", () => {
  beforeEach(() => {
    mockAnalyzeContext.mockReset();
  });

  it("renders the AI command bar with input", async () => {
    render(App);
    // The AiCommandBar uses a different placeholder
    const input = screen.getByPlaceholderText(/Alert me if Chrome/i);
    expect(input).toBeInTheDocument();
  });

  it("renders the toolbar with search and controls", async () => {
    render(App);
    const searchInput = screen.getByPlaceholderText(/Filter by name/i);
    expect(searchInput).toBeInTheDocument();
    expect(screen.getByText("AI Analyze")).toBeInTheDocument();
    expect(screen.getByText("Close")).toBeInTheDocument();
  });

  it("opens the help center from the toolbar", async () => {
    render(App);
    await fireEvent.click(screen.getByRole("button", { name: /Help Center/i }));
    expect(screen.getByText("How OmniMon works")).toBeInTheDocument();
  });

  it("opens deep-dive modal from dashboard cards", async () => {
    render(App);
    await fireEvent.click(screen.getAllByRole("button", { name: /Network/i })[0]);
    expect(screen.getByText("Deep Dive")).toBeInTheDocument();
  });
});
