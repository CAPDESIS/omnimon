import { writable, get } from "svelte/store";
import type { LocaleCode } from "../lib/i18n";
import type { CustomThemeOverrides } from "../lib/theme";
import type { NetworkAlertRule, ProfilePreset } from "../lib/types";
import { setCustomThemeOverrides } from "../lib/theme";
import { ipcSetNetworkAlertRules } from "../lib/ipc";

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
export type UserMode = "basic" | "pro";

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

const DEFAULT_PROFILE_PRESETS: ProfilePreset[] = [
  { id: "general", label: "General", idleThreshold: 1.0, pollIntervalMs: 2000, automationIntervalSecs: 5, aiProfile: "general" },
  { id: "developer", label: "Developer", idleThreshold: 0.6, pollIntervalMs: 1500, automationIntervalSecs: 3, aiProfile: "developer" },
  { id: "gaming", label: "Gaming", idleThreshold: 0.4, pollIntervalMs: 1000, automationIntervalSecs: 2, aiProfile: "gaming" },
  { id: "battery", label: "Battery Saver", idleThreshold: 2.0, pollIntervalMs: 4000, automationIntervalSecs: 10, aiProfile: "battery" },
];

const MIN_IDLE_THRESHOLD = 0.1;
const MAX_IDLE_THRESHOLD = 10.0;
const DEFAULT_IDLE_THRESHOLD = 1.0;

const DEFAULT_THEME: ThemeMode = "dark";
const DEFAULT_USER_MODE: UserMode = "pro";

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
const DEFAULT_POLL_INTERVAL_MS = 2000;
const MIN_POLL_INTERVAL_MS = 500;
const MAX_POLL_INTERVAL_MS = 10_000;
const DEFAULT_AUTOMATION_INTERVAL_SECS = 5;
const MIN_AUTOMATION_INTERVAL_SECS = 1;
const MAX_AUTOMATION_INTERVAL_SECS = 300;
const DEFAULT_AI_CACHE_TTL_MINUTES = 5;
const MIN_AI_CACHE_TTL_MINUTES = 0;
const MAX_AI_CACHE_TTL_MINUTES = 60;

const DEFAULT_NETWORK_ALERT_RULES: NetworkAlertRule[] = [
  {
    id: "default-high-bandwidth",
    name: "Alto bandwidth",
    enabled: true,
    condition: {
      kind: "high_bandwidth",
      threshold_mbps: 400,
      direction: "upload",
      process: null,
    },
    severity: "warning",
    cooldown_seconds: 30,
    notify_ai: false,
  },
  {
    id: "default-suspicious-port",
    name: "Conexion a puerto sospechoso",
    enabled: true,
    condition: {
      kind: "unusual_port",
      suspicious_ports: [4444, 6667, 8443, 31337],
    },
    severity: "critical",
    cooldown_seconds: 45,
    notify_ai: true,
  },
  {
    id: "default-process-spike",
    name: "Spike de proceso",
    enabled: true,
    condition: {
      kind: "process_network_spike",
      process_name: "chrome",
      multiplier: 5,
    },
    severity: "warning",
    cooldown_seconds: 60,
    notify_ai: true,
  },
  {
    id: "default-connection-count",
    name: "Demasiadas conexiones",
    enabled: true,
    condition: {
      kind: "connection_count_exceeded",
      max_connections: 200,
      process: null,
    },
    severity: "warning",
    cooldown_seconds: 30,
    notify_ai: false,
  },
];

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
export const pollIntervalMs = writable(DEFAULT_POLL_INTERVAL_MS);
export const automationIntervalSecs = writable(DEFAULT_AUTOMATION_INTERVAL_SECS);
export const aiCacheTtlMinutes = writable(DEFAULT_AI_CACHE_TTL_MINUTES);
export const activeProfilePreset = writable("general");
export const profilePresets = writable<ProfilePreset[]>([...DEFAULT_PROFILE_PRESETS]);

/** Current theme mode: "auto" follows system, or forced "light"/"dark". */
export const theme = writable<ThemeMode>(DEFAULT_THEME);

/** Collapse state for AI Profile panel. */
export const profilesCollapsedStore = writable(false);

/** Collapse state for Main Table panel. */
export const mainTableCollapsedStore = writable(false);

/** Collapse state for Network panel. */
export const networkMapCollapsedStore = writable(false);

