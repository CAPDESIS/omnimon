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
    expect(t("networkAlerts.types.new_external_connection")).toBe("Nueva conexión externa");
  });

  it("falls back to key when translation does not exist", () => {
    locale.set("es");
    expect(t("missing.path.key")).toBe("missing.path.key");
  });

  it("interpolates params and preserves unknown placeholders", () => {
    expect(t("process.browserTabs", { count: 3 })).toBe("Browser Tabs (3)");
    locale.set("es");
    expect(t("process.browserTabs", { count: 3 })).toBe("Pestañas del navegador (3)");
    expect(t("aiChat.errorGeneric")).toBe("Error al procesar la solicitud: {msg}");
  });

  it("initializes locale from saved preference", () => {
    initI18n("es");
    expect(get(locale)).toBe("es");
  });

  it("ignora locales invalidos al inicializar", () => {
    locale.set("en");
    initI18n("fr" as never);
    expect(get(locale)).toBe("en");
  });

  it("resolves auto locale from navigator y cae a en cuando no existe", () => {
    const langSpy = vi.spyOn(window.navigator, "language", "get").mockReturnValue("es-ES");
    initI18n("auto");
    expect(get(resolvedLocale)).toBe("es");
    langSpy.mockRestore();

    const unsupportedSpy = vi.spyOn(window.navigator, "language", "get").mockReturnValue("pt-BR");
    initI18n("auto");
    expect(get(resolvedLocale)).toBe("en");
    unsupportedSpy.mockRestore();
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
