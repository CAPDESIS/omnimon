import { get } from "svelte/store";
import { initI18n, locale, resolvedLocale, t } from "../i18n";
import { localePreference } from "../../stores/preferences";

describe("i18n", () => {
  beforeEach(() => {
    locale.set("en");
    localePreference.set("auto");
  });

  it("renders English by default", () => {
    expect(t("settings.title")).toBe("OmniMon Settings");
  });

  it("applies Spanish when locale changes", () => {
    locale.set("es");
    expect(t("settings.title")).toBe("Ajustes de OmniMon");
    expect(t("toolbar.aiAnalyze")).toBe("Analizar IA");
  });

  it("falls back to key when translation does not exist", () => {
    expect(t("missing.path.key")).toBe("missing.path.key");
  });

  it("interpolates params in translation strings", () => {
    expect(t("process.browserTabs", { count: 3 })).toBe("Browser Tabs (3)");
    locale.set("es");
    expect(t("process.browserTabs", { count: 3 })).toBe("Pestañas del navegador (3)");
  });

  it("initializes locale from saved preference", () => {
    initI18n("es");
    expect(get(locale)).toBe("es");
  });

  it("changes locale store directly", () => {
    locale.set("es");
    expect(get(locale)).toBe("es");
    locale.set("en");
    expect(get(locale)).toBe("en");
  });

  it("resolves auto locale from navigator", () => {
    const langSpy = vi.spyOn(window.navigator, "language", "get").mockReturnValue("es-ES");

    initI18n("auto");
    expect(get(resolvedLocale)).toBe("es");
    langSpy.mockRestore();
  });

  it("keeps locale in app state via localePreference store", () => {
    const unsub = localePreference.subscribe((val) => {
      locale.set(val);
    });

    localePreference.set("es");
    expect(get(localePreference)).toBe("es");
    expect(t("settings.language")).toBe("Idioma");

    localePreference.set("en");
    expect(get(localePreference)).toBe("en");
    expect(t("settings.language")).toBe("Language");

    unsub();
  });
});