/** Collapse state for Browser Tabs panel. */
export const browserTabsCollapsedStore = writable(false);

/** Collapse state for AI Chat panel. */
export const aiChatCollapsedStore = writable(false);

/** Collapse state for AI Command Bar panel. */
export const aiConfigCollapsedStore = writable(false);

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



/** User-facing workspace density mode. */
export const userMode = writable<UserMode>(DEFAULT_USER_MODE);
export const networkAlertRules = writable<NetworkAlertRule[]>([...DEFAULT_NETWORK_ALERT_RULES]);

export const displayName = writable("User");
export const profilePreset = writable<"minimal" | "balanced" | "power">("balanced");
export const dashboardLayout = writable<"compact" | "standard" | "expanded">("standard");
export const layoutModeStore = writable<"tabs" | "split">("tabs");

export const refreshInterval = writable(3000);

export const favoriteProcesses = writable<string[]>([]);
export const notificationLevel = writable<"off" | "critical" | "all">("all");

function sanitizePortList(raw: unknown): number[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((value): value is number => typeof value === "number" && Number.isInteger(value) && value >= 1 && value <= 65535)
    .slice(0, 32);
}

function sanitizeStringList(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((value): value is string => typeof value === "string")
    .map((value) => value.trim())
    .filter(Boolean)
    .slice(0, 32);
}

function sanitizeNetworkAlertRule(raw: unknown): NetworkAlertRule | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const rule = raw as Record<string, unknown>;
  const id = typeof rule.id === "string" ? rule.id.trim() : "";
  const name = typeof rule.name === "string" ? rule.name.trim() : "";
  const enabled = typeof rule.enabled === "boolean" ? rule.enabled : true;
  const severity = rule.severity;
  const cooldown = typeof rule.cooldown_seconds === "number" && Number.isFinite(rule.cooldown_seconds)
    ? Math.max(0, Math.min(Math.round(rule.cooldown_seconds), 3600))
    : 30;
  const notifyAi = typeof rule.notify_ai === "boolean" ? rule.notify_ai : false;
  const condition = rule.condition;

  if (!id || !name || !condition || typeof condition !== "object" || Array.isArray(condition)) return null;
  if (severity !== "info" && severity !== "warning" && severity !== "critical") return null;

  const c = condition as Record<string, unknown>;
  switch (c.kind) {
    case "high_bandwidth": {
      const threshold = typeof c.threshold_mbps === "number" && Number.isFinite(c.threshold_mbps) ? Math.max(0.1, c.threshold_mbps) : NaN;
      const direction = c.direction;
      if (!Number.isFinite(threshold)) return null;
      if (direction !== "upload" && direction !== "download" && direction !== "both") return null;
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "high_bandwidth",
          threshold_mbps: threshold,
          direction,
          process: typeof c.process === "string" && c.process.trim() ? c.process.trim() : null,
        },
      };
    }
    case "new_external_connection":
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "new_external_connection",
          exclude_known: typeof c.exclude_known === "boolean" ? c.exclude_known : true,
        },
      };
    case "unusual_port": {
      const suspiciousPorts = sanitizePortList(c.suspicious_ports);
      if (suspiciousPorts.length === 0) return null;
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "unusual_port",
          suspicious_ports: suspiciousPorts,
        },
      };
    }
    case "process_network_spike": {
      const processName = typeof c.process_name === "string" ? c.process_name.trim() : "";
      const multiplier = typeof c.multiplier === "number" && Number.isFinite(c.multiplier) ? Math.max(1.1, c.multiplier) : NaN;
      if (!processName || !Number.isFinite(multiplier)) return null;
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "process_network_spike",
          process_name: processName,
          multiplier,
        },
      };
    }
    case "connection_count_exceeded": {
      const maxConnections = typeof c.max_connections === "number" && Number.isFinite(c.max_connections)
        ? Math.max(1, Math.min(Math.round(c.max_connections), 100000))
        : NaN;
      if (!Number.isFinite(maxConnections)) return null;
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "connection_count_exceeded",
          max_connections: maxConnections,
          process: typeof c.process === "string" && c.process.trim() ? c.process.trim() : null,
        },
      };
    }
    case "suspicious_destination": {
      const patterns = sanitizeStringList(c.patterns);
      if (patterns.length === 0) return null;
      return {
        id,
        name,
        enabled,
        severity,
        cooldown_seconds: cooldown,
        notify_ai: notifyAi,
        condition: {
          kind: "suspicious_destination",
          patterns,
        },
      };
    }
    default:
      return null;
  }
}

