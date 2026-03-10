/**
 * Theme engine: Dark / Light / Cyberpunk
 * Exposes CSS custom properties that all components consume.
 */

export type ThemeId = "auto" | "light" | "dark" | "cyberpunk" | "custom";

export interface ThemeTokens {
  "--bg-primary": string;
  "--bg-secondary": string;
  "--bg-card": string;
  "--bg": string;
  "--bg-alt": string;
  "--bg-hover": string;
  "--bg-selected": string;
  "--bg-surface": string;
  "--text-primary": string;
  "--text-secondary": string;
  "--fg": string;
  "--fg-dim": string;
  "--border": string;
  "--border-subtle": string;
  "--accent": string;
  "--accent-hover": string;
  "--accent-dim": string;
  "--danger": string;
  "--danger-hover": string;
  "--success": string;
  "--warning": string;
  "--fg-muted": string;
  "--green": string;
  "--yellow": string;
  "--chart-cpu": string;
  "--chart-ram": string;
  "--chart-net-rx": string;
  "--chart-net-tx": string;
  "--chart-energy": string;
  "--chart-disk": string;
  "--chart-grid": string;
  "--chart-bg": string;
  "--toast-bg": string;
  "--toast-border": string;
  "--shadow-sm": string;
  "--shadow-md": string;
  "--shadow-lg": string;
  "--radius-sm": string;
  "--radius-md": string;
  "--radius-lg": string;
}

const dark: ThemeTokens = {
  "--bg-primary": "#0a0a0b",
  "--bg-secondary": "#121214",
  "--bg-card": "#161618",
  "--bg": "#0a0a0b",
  "--bg-alt": "#111113",
  "--bg-hover": "#1a1a1e",
  "--bg-selected": "#0d2847",
  "--bg-surface": "#161618",
  "--text-primary": "#ededef",
  "--text-secondary": "#a1a1aa",
  "--fg": "#ededef",
  "--fg-dim": "#a1a1aa",
  "--border": "#27272a",
  "--border-subtle": "#2a2a3a",
  "--accent": "#3b82f6",
  "--accent-hover": "#2563eb",
  "--accent-dim": "#0d2847",
  "--danger": "#ef4444",
  "--danger-hover": "#dc2626",
  "--success": "#22c55e",
  "--warning": "#eab308",
  "--fg-muted": "#71717a",
  "--green": "#22c55e",
  "--yellow": "#eab308",
  "--chart-cpu": "#3b82f6",
  "--chart-ram": "#a855f7",
  "--chart-net-rx": "#22c55e",
  "--chart-net-tx": "#f97316",
  "--chart-energy": "#eab308",
  "--chart-disk": "#06b6d4",
  "--chart-grid": "#141418",
  "--chart-bg": "#0a0a0b",
  "--toast-bg": "#18181b",
  "--toast-border": "#27272a",
  "--shadow-sm": "0 1px 2px rgba(0,0,0,0.4)",
  "--shadow-md": "0 4px 12px rgba(0,0,0,0.5)",
  "--shadow-lg": "0 8px 32px rgba(0,0,0,0.6)",
  "--radius-sm": "4px",
  "--radius-md": "8px",
  "--radius-lg": "12px",
};

const light: ThemeTokens = {
  "--bg-primary": "#fafafa",
  "--bg-secondary": "#f4f4f5",
  "--bg-card": "#ffffff",
  "--bg": "#fafafa",
  "--bg-alt": "#f4f4f5",
  "--bg-hover": "#e4e4e7",
  "--bg-selected": "#dbeafe",
  "--bg-surface": "#ffffff",
  "--text-primary": "#09090b",
  "--text-secondary": "#71717a",
  "--fg": "#09090b",
  "--fg-dim": "#71717a",
  "--border": "#e4e4e7",
  "--border-subtle": "#d4d4d8",
  "--accent": "#2563eb",
  "--accent-hover": "#1d4ed8",
  "--accent-dim": "#dbeafe",
  "--danger": "#dc2626",
  "--danger-hover": "#b91c1c",
  "--success": "#16a34a",
  "--warning": "#ca8a04",
  "--fg-muted": "#71717a",
  "--green": "#16a34a",
  "--yellow": "#ca8a04",
  "--chart-cpu": "#2563eb",
  "--chart-ram": "#9333ea",
  "--chart-net-rx": "#16a34a",
  "--chart-net-tx": "#ea580c",
  "--chart-energy": "#ca8a04",
  "--chart-disk": "#0891b2",
  "--chart-grid": "#d4d4d8",
  "--chart-bg": "#fafafa",
  "--toast-bg": "#ffffff",
  "--toast-border": "#e4e4e7",
  "--shadow-sm": "0 1px 2px rgba(0,0,0,0.05)",
  "--shadow-md": "0 4px 12px rgba(0,0,0,0.1)",
  "--shadow-lg": "0 8px 32px rgba(0,0,0,0.15)",
  "--radius-sm": "4px",
  "--radius-md": "8px",
  "--radius-lg": "12px",
};

