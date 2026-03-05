import { get } from "svelte/store";
import {
  fontSize,
  columns,
  aiProviderConfig,
  loadPreferences,
  savePreferences,
  initPreferenceSubscriptions,
  increaseFontSize,
  decreaseFontSize,
  DEFAULT_COLUMNS,
  DEFAULT_AI_CONFIG,
  MIN_FONT_SIZE,
  MAX_FONT_SIZE,
} from "../preferences";

// Mock the tauri store
const mockStore = {
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
};

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
});

describe("savePreferences", () => {
  it("saves all preference values to store", async () => {
    fontSize.set(14);
    columns.set({ ...DEFAULT_COLUMNS, group: false });
    aiProviderConfig.set({ provider: "anthropic", model: "claude-sonnet-4-20250514" });

    await savePreferences();

    expect(mockStore.set).toHaveBeenCalledWith("fontSize", 14);
    expect(mockStore.set).toHaveBeenCalledWith("columns", expect.objectContaining({ group: false }));
    expect(mockStore.set).toHaveBeenCalledWith(
      "aiProviderConfig",
      expect.objectContaining({ provider: "anthropic" }),
    );
    expect(mockStore.save).toHaveBeenCalled();
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