function sanitizeNetworkAlertRules(raw: unknown): NetworkAlertRule[] {
  if (!Array.isArray(raw)) return [...DEFAULT_NETWORK_ALERT_RULES];
  const seen = new Set<string>();
  const rules: NetworkAlertRule[] = [];
  for (const entry of raw) {
    const rule = sanitizeNetworkAlertRule(entry);
    if (!rule || seen.has(rule.id)) continue;
    seen.add(rule.id);
    rules.push(rule);
  }
  return rules.length > 0 ? rules : [...DEFAULT_NETWORK_ALERT_RULES];
}

function sanitizeProfilePreset(raw: unknown): ProfilePreset | null {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
  const preset = raw as Record<string, unknown>;
  const id = typeof preset.id === "string" ? preset.id.trim().toLowerCase() : "";
  const label = typeof preset.label === "string" ? preset.label.trim() : "";
  const idle = typeof preset.idleThreshold === "number" ? preset.idleThreshold : NaN;
  const poll = typeof preset.pollIntervalMs === "number" ? preset.pollIntervalMs : NaN;
  const automation = typeof preset.automationIntervalSecs === "number" ? preset.automationIntervalSecs : NaN;
  const ai = typeof preset.aiProfile === "string" ? preset.aiProfile : "";

  if (!/^[a-z0-9_-]{1,32}$/.test(id) || !label) return null;
  if (!["general", "developer", "gaming", "battery"].includes(ai)) return null;
  if (!Number.isFinite(idle) || !Number.isFinite(poll) || !Number.isFinite(automation)) return null;

  return {
    id,
    label: label.slice(0, 48),
    idleThreshold: Math.min(Math.max(idle, MIN_IDLE_THRESHOLD), MAX_IDLE_THRESHOLD),
    pollIntervalMs: Math.min(Math.max(Math.round(poll), MIN_POLL_INTERVAL_MS), MAX_POLL_INTERVAL_MS),
    automationIntervalSecs: Math.min(Math.max(Math.round(automation), MIN_AUTOMATION_INTERVAL_SECS), MAX_AUTOMATION_INTERVAL_SECS),
    aiProfile: ai as ProfilePreset["aiProfile"],
  };
}

function sanitizeProfilePresets(raw: unknown): ProfilePreset[] {
  if (!Array.isArray(raw)) return [...DEFAULT_PROFILE_PRESETS];
  const seen = new Set<string>();
  const presets: ProfilePreset[] = [];
  for (const entry of raw) {
    const preset = sanitizeProfilePreset(entry);
    if (!preset || seen.has(preset.id)) continue;
    seen.add(preset.id);
    presets.push(preset);
  }
  return presets.length > 0 ? presets : [...DEFAULT_PROFILE_PRESETS];
}

export function applyProfilePresetById(id: string): boolean {
  const preset = get(profilePresets).find((entry) => entry.id === id);
  if (!preset) return false;
  activeProfilePreset.set(preset.id);
  idleThreshold.set(preset.idleThreshold);
  pollIntervalMs.set(preset.pollIntervalMs);
  automationIntervalSecs.set(preset.automationIntervalSecs);
  return true;
}

export function syncAiProfileToPreset(profile: ProfilePreset["aiProfile"]): void {
  const preset = get(profilePresets).find((entry) => entry.aiProfile === profile);
  if (preset) {
    applyProfilePresetById(preset.id);
  }
}

export function setProfilePresets(nextPresets: ProfilePreset[]): void {
  const sanitized = sanitizeProfilePresets(nextPresets);
  profilePresets.set(sanitized);
  if (!applyProfilePresetById(get(activeProfilePreset))) {
    applyProfilePresetById(sanitized[0]?.id ?? "general");
  }
}

let storeInstance: any = null;

async function getStore() {
  if (storeInstance) return storeInstance;
  try {
    const { load } = await import("@tauri-apps/plugin-store");
    storeInstance = await load("preferences.json", { autoSave: false, defaults: {} });
    console.debug("[PREFERENCES] Loaded Tauri store instance.");
    return storeInstance;
  } catch (err) {
    console.warn("[PREFERENCES] Failed to load Tauri store instance:", err);
    return null;
  }
}

