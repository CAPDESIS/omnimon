import { get } from "svelte/store";
import {
  fontSize,
  columns,
  columnOrder,
  aiProviderConfig,
  idleThreshold,
  pollIntervalMs,
  automationIntervalSecs,
  aiCacheTtlMinutes,
  activeProfilePreset,
  profilePresets,
  networkAlertRules,
  theme,
  userMode,
  tabPanelHeight,
  localePreference,
  customTheme,
  moveColumnToIndex,
  applyProfilePresetById,
  syncAiProfileToPreset,
  setProfilePresets,
  loadPreferences,
  savePreferences,
  initPreferenceSubscriptions,
  increaseFontSize,
  decreaseFontSize,
  moveColumnUp,
  moveColumnDown,
  DEFAULT_COLUMNS,
  DEFAULT_AI_CONFIG,
  DEFAULT_IDLE_THRESHOLD,
  DEFAULT_POLL_INTERVAL_MS,
  DEFAULT_AUTOMATION_INTERVAL_SECS,
  DEFAULT_AI_CACHE_TTL_MINUTES,
  DEFAULT_PROFILE_PRESETS,
  DEFAULT_NETWORK_ALERT_RULES,
  DEFAULT_USER_MODE,
  DEFAULT_LOCALE,
  MIN_IDLE_THRESHOLD,
  MAX_IDLE_THRESHOLD,
  MIN_FONT_SIZE,
  MAX_FONT_SIZE,
} from "../preferences";
import { load } from "@tauri-apps/plugin-store";
import { getCustomThemeOverrides } from "../../lib/theme";

vi.mock("../../lib/ipc", async () => {
  const actual = await vi.importActual<typeof import("../../lib/ipc")>("../../lib/ipc");
  return {
    ...actual,
    ipcSetNetworkAlertRules: vi.fn().mockResolvedValue(1),
  };
});

// Mock the tauri store
const mockStore = {
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
};

const mockLoad = vi.mocked(load);

vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn(() => Promise.resolve(mockStore)),
}));

beforeEach(() => {
  vi.clearAllMocks();
  mockStore.get.mockReset();
  mockStore.set.mockReset();
  mockStore.save.mockReset();
  // Reset stores to defaults
  fontSize.set(12);
  columns.set({ ...DEFAULT_COLUMNS });
  aiProviderConfig.set({ ...DEFAULT_AI_CONFIG });
  columnOrder.set(["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"]);
  idleThreshold.set(DEFAULT_IDLE_THRESHOLD);
  pollIntervalMs.set(DEFAULT_POLL_INTERVAL_MS);
  automationIntervalSecs.set(DEFAULT_AUTOMATION_INTERVAL_SECS);
  aiCacheTtlMinutes.set(DEFAULT_AI_CACHE_TTL_MINUTES);
  activeProfilePreset.set("general");
  profilePresets.set([...DEFAULT_PROFILE_PRESETS]);
  networkAlertRules.set([...DEFAULT_NETWORK_ALERT_RULES]);
  theme.set("auto");
  tabPanelHeight.set(160);
  localePreference.set(DEFAULT_LOCALE);
  userMode.set(DEFAULT_USER_MODE);
  customTheme.set(null);
});

