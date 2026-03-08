import { writable, get } from "svelte/store";
import type { LocaleCode } from "../lib/i18n";
import type { CustomThemeOverrides } from "../lib/theme";
import { setCustomThemeOverrides } from "../lib/theme";

export interface ColumnConfig {
  name: boolean;
  detail: boolean;
  group: boolean;
  ram: boolean;
  cpu: boolean;
  energy: boolean;
  network: boolean;
  uptime: boolean;
  pid: boolean;
  state: boolean;
}

export type ColumnKey = keyof ColumnConfig;

export const COLUMN_KEYS: ColumnKey[] = ["name", "detail", "group", "ram", "cpu", "energy", "network", "uptime", "pid", "state"];

export interface AiProviderConfig {
  provider: string;
  model: string;
}

export type ThemeMode = "auto" | "light" | "dark" | "cyberpunk" | "custom";

const DEFAULT_FONT_SIZE = 12;
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 48;

const DEFAULT_COLUMNS: ColumnConfig = {
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
};

const DEFAULT_AI_CONFIG: AiProviderConfig = {
  provider: "openrouter",
  model: "meta-llama/llama-3.2-3b-instruct:free",
};

const MIN_IDLE_THRESHOLD = 0.1;
const MAX_IDLE_THRESHOLD = 10.0;
const DEFAULT_IDLE_THRESHOLD = 1.0;

const DEFAULT_THEME: ThemeMode = "auto";

const DEFAULT_LOCALE: LocaleCode = "auto";

const DEFAULT_TAB_PANEL_HEIGHT = 160;
const MIN_TAB_PANEL_HEIGHT = 40;
const MAX_TAB_PANEL_HEIGHT = 800;
const DEFAULT_NETWORK_PANEL_HEIGHT = 280;
const MIN_NETWORK_PANEL_HEIGHT = 140;
const MAX_NETWORK_PANEL_HEIGHT = 720;
const DEFAULT_AI_CHAT_HEIGHT = 220;
const MIN_AI_CHAT_HEIGHT = 140;
const MAX_AI_CHAT_HEIGHT = 640;

/** Current font size (in px) for the process table. */
export const fontSize = writable(DEFAULT_FONT_SIZE);

/** Visibility toggle for each column in the process table. */
export const columns = writable<ColumnConfig>({ ...DEFAULT_COLUMNS });

/** Display order of columns in the process table. */
export const columnOrder = writable<ColumnKey[]>([...COLUMN_KEYS]);

/** Selected AI provider and model for process analysis. */
export const aiProviderConfig = writable<AiProviderConfig>({ ...DEFAULT_AI_CONFIG });

/** CPU usage threshold (in %) below which a process is considered idle. */
export const idleThreshold = writable(DEFAULT_IDLE_THRESHOLD);

/** Current theme mode: "auto" follows system, or forced "light"/"dark". */
export const theme = writable<ThemeMode>(DEFAULT_THEME);

/** Height (in px) of the browser tabs panel at the bottom of the UI. */
export const tabPanelHeight = writable(DEFAULT_TAB_PANEL_HEIGHT);

/** Height (in px) of the network panel. */
export const networkPanelHeight = writable(DEFAULT_NETWORK_PANEL_HEIGHT);

/** Height (in px) of the AI chat panel. */
export const aiChatPanelHeight = writable(DEFAULT_AI_CHAT_HEIGHT);

/** User's preferred locale ("en", "es", or "auto" for system detection). */
export const localePreference = writable<LocaleCode>(DEFAULT_LOCALE);

/** User-defined custom theme palette. Applied when theme === "custom". */
export const customTheme = writable<CustomThemeOverrides | null>(null);

let storeInstance: any = null;

async function getStore() {
  if (storeInstance) return storeInstance;
  try {
    const { load } = await import("@tauri-apps/plugin-store");
    storeInstance = await load("preferences.json", { autoSave: false, defaults: {} });
    return storeInstance;
  } catch {
    return null;
  }
}

