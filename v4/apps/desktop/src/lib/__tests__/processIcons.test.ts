import { describe, expect, it } from "vitest";

import {
  categoryLabel,
  getProcessCategory,
  getProcessIcon,
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

  it("cubre todos los mappings principales de categoria", () => {
    expect(getProcessCategory("bun")).toBe("server");
    expect(getProcessCategory("Spotify")).toBe("media");
    expect(getProcessCategory("Zed")).toBe("code");
    expect(getProcessCategory("WireGuard")).toBe("security");
    expect(getProcessCategory("Finder")).toBe("files");
    expect(getProcessCategory("Outlook")).toBe("mail");
    expect(getProcessCategory("launchd")).toBe("system");
  });

  it("retorna path SVG estable para categorias conocidas y fallback", () => {
    expect(getProcessIconPath("browser")).toContain("M8 1a7 7");
    expect(getProcessIconPath("default")).toContain("M8 2a6 6");
    expect(iconForProcess("Google Chrome", "Browser")).toBe(getProcessIconPath("browser"));
    expect(getProcessIconPath("mail")).not.toBe(getProcessIconPath("default"));
  });

  it("retorna iconos emoji por substring y fallback", () => {
    expect(getProcessIcon("Google Chrome Helper")).toBe("🌐");
    expect(getProcessIcon("Visual Studio Code")).toBe("💻");
    expect(getProcessIcon("docker-desktop")).toBe("🐳");
    expect(getProcessIcon("python3.12")).toBe("🐍");
    expect(getProcessIcon("totally-unknown-app")).toBe("⚙️");
  });

  it("categoryLabel cubre categorias restantes", () => {
    expect(categoryLabel("browser")).toBe("Browser");
    expect(categoryLabel("terminal")).toBe("Terminal");
    expect(categoryLabel("database")).toBe("Database");
    expect(categoryLabel("server")).toBe("Server");
    expect(categoryLabel("media")).toBe("Media");
    expect(categoryLabel("code")).toBe("Development");
    expect(categoryLabel("security")).toBe("Security");
    expect(categoryLabel("mail")).toBe("Mail");
  });

  it("detecta data URLs nativas validas", () => {
    expect(isNativeIconDataUrl("data:image/png;base64,AAAA")).toBe(true);
    expect(isNativeIconDataUrl("data:image/svg+xml;base64,BBBB")).toBe(true);
    expect(isNativeIconDataUrl("https://example.com/icon.png")).toBe(false);
    expect(isNativeIconDataUrl(null)).toBe(false);
    expect(isNativeIconDataUrl(undefined)).toBe(false);
  });
});