describe("loadPreferences", () => {
  it("loads saved font size", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "fontSize") return 16;
      return undefined;
    });
    await loadPreferences();
    expect(get(fontSize)).toBe(16);
  });

  it("ignores font size outside valid range", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "fontSize") return 50;
      return undefined;
    });
    await loadPreferences();
    expect(get(fontSize)).toBe(12); // default
  });

  it("ignores non-number font size", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "fontSize") return "big";
      return undefined;
    });
    await loadPreferences();
    expect(get(fontSize)).toBe(12);
  });

  it("loads saved columns config", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "columns") return { ...DEFAULT_COLUMNS, ram: false, pid: false };
      return undefined;
    });
    await loadPreferences();
    const cols = get(columns);
    expect(cols.ram).toBe(false);
    expect(cols.pid).toBe(false);
    expect(cols.name).toBe(true);
  });

  it("merges partial columns with defaults", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "columns") return { cpu: false }; // only cpu specified
      return undefined;
    });
    await loadPreferences();
    const cols = get(columns);
    expect(cols.cpu).toBe(false);
    expect(cols.name).toBe(true);
    expect(cols.ram).toBe(true);
  });

  it("loads saved AI provider config", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "aiProviderConfig") return { provider: "openai", model: "gpt-4o" };
      return undefined;
    });
    await loadPreferences();
    const ai = get(aiProviderConfig);
    expect(ai.provider).toBe("openai");
    expect(ai.model).toBe("gpt-4o");
  });

  it("uses defaults when store returns null/undefined", async () => {
    mockStore.get.mockResolvedValue(undefined);
    await loadPreferences();
    expect(get(fontSize)).toBe(12);
    expect(get(columns)).toEqual(DEFAULT_COLUMNS);
    expect(get(aiProviderConfig)).toEqual(DEFAULT_AI_CONFIG);
  });

  it("loads and sanitizes columnOrder, appending missing keys", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "columnOrder") return ["cpu", "name", "invalid-key"];
      return undefined;
    });

    await loadPreferences();

    const order = get(columnOrder);
    expect(order[0]).toBe("cpu");
    expect(order[1]).toBe("name");
    expect(order).toContain("pid");
    expect(order).toContain("state");
    expect(order).not.toContain("invalid-key" as never);
  });

  it("loads idle threshold, theme, and tab panel height when valid", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "idleThreshold") return 2.5;
      if (key === "pollIntervalMs") return 1500;
      if (key === "automationIntervalSecs") return 10;
      if (key === "activeProfilePreset") return "developer";
      if (key === "profilePresets") return DEFAULT_PROFILE_PRESETS;
      if (key === "theme") return "dark";
      if (key === "userMode") return "basic";
      if (key === "tabPanelHeight") return 240;
      if (key === "aiCacheTtlMinutes") return 15;
      if (key === "networkAlertRules") return DEFAULT_NETWORK_ALERT_RULES;
      return undefined;
    });

    await loadPreferences();

    expect(get(idleThreshold)).toBe(2.5);
    expect(get(pollIntervalMs)).toBe(1500);
    expect(get(automationIntervalSecs)).toBe(10);
    expect(get(activeProfilePreset)).toBe("developer");
    expect(get(aiCacheTtlMinutes)).toBe(15);
    expect(get(networkAlertRules)).toEqual(DEFAULT_NETWORK_ALERT_RULES);
    expect(get(theme)).toBe("dark");
    expect(get(userMode)).toBe("basic");
    expect(get(tabPanelHeight)).toBe(240);
  });

  it("ignores out-of-range and invalid preference values", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "idleThreshold") return MAX_IDLE_THRESHOLD + 1;
      if (key === "theme") return "neon";
      if (key === "tabPanelHeight") return 5;
      if (key === "aiProviderConfig") return { provider: 42, model: null };
      return undefined;
    });

    await loadPreferences();

    expect(get(idleThreshold)).toBe(DEFAULT_IDLE_THRESHOLD);
    expect(get(theme)).toBe("auto");
    expect(get(tabPanelHeight)).toBe(160);
    expect(get(aiProviderConfig)).toEqual(DEFAULT_AI_CONFIG);
  });

  it("sanitiza reglas de alertas de red invalidas y deduplica ids", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "networkAlertRules") {
        return [
          {
            id: "dup",
            name: "Bandwidth",
            enabled: true,
            severity: "warning",
            cooldown_seconds: 12.3,
            notify_ai: true,
            condition: {
              kind: "high_bandwidth",
              threshold_mbps: 0.01,
              direction: "both",
              process: " chrome ",
            },
          },
          {
            id: "dup",
            name: "Duplicate should be ignored",
            enabled: true,
            severity: "warning",
            cooldown_seconds: 20,
            notify_ai: false,
            condition: {
              kind: "new_external_connection",
              exclude_known: false,
            },
          },
          {
            id: "bad",
            name: "",
            enabled: true,
            severity: "wat",
            cooldown_seconds: 10,
            notify_ai: false,
            condition: { kind: "unusual_port", suspicious_ports: [] },
          },
        ];
      }
      return undefined;
    });

    await loadPreferences();

    expect(get(networkAlertRules)).toHaveLength(1);
    expect(get(networkAlertRules)[0].condition.kind).toBe("high_bandwidth");
    if (get(networkAlertRules)[0].condition.kind === "high_bandwidth") {
      expect(get(networkAlertRules)[0].condition.threshold_mbps).toBe(0.1);
      expect(get(networkAlertRules)[0].condition.process).toBe("chrome");
    }
  });

  it("sanitiza suspicious_destination y connection_count_exceeded", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "networkAlertRules") {
        return [
          {
            id: "dest",
            name: "Destination",
            enabled: true,
            severity: "critical",
            cooldown_seconds: 10,
            notify_ai: true,
            condition: {
              kind: "suspicious_destination",
              patterns: [" evil ", "", 42],
            },
          },
          {
            id: "conn",
            name: "Connections",
            enabled: true,
            severity: "warning",
            cooldown_seconds: 10,
            notify_ai: false,
            condition: {
              kind: "connection_count_exceeded",
              max_connections: 200.8,
              process: " Safari ",
            },
          },
        ];
      }
      return undefined;
    });

    await loadPreferences();

    expect(get(networkAlertRules)).toHaveLength(2);
    expect(get(networkAlertRules)[0].condition.kind).toBe("suspicious_destination");
    expect(get(networkAlertRules)[1].condition.kind).toBe("connection_count_exceeded");
    if (get(networkAlertRules)[0].condition.kind === "suspicious_destination") {
      expect(get(networkAlertRules)[0].condition.patterns).toEqual(["evil"]);
    }
    if (get(networkAlertRules)[1].condition.kind === "connection_count_exceeded") {
      expect(get(networkAlertRules)[1].condition.max_connections).toBe(201);
      expect(get(networkAlertRules)[1].condition.process).toBe("Safari");
    }
  });

  it("cae a defaults cuando profilePresets sanitizados quedan vacios", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "profilePresets") {
        return [
          { id: "", label: "Bad", idleThreshold: 1, pollIntervalMs: 1, automationIntervalSecs: 1, aiProfile: "general" },
        ];
      }
      return undefined;
    });

    await loadPreferences();

    expect(get(profilePresets)).toEqual(DEFAULT_PROFILE_PRESETS);
    expect(get(activeProfilePreset)).toBe("general");
  });

  it("carga customTheme valido y lo sincroniza", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "customTheme") {
        return {
          name: "Ocean",
          base: "light",
          overrides: { "--accent": "#0055ff" },
        };
      }
      return undefined;
    });

    await loadPreferences();

    expect(get(customTheme)).toEqual({
      name: "Ocean",
      base: "light",
      overrides: { "--accent": "#0055ff" },
    });
    expect(getCustomThemeOverrides()).toEqual({
      name: "Ocean",
      base: "light",
      overrides: { "--accent": "#0055ff" },
    });
  });

  it("carga alturas y colapsados validos", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "networkPanelHeight") return 333;
      if (key === "aiChatPanelHeight") return 444;
      if (key === "localePreference") return "es";
      if (key === "profilesCollapsed") return true;
      if (key === "mainTableCollapsed") return true;
      if (key === "networkMapCollapsed") return true;
      if (key === "browserTabsCollapsed") return true;
      if (key === "aiChatCollapsed") return true;
      if (key === "aiConfigCollapsed") return true;
      return undefined;
    });

    const prefs = await import("../preferences");
    await prefs.loadPreferences();

    expect(get(prefs.localePreference)).toBe("es");
    expect(get(prefs.networkPanelHeight)).toBe(333);
    expect(get(prefs.aiChatPanelHeight)).toBe(444);
    expect(get(prefs.profilesCollapsedStore)).toBe(true);
    expect(get(prefs.mainTableCollapsedStore)).toBe(true);
    expect(get(prefs.networkMapCollapsedStore)).toBe(true);
    expect(get(prefs.browserTabsCollapsedStore)).toBe(true);
    expect(get(prefs.aiChatCollapsedStore)).toBe(true);
    expect(get(prefs.aiConfigCollapsedStore)).toBe(true);
  });

  it("handles plugin-store unavailability gracefully", async () => {
    mockLoad.mockRejectedValueOnce(new Error("plugin not available"));
    await expect(loadPreferences()).resolves.toBeUndefined();
  });

  it("falls back to defaults when store read throws", async () => {
    mockStore.get.mockRejectedValue(new Error("read failed"));
    await expect(loadPreferences()).resolves.toBeUndefined();
    expect(get(fontSize)).toBe(12);
  });
});

