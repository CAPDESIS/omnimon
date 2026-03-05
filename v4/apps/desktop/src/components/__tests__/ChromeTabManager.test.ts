import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import ChromeTabManager from "../ChromeTabManager.svelte";
import type { BrowserTab, ProcessEntry } from "../../lib/types";
import { _resetForTest, browserTabs, processes } from "../../stores/processes";

const mockInvoke = vi.mocked(invoke);

function makeTab(overrides: Partial<BrowserTab> = {}): BrowserTab {
  return {
    id: "tab-1",
    title: "Example Tab",
    url: "https://example.com/path",
    browser: "Chrome",
    ...overrides,
  };
}

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 100,
    name: "Chrome",
    exec_name: "/Applications/Google Chrome.app",
    ram_mb: 128,
    cpu_pct: 5,
    uptime: "1h",
    group: "Browser",
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

beforeEach(() => {
  _resetForTest();
  mockInvoke.mockReset();
});

describe("ChromeTabManager", () => {
  it("renders mixed Chrome/Safari tabs, long domains, and untitled tabs", () => {
    browserTabs.set([
      makeTab({
        id: "chrome-1",
        title: "Very long domain",
        url: "https://very.long.subdomain.example.co.uk/path?q=1",
        browser: "Chrome",
      }),
      makeTab({
        id: "safari-1",
        title: "",
        url: "",
        browser: "Safari",
      }),
    ]);
    processes.set([makeProc({ ram_mb: 512 })]);

    render(ChromeTabManager);

    expect(screen.getByText("Chrome")).toBeInTheDocument();
    expect(screen.getByText("Safari")).toBeInTheDocument();
    expect(screen.getByText("very.long.subdomain.example.co.uk")).toBeInTheDocument();
    expect(screen.getByText("(Untitled)")).toBeInTheDocument();
    expect(screen.getByText("512 MB")).toBeInTheDocument();
  });

  it("does not render sections for unsupported browsers", () => {
    browserTabs.set([
      {
        id: "ff-1",
        title: "Firefox",
        url: "https://mozilla.org",
        browser: "Firefox" as BrowserTab["browser"],
      },
    ]);

    render(ChromeTabManager);

    expect(screen.queryByText("Chrome")).not.toBeInTheDocument();
    expect(screen.queryByText("Safari")).not.toBeInTheDocument();
  });

  it("closes a single tab and survives AppleScript permission errors", async () => {
    browserTabs.set([makeTab({ id: "chrome-2", url: "https://docs.example.com", browser: "Chrome" })]);
    mockInvoke.mockRejectedValueOnce(new Error("AppleScript permission denied"));

    render(ChromeTabManager);

    const closeButton = screen.getByTitle("Close tab");
    await fireEvent.click(closeButton);

    expect(mockInvoke).toHaveBeenCalledWith("close_browser_tab", {
      tabId: "chrome-2",
      tabUrl: "https://docs.example.com",
      browser: "Chrome",
    });
    await waitFor(() => {
      expect(closeButton).not.toBeDisabled();
    });
  });

  it("closes all tabs in a section and ignores per-tab failures", async () => {
    browserTabs.set([
      makeTab({ id: "chrome-1", browser: "Chrome", url: "https://one.example.com" }),
      makeTab({ id: "chrome-2", browser: "Chrome", url: "https://two.example.com" }),
      makeTab({ id: "safari-1", browser: "Safari", url: "https://apple.com" }),
    ]);
    mockInvoke.mockResolvedValueOnce(true);
    mockInvoke.mockRejectedValueOnce(new Error("AppleScript permission denied"));

    render(ChromeTabManager);

    await fireEvent.click(screen.getByTitle("Close all Chrome tabs"));

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((call) => call[0] === "close_browser_tab");
      expect(calls).toHaveLength(2);
    });
  });
});
