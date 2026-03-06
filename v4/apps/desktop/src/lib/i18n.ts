import { writable, derived, get } from "svelte/store";
import en from "../locales/en.json";
import es from "../locales/es.json";

export type LocaleCode = "en" | "es" | "auto";

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
