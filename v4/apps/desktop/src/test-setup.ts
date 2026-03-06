import "@testing-library/jest-dom/vitest";
import { writable, derived, get } from "svelte/store";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("svelte-i18n", () => {
  const locale = writable("en");
  const dictionary = writable<Record<string, unknown>>({});

  const lookup = (obj: unknown, key: string): unknown =>
    key.split(".").reduce<unknown>((acc, part) => {
      if (acc && typeof acc === "object") return (acc as Record<string, unknown>)[part];
      return undefined;
    }, obj);

  const format = (value: string, params?: Record<string, string | number>): string => {
    if (!params) return value;
    return value.replace(/\{(\w+)\}/g, (_, k) => String(params[k] ?? `{${k}}`));
  };

  const translate = derived([dictionary, locale], ([$dictionary, $locale]) => {
    return (key: string, params?: Record<string, string | number>) => {
      const messages = ($dictionary as Record<string, Record<string, unknown>>)[$locale] ?? {};
      const fallback = ($dictionary as Record<string, Record<string, unknown>>).en ?? {};
      const raw = lookup(messages, key) ?? lookup(fallback, key) ?? key;
      return format(String(raw), params);
    };
  });

  return {
    locale,
    dictionary,
    _: translate,
    $_: translate,
    t: (key: string, params?: Record<string, string | number>) => get(translate)(key, params),
    $t: (key: string, params?: Record<string, string | number>) => get(translate)(key, params),
    init: vi.fn(),
    addMessages: vi.fn((lang: string, msgs: Record<string, unknown>) => {
      dictionary.update((d) => ({ ...d, [lang]: msgs }));
    }),
    register: vi.fn(),
    getLocaleFromNavigator: vi.fn(() => "en"),
    waitLocale: vi.fn(async () => undefined),
  };
});