describe("savePreferences", () => {
  it("saves all preference values to store", async () => {
    fontSize.set(14);
    columns.set({ ...DEFAULT_COLUMNS, group: false });
    aiProviderConfig.set({ provider: "anthropic", model: "claude-sonnet-4-20250514" });
    pollIntervalMs.set(1200);
    automationIntervalSecs.set(9);
    aiCacheTtlMinutes.set(12);
    activeProfilePreset.set("developer");
    userMode.set("basic");

    await savePreferences();

    expect(mockStore.set).toHaveBeenCalledWith("fontSize", 14);
    expect(mockStore.set).toHaveBeenCalledWith("columns", expect.objectContaining({ group: false }));
    expect(mockStore.set).toHaveBeenCalledWith(
      "aiProviderConfig",
      expect.objectContaining({ provider: "anthropic" }),
    );
    expect(mockStore.set).toHaveBeenCalledWith("pollIntervalMs", 1200);
    expect(mockStore.set).toHaveBeenCalledWith("automationIntervalSecs", 9);
    expect(mockStore.set).toHaveBeenCalledWith("aiCacheTtlMinutes", 12);
    expect(mockStore.set).toHaveBeenCalledWith("activeProfilePreset", "developer");
    expect(mockStore.set).toHaveBeenCalledWith("userMode", "basic");
    expect(mockStore.set).toHaveBeenCalledWith("networkAlertRules", expect.any(Array));
    expect(mockStore.save).toHaveBeenCalled();
  });

  it("no-ops when plugin-store is unavailable", async () => {
    mockLoad.mockRejectedValueOnce(new Error("plugin missing"));
    await expect(savePreferences()).resolves.toBeUndefined();
  });

  it("swallows persistence failures", async () => {
    mockStore.set.mockRejectedValueOnce(new Error("disk full"));
    await expect(savePreferences()).resolves.toBeUndefined();
  });
});

