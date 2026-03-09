import { describe, expect, it } from "vitest";

import {
  categoryLabel,
  getProcessCategory,
  getProcessIconPath,
  iconForProcess,
  isNativeIconDataUrl,
} from "../processIcons";

describe("processIcons", () => {
  it("retorna etiquetas humanas para categorias", () => {
    expect(categoryLabel("system")).toBe("System");
    expect(categoryLabel("files")).toBe("Files");
    expect(categoryLabel("default")).toBe("Application");
  });

  it("detecta categorias por group y nombre", () => {
    expect(getProcessCategory("whatever", "Browser")).toBe("browser");
    expect(getProcessCategory("launchd", "OS")).toBe("system");
    expect(getProcessCategory("Firefox")).toBe("browser");
    expect(getProcessCategory("iTerm2")).toBe("terminal");
    expect(getProcessCategory("postgres")).toBe("database");
    expect(getProcessCategory("Unknown Custom App")).toBe("default");
  });

  it("retorna path SVG estable para categorias conocidas y fallback", () => {
    expect(getProcessIconPath("browser")).toContain("M8 1a7 7");
    expect(getProcessIconPath("default")).toContain("M8 2a6 6");
    expect(iconForProcess("Google Chrome", "Browser")).toBe(getProcessIconPath("browser"));
  });

  it("detecta data URLs nativas validas", () => {
    expect(isNativeIconDataUrl("data:image/png;base64,AAAA")).toBe(true);
    expect(isNativeIconDataUrl("data:image/svg+xml;base64,BBBB")).toBe(true);
    expect(isNativeIconDataUrl("https://example.com/icon.png")).toBe(false);
    expect(isNativeIconDataUrl(null)).toBe(false);
    expect(isNativeIconDataUrl(undefined)).toBe(false);
  });
});