/** Loads all user preferences from the Tauri persistent store, falling back to defaults on error. */
export async function loadPreferences(): Promise<void> {
  const store = await getStore();
  if (!store) return;

  try {
    const savedFontSize = await store.get("fontSize");
    if (typeof savedFontSize === "number" && savedFontSize >= MIN_FONT_SIZE && savedFontSize <= MAX_FONT_SIZE) {
      fontSize.set(savedFontSize);
    }

    const savedColumns = await store.get("columns");
    if (savedColumns && typeof savedColumns === "object") {
      const merged = { ...DEFAULT_COLUMNS };
      for (const key of Object.keys(DEFAULT_COLUMNS) as (keyof ColumnConfig)[]) {
        if (typeof (savedColumns as Record<string, unknown>)[key] === "boolean") {
          merged[key] = (savedColumns as Record<string, unknown>)[key] as boolean;
        }
      }
      columns.set(merged);
    }

    const savedOrder = await store.get("columnOrder");
    if (Array.isArray(savedOrder)) {
      const valid = savedOrder.filter((k: unknown) => typeof k === "string" && COLUMN_KEYS.includes(k as ColumnKey)) as ColumnKey[];
      // Ensure all keys present (append any missing)
      const seen = new Set(valid);
      for (const k of COLUMN_KEYS) {
        if (!seen.has(k)) valid.push(k);
      }
      columnOrder.set(valid);
    }

    const savedAi = await store.get("aiProviderConfig");
    if (savedAi && typeof savedAi === "object") {
      const ai = savedAi as Record<string, unknown>;
      aiProviderConfig.set({
        provider: typeof ai.provider === "string" ? ai.provider : DEFAULT_AI_CONFIG.provider,
        model: typeof ai.model === "string" ? ai.model : DEFAULT_AI_CONFIG.model,
      });
    }

    const savedIdleThreshold = await store.get("idleThreshold");
    if (typeof savedIdleThreshold === "number" && savedIdleThreshold >= MIN_IDLE_THRESHOLD && savedIdleThreshold <= MAX_IDLE_THRESHOLD) {
      idleThreshold.set(savedIdleThreshold);
    }

    const savedTheme = await store.get("theme");
    if (typeof savedTheme === "string" && (savedTheme === "auto" || savedTheme === "light" || savedTheme === "dark" || savedTheme === "cyberpunk" || savedTheme === "custom")) {
      theme.set(savedTheme as ThemeMode);
    }

    const savedCustomTheme = await store.get("customTheme");
    if (savedCustomTheme && typeof savedCustomTheme === "object") {
      const ct = savedCustomTheme as CustomThemeOverrides;
      if (ct.name && ct.base && ct.overrides) {
        customTheme.set(ct);
        setCustomThemeOverrides(ct);
      }
    }

    const savedTabPanelHeight = await store.get("tabPanelHeight");
    if (typeof savedTabPanelHeight === "number" && savedTabPanelHeight >= MIN_TAB_PANEL_HEIGHT && savedTabPanelHeight <= MAX_TAB_PANEL_HEIGHT) {
      tabPanelHeight.set(savedTabPanelHeight);
    }

    const savedNetworkPanelHeight = await store.get("networkPanelHeight");
    if (typeof savedNetworkPanelHeight === "number" && savedNetworkPanelHeight >= MIN_NETWORK_PANEL_HEIGHT && savedNetworkPanelHeight <= MAX_NETWORK_PANEL_HEIGHT) {
      networkPanelHeight.set(savedNetworkPanelHeight);
    }

    const savedAiChatPanelHeight = await store.get("aiChatPanelHeight");
    if (typeof savedAiChatPanelHeight === "number" && savedAiChatPanelHeight >= MIN_AI_CHAT_HEIGHT && savedAiChatPanelHeight <= MAX_AI_CHAT_HEIGHT) {
      aiChatPanelHeight.set(savedAiChatPanelHeight);
    }

    const savedLocale = await store.get("locale");
    if (typeof savedLocale === "string" && (savedLocale === "en" || savedLocale === "es" || savedLocale === "auto")) {
      localePreference.set(savedLocale as LocaleCode);
    }
  } catch {
    // Use defaults on any read error
  }
}

