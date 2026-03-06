import { getProcessCategory, getProcessIconPath, iconForProcess, categoryLabel } from "../processIcons";

describe("processIcons", () => {
  describe("getProcessCategory", () => {
    it("returns browser for Chrome", () => {
      expect(getProcessCategory("Chrome")).toBe("browser");
    });

    it("returns browser from group", () => {
      expect(getProcessCategory("some-process", "Browser")).toBe("browser");
    });

    it("returns terminal for bash", () => {
      expect(getProcessCategory("bash")).toBe("terminal");
    });

    it("returns system for kernel_task", () => {
      expect(getProcessCategory("kernel_task")).toBe("system");
    });

    it("returns system from group", () => {
      expect(getProcessCategory("random", "System")).toBe("system");
    });

    it("returns database for postgres", () => {
      expect(getProcessCategory("postgres")).toBe("database");
    });

    it("returns server for nginx", () => {
      expect(getProcessCategory("nginx")).toBe("server");
    });

    it("returns media for spotify", () => {
      expect(getProcessCategory("Spotify")).toBe("media");
    });

    it("returns code for vscode", () => {
      expect(getProcessCategory("code")).toBe("code");
    });

    it("returns security for 1password", () => {
      expect(getProcessCategory("1password")).toBe("security");
    });

    it("returns files for Finder", () => {
      expect(getProcessCategory("Finder")).toBe("files");
    });

    it("returns mail for Outlook", () => {
      expect(getProcessCategory("Outlook")).toBe("mail");
    });

    it("returns default for unknown", () => {
      expect(getProcessCategory("my-custom-app")).toBe("default");
    });
  });

  describe("getProcessIconPath", () => {
    it("returns non-empty string for every category", () => {
      for (const cat of ["browser", "terminal", "system", "database", "server", "media", "code", "security", "files", "mail", "default"]) {
        expect(getProcessIconPath(cat)).toBeTruthy();
        expect(typeof getProcessIconPath(cat)).toBe("string");
      }
    });

    it("returns default path for unknown category", () => {
      expect(getProcessIconPath("nonexistent")).toBeTruthy();
    });
  });

  describe("iconForProcess", () => {
    it("returns SVG path data for chrome", () => {
      const path = iconForProcess("Chrome", "Browser");
      expect(path).toBeTruthy();
      expect(path.length).toBeGreaterThan(10);
    });

    it("returns SVG path data for unknown process", () => {
      const path = iconForProcess("random-app");
      expect(path).toBeTruthy();
    });
  });

  describe("categoryLabel", () => {
    it("returns human labels", () => {
      expect(categoryLabel("browser")).toBe("Browser");
      expect(categoryLabel("terminal")).toBe("Terminal");
      expect(categoryLabel("code")).toBe("Development");
      expect(categoryLabel("default")).toBe("Application");
    });
  });
});