const cyberpunk: ThemeTokens = {
  "--bg-primary": "#0b0014",
  "--bg-secondary": "#120020",
  "--bg-card": "#0f001a",
  "--bg": "#0b0014",
  "--bg-alt": "#120020",
  "--bg-hover": "#1a0033",
  "--bg-selected": "#1a0040",
  "--bg-surface": "#0f001a",
  "--text-primary": "#e0d4ff",
  "--text-secondary": "#c4b5fd",
  "--fg": "#e0d4ff",
  "--fg-dim": "#c4b5fd",
  "--border": "#2d1b4e",
  "--border-subtle": "#1a0e30",
  "--accent": "#c026d3",
  "--accent-hover": "#a21caf",
  "--accent-dim": "#2a0842",
  "--danger": "#f43f5e",
  "--danger-hover": "#e11d48",
  "--success": "#4ade80",
  "--warning": "#facc15",
  "--fg-muted": "#c4b5fd",
  "--green": "#4ade80",
  "--yellow": "#facc15",
  "--chart-cpu": "#c026d3",
  "--chart-ram": "#7c3aed",
  "--chart-net-rx": "#4ade80",
  "--chart-net-tx": "#fb923c",
  "--chart-energy": "#facc15",
  "--chart-disk": "#22d3ee",
  "--chart-grid": "#150028",
  "--chart-bg": "#0b0014",
  "--toast-bg": "#120020",
  "--toast-border": "#2d1b4e",
  "--shadow-sm": "0 1px 4px rgba(192,38,211,0.2)",
  "--shadow-md": "0 4px 16px rgba(192,38,211,0.25)",
  "--shadow-lg": "0 8px 40px rgba(192,38,211,0.3)",
  "--radius-sm": "4px",
  "--radius-md": "8px",
  "--radius-lg": "12px",
};

export const themes: Record<string, ThemeTokens> = { dark, light, cyberpunk };

/** User-defined custom theme overrides. Stored in preferences. */
export interface CustomThemeOverrides {
  name: string;
  base: "dark" | "light" | "cyberpunk";
  overrides: Partial<ThemeTokens>;
}

/** Merge a base theme with user overrides. */
export function resolveCustomTheme(custom: CustomThemeOverrides): ThemeTokens {
  const base = themes[custom.base] ?? dark;
  return { ...base, ...custom.overrides };
}

/** Active custom theme overrides (set from preferences). */
let activeCustomOverrides: CustomThemeOverrides | null = null;

export function setCustomThemeOverrides(overrides: CustomThemeOverrides | null): void {
  activeCustomOverrides = overrides;
}

export function getCustomThemeOverrides(): CustomThemeOverrides | null {
  return activeCustomOverrides;
}

/**
 * Applies theme tokens as CSS custom properties on the root element.
 * "auto" resolves to dark/light via system preference.
 * "custom" uses user-defined overrides on top of a base theme.
 */
export function applyThemeTokens(themeId: ThemeId): void {
  if (typeof document === "undefined") return;
  const html = document.documentElement;

  let resolved: ThemeTokens;
  if (themeId === "auto") {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    resolved = prefersDark ? dark : light;
    html.removeAttribute("data-theme");
  } else if (themeId === "custom" && activeCustomOverrides) {
    resolved = resolveCustomTheme(activeCustomOverrides);
    html.setAttribute("data-theme", "custom");
  } else {
    resolved = themes[themeId] ?? dark;
    html.setAttribute("data-theme", themeId);
  }

  for (const [prop, value] of Object.entries(resolved)) {
    html.style.setProperty(prop, value);
  }
}

/** Read current resolved theme tokens from the DOM. Useful for lightweight-charts sync. */
export function getCurrentThemeTokens(): Record<string, string> {
  if (typeof document === "undefined") return {};
  const style = getComputedStyle(document.documentElement);
  const result: Record<string, string> = {};
  for (const key of Object.keys(dark)) {
    result[key] = style.getPropertyValue(key).trim();
  }
  return result;
}

/** Returns the OS platform hint for platform-specific styling. */
export function detectPlatform(): "macos" | "windows" | "linux" {
  if (typeof navigator === "undefined") return "macos";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) return "macos";
  if (ua.includes("win")) return "windows";
  return "linux";
}
