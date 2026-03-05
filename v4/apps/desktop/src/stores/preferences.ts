import { writable, get } from "svelte/store";

export interface ColumnConfig {
  name: boolean;
  detail: boolean;
  group: boolean;
  ram: boolean;
  cpu: boolean;
  uptime: boolean;
  pid: boolean;
  state: boolean;
}

export interface AiProviderConfig {
  provider: string;
  model: string;
}

const DEFAULT_FONT_SIZE = 12;
const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 18;

const DEFAULT_COLUMNS: ColumnConfig = {
  name: true,
  detail: true,
  group: true,
  ram: true,
  cpu: true,
  uptime: true,
  pid: true,
  state: true,
};

const DEFAULT_AI_CONFIG: AiProviderConfig = {
  provider: "openrouter",
  model: "google/gemini-flash-1.5-8b",
};

export const fontSize = writable(DEFAULT_FONT_SIZE);
export const columns = writable<ColumnConfig>({ ...DEFAULT_COLUMNS });
export const aiProviderConfig = writable<AiProviderConfig>({ ...DEFAULT_AI_CONFIG });

let storeInstance: any = null;

async function getStore() {
  if (storeInstance) return storeInstance;
  try {
    const { load } = await import("@tauri-apps/plugin-store");
    storeInstance = await load("preferences.json", { autoSave: false });
    return storeInstance;
  } catch {
    return null;
  }
}

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

    const savedAi = await store.get("aiProviderConfig");
    if (savedAi && typeof savedAi === "object") {
      const ai = savedAi as Record<string, unknown>;
      aiProviderConfig.set({
        provider: typeof ai.provider === "string" ? ai.provider : DEFAULT_AI_CONFIG.provider,
        model: typeof ai.model === "string" ? ai.model : DEFAULT_AI_CONFIG.model,
      });
    }
  } catch {
    // Use defaults on any read error
  }
}

export async function savePreferences(): Promise<void> {
  const store = await getStore();
  if (!store) return;

  try {
    await store.set("fontSize", get(fontSize));
    await store.set("columns", get(columns));
    await store.set("aiProviderConfig", get(aiProviderConfig));
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

export function initPreferenceSubscriptions(): () => void {
  const unsubs = [
    fontSize.subscribe(() => debouncedSave()),
    columns.subscribe(() => debouncedSave()),
    aiProviderConfig.subscribe(() => debouncedSave()),
  ];
  return () => {
    unsubs.forEach((u) => u());
    clearTimeout(debounceTimer);
  };
}

export function increaseFontSize(): void {
  fontSize.update((v) => Math.min(v + 1, MAX_FONT_SIZE));
}

export function decreaseFontSize(): void {
  fontSize.update((v) => Math.max(v - 1, MIN_FONT_SIZE));
}

export { MIN_FONT_SIZE, MAX_FONT_SIZE, DEFAULT_COLUMNS, DEFAULT_AI_CONFIG };