/** Loads all user preferences from the Tauri persistent store, falling back to defaults on error. */
export async function loadPreferences(): Promise<void> {
  const store = await getStore();
  if (!store) {
    console.debug("[PREFERENCES] Store unavailable. Using default preferences.");
    return;
  }

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

    const savedNetworkAlertRules = await store.get("networkAlertRules");
    networkAlertRules.set(sanitizeNetworkAlertRules(savedNetworkAlertRules));

    const savedProfilePresets = await store.get("profilePresets");
    const sanitizedPresets = sanitizeProfilePresets(savedProfilePresets);
    setProfilePresets(sanitizedPresets);

    const savedActiveProfilePreset = await store.get("activeProfilePreset");
    const fallbackPresetId = typeof savedActiveProfilePreset === "string" ? savedActiveProfilePreset : sanitizedPresets[0]?.id ?? "general";
    const fallbackPreset = sanitizedPresets.find((preset) => preset.id === fallbackPresetId) ?? sanitizedPresets[0];
    if (fallbackPreset) {
      activeProfilePreset.set(fallbackPreset.id);
      if (typeof savedActiveProfilePreset === "string") {
        activeProfilePreset.set(savedActiveProfilePreset);
      }
    }

    const savedIdleThreshold = await store.get("idleThreshold");
    if (typeof savedIdleThreshold === "number" && savedIdleThreshold >= MIN_IDLE_THRESHOLD && savedIdleThreshold <= MAX_IDLE_THRESHOLD) {
      idleThreshold.set(savedIdleThreshold);
    } else if (fallbackPreset) {
      idleThreshold.set(fallbackPreset.idleThreshold);
    }

    const savedPollIntervalMs = await store.get("pollIntervalMs");
    if (typeof savedPollIntervalMs === "number" && savedPollIntervalMs >= MIN_POLL_INTERVAL_MS && savedPollIntervalMs <= MAX_POLL_INTERVAL_MS) {
      pollIntervalMs.set(Math.round(savedPollIntervalMs));
    } else if (fallbackPreset) {
      pollIntervalMs.set(fallbackPreset.pollIntervalMs);
    }

    const savedAutomationIntervalSecs = await store.get("automationIntervalSecs");
    if (typeof savedAutomationIntervalSecs === "number" && savedAutomationIntervalSecs >= MIN_AUTOMATION_INTERVAL_SECS && savedAutomationIntervalSecs <= MAX_AUTOMATION_INTERVAL_SECS) {
      automationIntervalSecs.set(Math.round(savedAutomationIntervalSecs));
    } else if (fallbackPreset) {
      automationIntervalSecs.set(fallbackPreset.automationIntervalSecs);
    }

    const savedAiCacheTtlMinutes = await store.get("aiCacheTtlMinutes");
    if (typeof savedAiCacheTtlMinutes === "number" && savedAiCacheTtlMinutes >= MIN_AI_CACHE_TTL_MINUTES && savedAiCacheTtlMinutes <= MAX_AI_CACHE_TTL_MINUTES) {
      aiCacheTtlMinutes.set(Math.round(savedAiCacheTtlMinutes));
    }

    const savedTheme = await store.get("theme");
    if (typeof savedTheme === "string" && (savedTheme === "auto" || savedTheme === "light" || savedTheme === "dark" || savedTheme === "cyberpunk" || savedTheme === "custom")) {
      theme.set(savedTheme as ThemeMode);
    }

    const savedUserMode = await store.get("userMode");
    if (savedUserMode === "basic" || savedUserMode === "pro") {
      userMode.set(savedUserMode);
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

    const savedLocale = await store.get("localePreference");
    if (typeof savedLocale === "string" && (savedLocale === "en" || savedLocale === "es" || savedLocale === "auto")) {
      localePreference.set(savedLocale as LocaleCode);
    }

    const savedProfilesCollapsed = await store.get("profilesCollapsed");
    if (typeof savedProfilesCollapsed === "boolean") profilesCollapsedStore.set(savedProfilesCollapsed);

    const savedMainTableCollapsed = await store.get("mainTableCollapsed");
    if (typeof savedMainTableCollapsed === "boolean") mainTableCollapsedStore.set(savedMainTableCollapsed);

    const savedNetworkMapCollapsed = await store.get("networkMapCollapsed");
    if (typeof savedNetworkMapCollapsed === "boolean") networkMapCollapsedStore.set(savedNetworkMapCollapsed);

    const savedBrowserTabsCollapsed = await store.get("browserTabsCollapsed");
    if (typeof savedBrowserTabsCollapsed === "boolean") browserTabsCollapsedStore.set(savedBrowserTabsCollapsed);

    const savedAiChatCollapsed = await store.get("aiChatCollapsed");
    if (typeof savedAiChatCollapsed === "boolean") aiChatCollapsedStore.set(savedAiChatCollapsed);

    const savedAiConfigCollapsed = await store.get("aiConfigCollapsed");
    if (typeof savedAiConfigCollapsed === "boolean") aiConfigCollapsedStore.set(savedAiConfigCollapsed);

    const savedDisplayName = await store.get("displayName");
    if (typeof savedDisplayName === "string") displayName.set(savedDisplayName);

    const savedProfilePreset = await store.get("profilePreset");
    if (savedProfilePreset === "minimal" || savedProfilePreset === "balanced" || savedProfilePreset === "power") profilePreset.set(savedProfilePreset);

    const savedDashboardLayout = await store.get("dashboardLayout");
    if (savedDashboardLayout === "compact" || savedDashboardLayout === "standard" || savedDashboardLayout === "expanded") dashboardLayout.set(savedDashboardLayout);

    const savedLayoutMode = await store.get("layoutMode");
    if (savedLayoutMode === "tabs" || savedLayoutMode === "split") layoutModeStore.set(savedLayoutMode);

    const savedRefreshInterval = await store.get("refreshInterval");
    if (typeof savedRefreshInterval === "number") refreshInterval.set(savedRefreshInterval);

    const savedFavoriteProcesses = await store.get("favoriteProcesses");
    if (Array.isArray(savedFavoriteProcesses)) favoriteProcesses.set(savedFavoriteProcesses.filter(s => typeof s === "string"));

    const savedNotificationLevel = await store.get("notificationLevel");
    if (savedNotificationLevel === "off" || savedNotificationLevel === "critical" || savedNotificationLevel === "all") notificationLevel.set(savedNotificationLevel);

  } catch (err) {
    console.warn("[PREFERENCES] Failed to read some preferences, falling back to defaults:", err);
  }
}

