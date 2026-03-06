import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import ChromeTabManager from "../ChromeTabManager.svelte";
import type { BrowserTab, ProcessEntry } from "../../lib/types";
import { _resetForTest, browserTabs, processes } from "../../stores/processes";

const mockInvoke = vi.mocked(invoke);

// Mock window.confirm for confirmation dialogs
window.confirm = vi.fn(() => true);

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
    expect(screen.getAllByText("512 MB").length).toBeGreaterThanOrEqual(1);
  });

  it("renders sections dynamically for any browser with tabs", () => {
    browserTabs.set([
      {
        id: "ff-1",
        title: "Firefox Tab",
        url: "https://mozilla.org",
        browser: "Firefox",
      },
    ]);

    render(ChromeTabManager);

    // Firefox section appears
    expect(screen.getByText("Firefox")).toBeInTheDocument();
    // No Chrome or Safari sections since they have no tabs
    expect(screen.queryByText("Chrome")).not.toBeInTheDocument();
    expect(screen.queryByText("Safari")).not.toBeInTheDocument();
  });

  it("detects Brave, Edge and Arc process RAM totals", () => {
    browserTabs.set([
      makeTab({ id: "b1", title: "Brave", browser: "Brave", url: "https://brave.com" }),
      makeTab({ id: "e1", title: "Edge", browser: "Edge", url: "https://microsoft.com" }),
      makeTab({ id: "a1", title: "Arc", browser: "Arc", url: "https://arc.net" }),
    ]);
    processes.set([
      makeProc({ name: "Brave Worker", exec_name: "Brave Browser Helper", group: "Browser", ram_mb: 333 }),
      makeProc({ name: "Edge Worker", exec_name: "Microsoft Edge Helper", group: "Browser", ram_mb: 444 }),
      makeProc({ name: "Arc Worker", exec_name: "Arc Helper", group: "Browser", ram_mb: 555 }),
    ]);

    render(ChromeTabManager);

    expect(screen.getAllByText("Brave").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Edge").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Arc").length).toBeGreaterThan(0);
    expect(screen.getByText("333 MB")).toBeInTheDocument();
    expect(screen.getByText("444 MB")).toBeInTheDocument();
    expect(screen.getByText("555 MB")).toBeInTheDocument();
  });

  it("closes a single tab and survives AppleScript permission errors", async () => {
    browserTabs.set([makeTab({ id: "chrome-2", url: "https://docs.example.com", browser: "Chrome" })]);
    mockInvoke.mockRejectedValueOnce(new Error("AppleScript permission denied"));

    render(ChromeTabManager);

    const closeButton = screen.getByTitle("Close this tab");
    await fireEvent.click(closeButton);

    expect(mockInvoke).toHaveBeenCalledWith("close_browser_tab", {
      tabId: "chrome-2",
      tabUrl: "https://docs.example.com",
      browser: "Chrome",
    });
    // Tab stays in the list because the close failed (error caught, not removed)
    await waitFor(() => {
      expect(screen.getByText("Example Tab")).toBeInTheDocument();
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

  it("successfully closes a tab and removes it from the list", async () => {
    browserTabs.set([
      makeTab({ id: "chrome-1", title: "Tab A", browser: "Chrome", url: "https://a.com" }),
      makeTab({ id: "chrome-2", title: "Tab B", browser: "Chrome", url: "https://b.com" }),
    ]);
    processes.set([makeProc()]);
    mockInvoke.mockResolvedValueOnce(true);

    render(ChromeTabManager);

    expect(screen.getByText("Tab A")).toBeInTheDocument();
    expect(screen.getByText("Tab B")).toBeInTheDocument();

    const closeButtons = screen.getAllByTitle("Close this tab");
    await fireEvent.click(closeButtons[0]);

    await waitFor(() => {
      expect(screen.queryByText("Tab A")).not.toBeInTheDocument();
      expect(screen.getByText("Tab B")).toBeInTheDocument();
    });
  });

  it("selects and deselects tabs via checkbox click", async () => {
    browserTabs.set([
      makeTab({ id: "chrome-1", title: "Tab A", browser: "Chrome" }),
      makeTab({ id: "chrome-2", title: "Tab B", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager);

    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(2);

    // Click first checkbox to select
    await fireEvent.click(checkboxes[0]);
    expect(checkboxes[0]).toBeChecked();

    // Click again to deselect
    await fireEvent.click(checkboxes[0]);
    expect(checkboxes[0]).not.toBeChecked();
  });

  it("select all and select none buttons work", async () => {
    browserTabs.set([
      makeTab({ id: "chrome-1", title: "Tab A", browser: "Chrome" }),
      makeTab({ id: "chrome-2", title: "Tab B", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager);

    // Click "All" to select all Chrome tabs
    await fireEvent.click(screen.getByTitle("Select all Chrome tabs"));

    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes[0]).toBeChecked();
    expect(checkboxes[1]).toBeChecked();

    // Click "None" to deselect all
    await fireEvent.click(screen.getByTitle("Deselect all"));

    expect(checkboxes[0]).not.toBeChecked();
    expect(checkboxes[1]).not.toBeChecked();
  });

  it("closes selected tabs via Close N button", async () => {
    browserTabs.set([
      makeTab({ id: "chrome-1", title: "Tab A", browser: "Chrome", url: "https://a.com" }),
      makeTab({ id: "chrome-2", title: "Tab B", browser: "Chrome", url: "https://b.com" }),
    ]);
    processes.set([makeProc()]);
    mockInvoke.mockResolvedValue(true);

    render(ChromeTabManager);

    // Select all
    await fireEvent.click(screen.getByTitle("Select all Chrome tabs"));

    // "Close 2" button should appear
    const closeSelectedBtn = screen.getByTitle("Close 2 selected tab(s)");
    await fireEvent.click(closeSelectedBtn);

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((call) => call[0] === "close_browser_tab");
      expect(calls).toHaveLength(2);
    });
  });

  it("toggles section collapse via header click", async () => {
    browserTabs.set([makeTab({ id: "chrome-1", title: "My Tab", browser: "Chrome" })]);
    processes.set([makeProc()]);

    render(ChromeTabManager);

    // Initially expanded - tab content visible
    expect(screen.getByText("My Tab")).toBeInTheDocument();

    // Click header to collapse
    const header = screen.getByRole("button", { name: /Chrome tabs/i });
    await fireEvent.click(header);

    // Tab content should be hidden
    expect(screen.queryByText("My Tab")).not.toBeInTheDocument();

    // Click header to expand again
    await fireEvent.click(header);
    expect(screen.getByText("My Tab")).toBeInTheDocument();
  });

  it("ignores unrelated keyboard key on tab row", async () => {
    browserTabs.set([makeTab({ id: "chrome-1", title: "My Tab", browser: "Chrome" })]);
    render(ChromeTabManager);

    const row = screen.getByRole("row");
    const checkbox = screen.getByRole("checkbox");
    expect(checkbox).not.toBeChecked();

    await fireEvent.keyDown(row, { key: "A" });
    expect(checkbox).not.toBeChecked();
  });

  it("supports Enter and Space keys on tab rows", async () => {
    browserTabs.set([makeTab({ id: "k1", title: "Key Tab", browser: "Chrome" })]);
    render(ChromeTabManager);

    const row = screen.getByRole("row");
    const checkbox = screen.getByRole("checkbox");

    await fireEvent.keyDown(row, { key: "Enter" });
    expect(checkbox).toBeChecked();

    await fireEvent.keyDown(row, { key: " " });
    expect(checkbox).not.toBeChecked();
  });

  it("renders nothing when there are no tabs", () => {
    browserTabs.set([]);
    const { container } = render(ChromeTabManager);
    expect(container.querySelector(".chrome-manager")).toBeNull();
  });

  it("shows high RAM in danger color", () => {
    browserTabs.set([makeTab({ id: "chrome-1", browser: "Chrome" })]);
    processes.set([makeProc({ ram_mb: 2048 })]);

    render(ChromeTabManager);

    // 2048 MB should be in danger color
    const ramText = screen.getByText("2048 MB");
    expect(ramText).toBeInTheDocument();
    expect(ramText.style.color).toBe("var(--danger)");
  });

  it("shows medium RAM in yellow color", () => {
    browserTabs.set([makeTab({ id: "chrome-1", browser: "Chrome" })]);
    processes.set([makeProc({ ram_mb: 500 })]);

    render(ChromeTabManager);

    const ramText = screen.getByText("500 MB");
    expect(ramText).toBeInTheDocument();
    expect(ramText.style.color).toBe("var(--yellow)");
  });

  it("handles keyboard Enter/Space on header for collapse toggle", async () => {
    browserTabs.set([makeTab({ id: "chrome-1", title: "My Tab", browser: "Chrome" })]);
    processes.set([makeProc()]);

    render(ChromeTabManager);

    const header = screen.getByRole("button", { name: /Chrome tabs/i });

    // Press Enter to collapse
    await fireEvent.keyDown(header, { key: "Enter" });
    expect(screen.queryByText("My Tab")).not.toBeInTheDocument();

    // Press Space to expand
    await fireEvent.keyDown(header, { key: " " });
    expect(screen.getByText("My Tab")).toBeInTheDocument();
  });

  it("clicking a tab row toggles its selection", async () => {
    browserTabs.set([makeTab({ id: "chrome-1", title: "Tab A", browser: "Chrome" })]);
    processes.set([makeProc()]);

    render(ChromeTabManager);

    const checkbox = screen.getByRole("checkbox");
    expect(checkbox).not.toBeChecked();

    // Click the row (not the checkbox)
    const row = checkbox.closest(".tab-row")!;
    await fireEvent.click(row);
    expect(checkbox).toBeChecked();

    // Click row again to deselect
    await fireEvent.click(row);
    expect(checkbox).not.toBeChecked();
  });

  it("filters tabs by title when filter prop is set", () => {
    browserTabs.set([
      makeTab({ id: "t1", title: "YouTube Music", url: "https://music.youtube.com", browser: "Chrome" }),
      makeTab({ id: "t2", title: "GitHub", url: "https://github.com", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager, { props: { filter: "youtube" } });

    expect(screen.getByText("YouTube Music")).toBeInTheDocument();
    expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
    // Shows filtered count
    expect(screen.getByText(/1\/2/)).toBeInTheDocument();
  });

  it("filters tabs by domain when filter prop is set", () => {
    browserTabs.set([
      makeTab({ id: "t1", title: "Home", url: "https://github.com/repo", browser: "Chrome" }),
      makeTab({ id: "t2", title: "Search", url: "https://google.com/search", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager, { props: { filter: "github" } });

    expect(screen.getByText("Home")).toBeInTheDocument();
    expect(screen.queryByText("Search")).not.toBeInTheDocument();
  });

  it("shows 'No matching tabs' when filter excludes all tabs", () => {
    browserTabs.set([
      makeTab({ id: "t1", title: "Tab A", url: "https://example.com", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager, { props: { filter: "nonexistent" } });

    expect(screen.getByText("No matching tabs")).toBeInTheDocument();
  });

  it("focuses tab and handles focus errors", async () => {
    browserTabs.set([makeTab({ id: "f1", title: "Focus Me", browser: "Chrome", url: "https://focus.com" })]);
    render(ChromeTabManager);

    mockInvoke.mockResolvedValueOnce(true);
    await fireEvent.click(screen.getByText("Focus Me"));
    expect(mockInvoke).toHaveBeenCalledWith("focus_browser_tab", {
      tabId: "f1",
      tabUrl: "https://focus.com",
      browser: "Chrome",
    });

    mockInvoke.mockRejectedValueOnce(new Error("focus denied"));
    await fireEvent.click(screen.getByText("Focus Me"));
    expect(screen.getByText("Focus Me")).toBeInTheDocument();
  });

  it("does not close tab when confirmation is canceled", async () => {
    browserTabs.set([makeTab({ id: "c1", title: "Cancelable", browser: "Chrome" })]);
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(ChromeTabManager);
    await fireEvent.click(screen.getByTitle("Close this tab"));

    expect(mockInvoke).not.toHaveBeenCalledWith("close_browser_tab", expect.anything());
    expect(screen.getByText("Cancelable")).toBeInTheDocument();
    confirmSpy.mockRestore();
  });

  it("returns early when Close All gets empty filtered tabs", async () => {
    browserTabs.set([makeTab({ id: "x1", title: "Alpha", browser: "Chrome", url: "https://alpha.com" })]);
    render(ChromeTabManager, { props: { filter: "zzz" } });

    await fireEvent.click(screen.getByTitle("Close all Chrome tabs"));
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("does not close selected when selected ids no longer exist", async () => {
    browserTabs.set([
      makeTab({ id: "s1", title: "One", browser: "Chrome", url: "https://one.com" }),
      makeTab({ id: "s2", title: "Two", browser: "Chrome", url: "https://two.com" }),
    ]);
    render(ChromeTabManager);

    await fireEvent.click(screen.getByTitle("Select all Chrome tabs"));
    browserTabs.set([makeTab({ id: "new", title: "New", browser: "Chrome", url: "https://new.com" })]);
    await fireEvent.click(screen.getByTitle("Close 2 selected tab(s)"));

    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("respects canceled confirmation for Close Selected and Close All", async () => {
    browserTabs.set([
      makeTab({ id: "k1", title: "One", browser: "Chrome", url: "https://one.com" }),
      makeTab({ id: "k2", title: "Two", browser: "Chrome", url: "https://two.com" }),
    ]);
    render(ChromeTabManager);
    await fireEvent.click(screen.getByTitle("Select all Chrome tabs"));

    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);
    await fireEvent.click(screen.getByTitle("Close 2 selected tab(s)"));
    await fireEvent.click(screen.getByTitle("Close all Chrome tabs"));

    expect(mockInvoke).not.toHaveBeenCalledWith("close_browser_tab", expect.anything());
    confirmSpy.mockRestore();
  });

  it("shows all tabs when filter is empty", () => {
    browserTabs.set([
      makeTab({ id: "t1", title: "Tab A", browser: "Chrome" }),
      makeTab({ id: "t2", title: "Tab B", browser: "Chrome" }),
    ]);
    processes.set([makeProc()]);

    render(ChromeTabManager, { props: { filter: "" } });

    expect(screen.getByText("Tab A")).toBeInTheDocument();
    expect(screen.getByText("Tab B")).toBeInTheDocument();
  });
});
