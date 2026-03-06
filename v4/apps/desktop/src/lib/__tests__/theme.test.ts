import { describe, it, expect, beforeEach } from "vitest";
import {
  themes,
  applyThemeTokens,
  detectPlatform,
  resolveCustomTheme,
  setCustomThemeOverrides,
  getCustomThemeOverrides,
  getCurrentThemeTokens,
  type ThemeId,
  type CustomThemeOverrides,
} from "../theme";

describe("themes", () => {
  it("has dark, light, and cyberpunk themes", () => {
    expect(themes).toHaveProperty("dark");
    expect(themes).toHaveProperty("light");
    expect(themes).toHaveProperty("cyberpunk");
  });

  it("each theme has all required CSS variables", () => {
    const requiredVars = [
      "--bg", "--bg-alt", "--bg-hover", "--bg-selected", "--bg-surface",
      "--fg", "--fg-dim", "--border", "--border-subtle",
      "--accent", "--accent-hover", "--accent-dim",
      "--danger", "--danger-hover", "--green", "--yellow",
      "--chart-cpu", "--chart-ram", "--chart-net-rx", "--chart-net-tx",
      "--chart-grid", "--chart-bg",
      "--toast-bg", "--toast-border",
      "--shadow-sm", "--shadow-md", "--shadow-lg",
      "--radius-sm", "--radius-md", "--radius-lg",
    ];

    for (const [name, tokens] of Object.entries(themes)) {
      for (const v of requiredVars) {
        expect(tokens).toHaveProperty(v);
        expect((tokens as Record<string, string>)[v]).toBeTruthy();
      }
    }
  });

  it("dark theme has dark background", () => {
    expect(themes.dark["--bg"]).toBe("#0a0a0b");
  });

  it("light theme has light background", () => {
    expect(themes.light["--bg"]).toBe("#fafafa");
  });

  it("cyberpunk theme has purple accent", () => {
    expect(themes.cyberpunk["--accent"]).toBe("#c026d3");
  });
});

describe("applyThemeTokens", () => {
  beforeEach(() => {
    // Reset document state
    document.documentElement.removeAttribute("data-theme");
    for (const prop of Object.keys(themes.dark)) {
      document.documentElement.style.removeProperty(prop);
    }
  });

  it("applies dark theme tokens to root element", () => {
    applyThemeTokens("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(document.documentElement.style.getPropertyValue("--bg")).toBe("#0a0a0b");
  });

  it("applies light theme tokens", () => {
    applyThemeTokens("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(document.documentElement.style.getPropertyValue("--bg")).toBe("#fafafa");
  });

  it("applies cyberpunk theme tokens", () => {
    applyThemeTokens("cyberpunk");
    expect(document.documentElement.getAttribute("data-theme")).toBe("cyberpunk");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#c026d3");
  });

  it("auto mode removes data-theme attribute", () => {
    document.documentElement.setAttribute("data-theme", "dark");
    applyThemeTokens("auto");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("falls back to dark for unknown theme id", () => {
    applyThemeTokens("unknown" as ThemeId);
    expect(document.documentElement.style.getPropertyValue("--bg")).toBe("#0a0a0b");
  });
});

describe("resolveCustomTheme", () => {
  it("merges overrides onto base theme", () => {
    const custom: CustomThemeOverrides = {
      name: "My Theme",
      base: "dark",
      overrides: { "--accent": "#ff0000", "--bg": "#111111" },
    };
    const resolved = resolveCustomTheme(custom);
    expect(resolved["--accent"]).toBe("#ff0000");
    expect(resolved["--bg"]).toBe("#111111");
    expect(resolved["--fg"]).toBe(themes.dark["--fg"]);
  });

  it("uses light as base when specified", () => {
    const custom: CustomThemeOverrides = {
      name: "Light Custom",
      base: "light",
      overrides: { "--accent": "#00ff00" },
    };
    const resolved = resolveCustomTheme(custom);
    expect(resolved["--accent"]).toBe("#00ff00");
    expect(resolved["--bg"]).toBe(themes.light["--bg"]);
  });
});

describe("custom theme overrides", () => {
  afterEach(() => setCustomThemeOverrides(null));

  it("set and get custom overrides", () => {
    const custom: CustomThemeOverrides = {
      name: "Test",
      base: "cyberpunk",
      overrides: { "--danger": "#ff00ff" },
    };
    setCustomThemeOverrides(custom);
    expect(getCustomThemeOverrides()).toEqual(custom);
  });

  it("can clear custom overrides", () => {
    setCustomThemeOverrides({ name: "Temp", base: "dark", overrides: {} });
    setCustomThemeOverrides(null);
    expect(getCustomThemeOverrides()).toBeNull();
  });

  it("applies custom theme with overrides", () => {
    setCustomThemeOverrides({
      name: "Test",
      base: "dark",
      overrides: { "--accent": "#abcdef" },
    });
    applyThemeTokens("custom");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#abcdef");
    expect(document.documentElement.getAttribute("data-theme")).toBe("custom");
  });

  it("falls back to dark when custom has no overrides set", () => {
    setCustomThemeOverrides(null);
    applyThemeTokens("custom");
    expect(document.documentElement.style.getPropertyValue("--bg")).toBe(themes.dark["--bg"]);
  });
});

describe("getCurrentThemeTokens", () => {
  it("returns current theme values from DOM", () => {
    applyThemeTokens("dark");
    const tokens = getCurrentThemeTokens();
    expect(tokens["--bg"]).toBe(themes.dark["--bg"]);
    expect(tokens["--fg"]).toBe(themes.dark["--fg"]);
  });
});

describe("detectPlatform", () => {
  it("returns a valid platform string", () => {
    const result = detectPlatform();
    expect(["macos", "windows", "linux"]).toContain(result);
  });
});