describe("initPreferenceSubscriptions", () => {
  it("returns an unsubscribe function", () => {
    const unsub = initPreferenceSubscriptions();
    expect(typeof unsub).toBe("function");
    unsub();
  });

  it("debounces saves on store changes", async () => {
    vi.useFakeTimers();
    const unsub = initPreferenceSubscriptions();

    // Trigger multiple rapid changes
    fontSize.set(14);
    fontSize.set(15);
    fontSize.set(16);

    // Before debounce fires
    expect(mockStore.set).not.toHaveBeenCalledWith("fontSize", 16);

    // After debounce
    await vi.advanceTimersByTimeAsync(600);
    expect(mockStore.set).toHaveBeenCalledWith("fontSize", 16);

    unsub();
    vi.useRealTimers();
  });

  it("customTheme subscription sincroniza overrides", () => {
    const unsub = initPreferenceSubscriptions();

    customTheme.set({
      name: "Night",
      base: "dark",
      overrides: { "--accent": "#abc123" },
    });

    expect(getCustomThemeOverrides()).toEqual({
      name: "Night",
      base: "dark",
      overrides: { "--accent": "#abc123" },
    });

    unsub();
  });
});

describe("font size helpers", () => {
  it("increaseFontSize increments by 1", () => {
    fontSize.set(12);
    increaseFontSize();
    expect(get(fontSize)).toBe(13);
  });

  it("increaseFontSize caps at MAX_FONT_SIZE", () => {
    fontSize.set(MAX_FONT_SIZE);
    increaseFontSize();
    expect(get(fontSize)).toBe(MAX_FONT_SIZE);
  });

  it("decreaseFontSize decrements by 1", () => {
    fontSize.set(12);
    decreaseFontSize();
    expect(get(fontSize)).toBe(11);
  });

  it("decreaseFontSize floors at MIN_FONT_SIZE", () => {
    fontSize.set(MIN_FONT_SIZE);
    decreaseFontSize();
    expect(get(fontSize)).toBe(MIN_FONT_SIZE);
  });
});