/** Persists all current preference values to the Tauri persistent store. */
export async function savePreferences(): Promise<void> {
  const store = await getStore();
  if (!store) return;

  try {
    await store.set("fontSize", get(fontSize));
    await store.set("columns", get(columns));
    await store.set("columnOrder", get(columnOrder));
    await store.set("aiProviderConfig", get(aiProviderConfig));
    await store.set("idleThreshold", get(idleThreshold));
    await store.set("theme", get(theme));
    await store.set("tabPanelHeight", get(tabPanelHeight));
    await store.set("networkPanelHeight", get(networkPanelHeight));
    await store.set("aiChatPanelHeight", get(aiChatPanelHeight));
    await store.set("locale", get(localePreference));
    await store.set("customTheme", get(customTheme));
    await store.save();
  } catch {
    // Best-effort persistence
  }
}

let debounceTimer: ReturnType<typeof setTimeout> | undefined;

function debouncedSave() {
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    savePreferences();
  }, 500);
}

/** Subscribes to all preference stores and auto-saves on changes (debounced). Returns an unsubscribe function. */
export function initPreferenceSubscriptions(): () => void {
  const unsubs = [
    fontSize.subscribe(() => debouncedSave()),
    columns.subscribe(() => debouncedSave()),
    columnOrder.subscribe(() => debouncedSave()),
    aiProviderConfig.subscribe(() => debouncedSave()),
    idleThreshold.subscribe(() => debouncedSave()),
    theme.subscribe(() => debouncedSave()),
    tabPanelHeight.subscribe(() => debouncedSave()),
    networkPanelHeight.subscribe(() => debouncedSave()),
    aiChatPanelHeight.subscribe(() => debouncedSave()),
    localePreference.subscribe(() => debouncedSave()),
    customTheme.subscribe((ct) => {
      setCustomThemeOverrides(ct);
      debouncedSave();
    }),
  ];
  return () => {
    unsubs.forEach((u) => u());
    clearTimeout(debounceTimer);
  };
}

/** Increases the font size by 1px, clamped to MAX_FONT_SIZE. */
export function increaseFontSize(): void {
  fontSize.update((v) => Math.min(v + 1, MAX_FONT_SIZE));
}

/** Decreases the font size by 1px, clamped to MIN_FONT_SIZE. */
export function decreaseFontSize(): void {
  fontSize.update((v) => Math.max(v - 1, MIN_FONT_SIZE));
}

/** Moves the given column one position earlier (left) in the column order. */
export function moveColumnUp(key: ColumnKey): void {
  columnOrder.update((order) => {
    const idx = order.indexOf(key);
    if (idx <= 0) return order;
    const next = [...order];
    [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
    return next;
  });
}

/** Moves the given column one position later (right) in the column order. */
export function moveColumnDown(key: ColumnKey): void {
  columnOrder.update((order) => {
    const idx = order.indexOf(key);
    if (idx < 0 || idx >= order.length - 1) return order;
    const next = [...order];
    [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
    return next;
  });
}

export {
  MIN_FONT_SIZE,
  MAX_FONT_SIZE,
  DEFAULT_COLUMNS,
  DEFAULT_AI_CONFIG,
  MIN_IDLE_THRESHOLD,
  MAX_IDLE_THRESHOLD,
  DEFAULT_IDLE_THRESHOLD,
  DEFAULT_LOCALE,
  DEFAULT_NETWORK_PANEL_HEIGHT,
  MIN_NETWORK_PANEL_HEIGHT,
  MAX_NETWORK_PANEL_HEIGHT,
  DEFAULT_AI_CHAT_HEIGHT,
  MIN_AI_CHAT_HEIGHT,
  MAX_AI_CHAT_HEIGHT,
};