/** Persists all current preference values to the Tauri persistent store. */
export async function savePreferences(): Promise<void> {
  const store = await getStore();
  if (!store) {
    return;
  }

  try {
    await store.set("fontSize", get(fontSize));
    await store.set("columns", get(columns));
    await store.set("columnOrder", get(columnOrder));
    await store.set("aiProviderConfig", get(aiProviderConfig));
    await store.set("idleThreshold", get(idleThreshold));
    await store.set("pollIntervalMs", get(pollIntervalMs));
    await store.set("automationIntervalSecs", get(automationIntervalSecs));
    await store.set("aiCacheTtlMinutes", get(aiCacheTtlMinutes));
    await store.set("activeProfilePreset", get(activeProfilePreset));
    await store.set("profilePresets", get(profilePresets));
    await store.set("theme", get(theme));
    await store.set("userMode", get(userMode));
    await store.set("tabPanelHeight", get(tabPanelHeight));
    await store.set("networkPanelHeight", get(networkPanelHeight));
    await store.set("aiChatPanelHeight", get(aiChatPanelHeight));
    await store.set("localePreference", get(localePreference));

    await store.set("customTheme", get(customTheme));
    await store.set("networkAlertRules", get(networkAlertRules));
    
    await store.set("profilesCollapsed", get(profilesCollapsedStore));
    await store.set("mainTableCollapsed", get(mainTableCollapsedStore));
    await store.set("networkMapCollapsed", get(networkMapCollapsedStore));
    await store.set("browserTabsCollapsed", get(browserTabsCollapsedStore));
    await store.set("aiChatCollapsed", get(aiChatCollapsedStore));
    await store.set("aiConfigCollapsed", get(aiConfigCollapsedStore));

    // New profile stores
    await store.set("displayName", get(displayName));
    await store.set("profilePreset", get(profilePreset));
    await store.set("dashboardLayout", get(dashboardLayout));
    await store.set("layoutMode", get(layoutModeStore));
        // Actually, Svelte's `get` works with any store that has a `.subscribe` method. 
    await store.set("refreshInterval", get(refreshInterval));
    await store.set("favoriteProcesses", get(favoriteProcesses));
    await store.set("notificationLevel", get(notificationLevel));

    await store.save();
  } catch (err) {
    console.warn("[PREFERENCES] Failed to save preferences:", err);
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
  console.debug("[PREFERENCES] Initializing preference subscriptions for auto-save.");
  const unsubs = [
    fontSize.subscribe(() => debouncedSave()),
    columns.subscribe(() => debouncedSave()),
    columnOrder.subscribe(() => debouncedSave()),
    aiProviderConfig.subscribe(() => debouncedSave()),
    idleThreshold.subscribe(() => debouncedSave()),
    pollIntervalMs.subscribe(() => debouncedSave()),
    automationIntervalSecs.subscribe(() => debouncedSave()),
    aiCacheTtlMinutes.subscribe(() => debouncedSave()),
    activeProfilePreset.subscribe(() => debouncedSave()),
    profilePresets.subscribe(() => debouncedSave()),
    theme.subscribe(() => debouncedSave()),
    userMode.subscribe(() => debouncedSave()),
    tabPanelHeight.subscribe(() => debouncedSave()),
    networkPanelHeight.subscribe(() => debouncedSave()),
    aiChatPanelHeight.subscribe(() => debouncedSave()),
    localePreference.subscribe(() => debouncedSave()),
    displayName.subscribe(() => debouncedSave()),
    profilePreset.subscribe(() => debouncedSave()),
    dashboardLayout.subscribe(() => debouncedSave()),
    layoutModeStore.subscribe(() => debouncedSave()),
    refreshInterval.subscribe(() => debouncedSave()),
    favoriteProcesses.subscribe(() => debouncedSave()),
    notificationLevel.subscribe(() => debouncedSave()),

    customTheme.subscribe((ct) => {
      setCustomThemeOverrides(ct);
      debouncedSave();
    }),
    networkAlertRules.subscribe((rules) => {
      void ipcSetNetworkAlertRules(rules).catch((err) => {
        console.warn("[PREFERENCES] Failed to sync network alert rules:", err);
      });
      debouncedSave();
    }),
    profilesCollapsedStore.subscribe(() => debouncedSave()),
    mainTableCollapsedStore.subscribe(() => debouncedSave()),
    networkMapCollapsedStore.subscribe(() => debouncedSave()),
    browserTabsCollapsedStore.subscribe(() => debouncedSave()),
    aiChatCollapsedStore.subscribe(() => debouncedSave()),
    aiConfigCollapsedStore.subscribe(() => debouncedSave()),
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

/** Moves a column directly to a target index using drag-and-drop semantics. */
export function moveColumnToIndex(key: ColumnKey, targetIndex: number): void {
  columnOrder.update((order) => {
    const from = order.indexOf(key);
    if (from < 0) return order;
    const clampedTarget = Math.max(0, Math.min(targetIndex, order.length - 1));
    if (from === clampedTarget) return order;
    const next = [...order];
    const [item] = next.splice(from, 1);
    next.splice(clampedTarget, 0, item);
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
  DEFAULT_USER_MODE,
  DEFAULT_LOCALE,
  DEFAULT_NETWORK_PANEL_HEIGHT,
  MIN_NETWORK_PANEL_HEIGHT,
  MAX_NETWORK_PANEL_HEIGHT,
  DEFAULT_AI_CHAT_HEIGHT,
  MIN_AI_CHAT_HEIGHT,
  MAX_AI_CHAT_HEIGHT,
  DEFAULT_POLL_INTERVAL_MS,
  MIN_POLL_INTERVAL_MS,
  MAX_POLL_INTERVAL_MS,
  DEFAULT_AUTOMATION_INTERVAL_SECS,
  MIN_AUTOMATION_INTERVAL_SECS,
  MAX_AUTOMATION_INTERVAL_SECS,
  DEFAULT_AI_CACHE_TTL_MINUTES,
  MIN_AI_CACHE_TTL_MINUTES,
  MAX_AI_CACHE_TTL_MINUTES,
  DEFAULT_PROFILE_PRESETS,
  DEFAULT_NETWORK_ALERT_RULES,
};