describe("column ordering helpers", () => {
  it("moveColumnUp reorders a non-first column", () => {
    columnOrder.set(["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"]);
    moveColumnUp("group");
    expect(get(columnOrder).slice(0, 4)).toEqual(["name", "group", "detail", "ram"]);
  });

  it("moveColumnUp no-ops when column is first", () => {
    const before = get(columnOrder);
    moveColumnUp("name");
    expect(get(columnOrder)).toEqual(before);
  });

  it("moveColumnDown reorders a non-last column", () => {
    columnOrder.set(["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"]);
    moveColumnDown("detail");
    expect(get(columnOrder).slice(0, 4)).toEqual(["name", "group", "detail", "ram"]);
  });

  it("moveColumnDown no-ops for missing or last column", () => {
    const before = get(columnOrder);
    moveColumnDown("state");
    expect(get(columnOrder)).toEqual(before);

    moveColumnDown("missing" as never);
    expect(get(columnOrder)).toEqual(before);
  });

  it("moveColumnToIndex mueve y clamp target index", () => {
    columnOrder.set(["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"]);
    moveColumnToIndex("cpu", 1);
    expect(get(columnOrder).slice(0, 4)).toEqual(["name", "cpu", "detail", "group"]);

    moveColumnToIndex("name", 999);
    expect(get(columnOrder).at(-1)).toBe("name");
  });

  it("moveColumnToIndex no-op when key missing or target igual", () => {
    const before = get(columnOrder);
    moveColumnToIndex("missing" as never, 2);
    expect(get(columnOrder)).toEqual(before);

    moveColumnToIndex("name", 0);
    expect(get(columnOrder)).toEqual(before);
  });
});

describe("idle threshold bounds", () => {
  it("keeps valid minimum idle threshold", async () => {
    mockStore.get.mockImplementation((key: string) => {
      if (key === "idleThreshold") return MIN_IDLE_THRESHOLD;
      return undefined;
    });
    await loadPreferences();
    expect(get(idleThreshold)).toBe(MIN_IDLE_THRESHOLD);
  });
});

describe("module fallback paths", () => {
  it("returns early from load/save when plugin-store cannot load", async () => {
    vi.resetModules();
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(async () => {
        throw new Error("plugin unavailable");
      }),
    }));

    const prefs = await import("../preferences");
    await expect(prefs.loadPreferences()).resolves.toBeUndefined();
    await expect(prefs.savePreferences()).resolves.toBeUndefined();
  });
});

describe("profile preset helpers", () => {
  it("applyProfilePresetById actualiza stores y devuelve true", () => {
    expect(applyProfilePresetById("developer")).toBe(true);
    expect(get(activeProfilePreset)).toBe("developer");
    expect(get(idleThreshold)).toBe(DEFAULT_PROFILE_PRESETS[1].idleThreshold);
    expect(get(pollIntervalMs)).toBe(DEFAULT_PROFILE_PRESETS[1].pollIntervalMs);
  });

  it("applyProfilePresetById devuelve false para preset inexistente", () => {
    expect(applyProfilePresetById("missing")).toBe(false);
  });

  it("syncAiProfileToPreset aplica preset asociado cuando existe", () => {
    syncAiProfileToPreset("battery");
    expect(get(activeProfilePreset)).toBe("battery");
  });

  it("setProfilePresets sanitiza y usa fallback cuando activo no existe", () => {
    activeProfilePreset.set("missing");
    setProfilePresets([
      {
        id: " custom ",
        label: "Custom preset",
        idleThreshold: 99,
        pollIntervalMs: 999_999,
        automationIntervalSecs: 0,
        aiProfile: "developer",
      },
      {
        id: "custom",
        label: "Duplicate",
        idleThreshold: 0.2,
        pollIntervalMs: 1000,
        automationIntervalSecs: 5,
        aiProfile: "general",
      },
    ] as any);

    expect(get(profilePresets)).toHaveLength(1);
    expect(get(profilePresets)[0].id).toBe("custom");
    expect(get(profilePresets)[0].idleThreshold).toBe(MAX_IDLE_THRESHOLD);
    expect(get(profilePresets)[0].pollIntervalMs).toBe(10_000);
    expect(get(profilePresets)[0].automationIntervalSecs).toBe(1);
    expect(get(activeProfilePreset)).toBe("custom");
  });
});
