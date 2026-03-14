import { writable, derived, get } from "svelte/store";
import * as enModule from "../locales/en.json";
import * as esModule from "../locales/es.json";

export type LocaleCode = "en" | "es" | "auto";

function normalizeLocaleModule(mod: Record<string, unknown>): Record<string, unknown> {
  const fromDefault = mod.default;
  if (fromDefault && typeof fromDefault === "object") {
    return fromDefault as Record<string, unknown>;
  }
  return mod;
}

const en = normalizeLocaleModule(enModule as unknown as Record<string, unknown>);
const es = normalizeLocaleModule(esModule as unknown as Record<string, unknown>);

const translations: Record<string, Record<string, unknown>> = { en, es };

export const locale = writable<LocaleCode>("en");

export const resolvedLocale = derived(locale, ($locale) => {
  if ($locale !== "auto") return $locale;
  if (typeof navigator !== "undefined") {
    const lang = navigator.language.slice(0, 2);
    if (lang in translations) return lang;
  }
  return "en";
});

function resolve(obj: unknown, path: string): string | undefined {
  const parts = path.split(".");
  let current: unknown = obj;
  for (const part of parts) {
    if (current == null || typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === "string" ? current : undefined;
}

function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, key) =>
    key in params ? String(params[key]) : `{${key}}`,
  );
}

/** Returns the platform-appropriate modifier key label (Cmd on macOS, Ctrl elsewhere). */
export function modKey(): string {
  if (typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("mac")) {
    return "Cmd";
  }
  return "Ctrl";
}

export function t(key: string, params?: Record<string, string | number>): string {
  const lang = get(resolvedLocale);
  const value = resolve(translations[lang], key) ?? resolve(translations.en, key) ?? key;
  return interpolate(value, params);
}

export function initI18n(savedLocale?: LocaleCode): void {
  if (savedLocale && (savedLocale === "en" || savedLocale === "es" || savedLocale === "auto")) {
    locale.set(savedLocale);
  }
}
