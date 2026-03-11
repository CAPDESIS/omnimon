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
    ipcCheckApiKey: vi.fn(async () => true),
    ipcClearAiCache: vi.fn(async () => undefined),
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
    setPollingTarget: vi.fn(),
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
    pollIntervalMs: writable(2000),
    automationIntervalSecs: writable(5),
    aiCacheTtlMinutes: writable(5),
    activeProfilePreset: writable("general"),
    profilePresets: writable([
      { id: "general", label: "General", idleThreshold: 1, pollIntervalMs: 2000, automationIntervalSecs: 5, aiProfile: "general" },
    ]),
    theme: writable("auto"),
    userMode: writable("pro"),
    tabPanelHeight: writable(160),
    aiChatPanelHeight: writable(220),
    networkPanelHeight: writable(280),
    localePreference: writable("en"),
    customTheme: writable(null),
    profilesCollapsedStore: writable(false),
    mainTableCollapsedStore: writable(false),
    networkMapCollapsedStore: writable(false),
    browserTabsCollapsedStore: writable(false),
    aiChatCollapsedStore: writable(false),
    aiConfigCollapsedStore: writable(false),
    displayName: writable("User"),
    profilePreset: writable("balanced"),
    layoutModeStore: writable("tabs"),
    dashboardLayout: writable("standard"),
    refreshInterval: writable(2000),
    favoriteProcesses: writable([]),
    notificationLevel: writable("all"),
    loadPreferences: vi.fn(async () => {}),
    savePreferences: vi.fn(async () => {}),
    initPreferenceSubscriptions: vi.fn(() => () => {}),
    increaseFontSize: vi.fn(),
    decreaseFontSize: vi.fn(),
    moveColumnUp: vi.fn(),
    moveColumnDown: vi.fn(),
    MIN_IDLE_THRESHOLD: 0.1,
    MAX_IDLE_THRESHOLD: 10,
    MIN_AI_CACHE_TTL_MINUTES: 0,
    MAX_AI_CACHE_TTL_MINUTES: 60,
    applyProfilePresetById: vi.fn(() => true),
    syncAiProfileToPreset: vi.fn(),
    setProfilePresets: vi.fn(),
  };
});

vi.mock("../components/CloudSync.svelte", () => ({
  default: () => ({
    $$render: () => '<div data-testid="cloud-sync">cloud sync</div>',
  }),
}));

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
  networkAlerts: writable([]),
  networkAlertFilter: writable({ severity: "all", query: "" }),
  smartAlerts: writable([]),
  evaluateAlerts: vi.fn(),
  addAlertRule: vi.fn(),
  removeAlertRule: vi.fn(),
  clearFiredAlerts: vi.fn(),
  clearNetworkAlerts: vi.fn(),
  clearDynamicAlerts: vi.fn(),
  investigateNetworkAlert: vi.fn(),
  askAiAboutNetworkAlert: vi.fn(),
  matchesNetworkAlertFilter: vi.fn(() => true),
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
    await fireEvent.click(screen.getByRole("tab", { name: /AI Actions/i }));
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
    await waitFor(() => {
      expect(screen.getByText("How OmniMon works")).toBeInTheDocument();
    });
  });

  it("opens deep-dive modal from dashboard cards", async () => {
    render(App);
    const dashboardButtons = Array.from(document.querySelectorAll(".dashboard .metric-button"));
    await fireEvent.click(dashboardButtons[2] as HTMLButtonElement);
    await waitFor(() => {
      expect(screen.getByText("Deep Dive")).toBeInTheDocument();
    });
  });

  it("shows workspace mode selector in settings", async () => {
    render(App);
    const settingsButton = document.querySelector('.toolbar-actions button[title="AI Settings"]') as HTMLButtonElement;
    await fireEvent.click(settingsButton);
    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "OmniMon Settings" })).toBeInTheDocument();
    });
  });

  it("renders network basic mode hint when simplified mode is active", async () => {
    const { userMode } = await import("../stores/preferences");
    userMode.set("basic");
    render(App);
    // Navigate to the network tab first (hint is only visible in network view)
    const networkTab = screen.getByRole("tab", { name: /Network/i });
    await fireEvent.click(networkTab);
    await waitFor(() => {
      expect(screen.getAllByText(/unlock Network Map, deep diagnostics/i).length).toBeGreaterThan(0);
    });
  });
});
