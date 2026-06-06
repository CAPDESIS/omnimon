import "@testing-library/jest-dom/vitest";
import { configure } from "@testing-library/svelte";
import { writable, derived, get } from "svelte/store";

// @testing-library's waitFor/findBy default to a 1000ms timeout. On the loaded
// CAPDESIS self-hosted CI runner, render-heavy component interactions (e.g.
// App.test.ts opening modals) settle slower than locally and can exceed 1000ms,
// failing intermittently. Give async utilities more slack so they stay
// deterministic under runner load (passes locally with this set).
configure({ asyncUtilTimeout: 8000 });

// Polyfill Element.prototype.animate for Svelte transitions in happy-dom
if (typeof Element.prototype.animate !== "function") {
  Element.prototype.animate = function () {
    return {
      finished: Promise.resolve(),
      cancel: () => {},
      finish: () => {},
      play: () => {},
      pause: () => {},
      onfinish: null,
      oncancel: null,
      addEventListener: () => {},
      removeEventListener: () => {},
    } as unknown as Animation;
  };
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// lightweight-charts uses Canvas APIs and color parsing not available in JSDOM
vi.mock("lightweight-charts", () => {
  const noop = () => {};
  const mockSeries = { setData: noop, applyOptions: noop, update: noop };
  return {
    createChart: () => ({
      addSeries: () => mockSeries,
      timeScale: () => ({ fitContent: noop }),
      priceScale: () => ({ applyOptions: noop }),
      applyOptions: noop,
      remove: noop,
    }),
    AreaSeries: "Area",
    ColorType: { Solid: 0 },
    CrosshairMode: { Hidden: 0, Normal: 1 },
    LineStyle: { Dashed: 1 },
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
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
