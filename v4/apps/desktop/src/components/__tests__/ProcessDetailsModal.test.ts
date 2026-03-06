import { render, screen, fireEvent, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import ProcessDetailsModal from "../ProcessDetailsModal.svelte";
import type { ProcessEntry } from "../../lib/types";
import { _resetForTest, browserTabs } from "../../stores/processes";

const mockInvoke = vi.mocked(invoke);

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 42,
    name: "TestApp",
    exec_name: "/usr/bin/testapp",
    ram_mb: 128.5,
    cpu_pct: 12.3,
    uptime: "3h 15m",
    group: "Utilities",
    is_system: false,
    idle: true,
    state: "R",
    ...overrides,
  };
}

beforeEach(() => {
  _resetForTest();
  mockInvoke.mockReset();
});

describe("rendering", () => {
  it("renders all process fields", () => {
    const proc = makeProc();
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: proc, onclose } });

    expect(screen.getAllByText("TestApp").length).toBeGreaterThan(0);
    expect(screen.getByText("PID 42")).toBeInTheDocument();
    expect(screen.getByText("/usr/bin/testapp")).toBeInTheDocument();
    expect(screen.getByText("128.5 MB")).toBeInTheDocument();
    expect(screen.getByText("12.3%")).toBeInTheDocument();
    expect(screen.getByText("3h 15m")).toBeInTheDocument();
    expect(screen.getByText("Utilities")).toBeInTheDocument();
    expect(screen.getByText("R")).toBeInTheDocument();
    expect(screen.getByText("No")).toBeInTheDocument(); // is_system
  });

  it("renders dialog with correct role", () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("renders section labels", () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    expect(screen.getByText("Process")).toBeInTheDocument();
    expect(screen.getByText("Resources")).toBeInTheDocument();
  });

  it("shows Chrome tabs for Chrome Helper process", () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
      { id: "tab-2", title: "Google", url: "https://google.com", browser: "Chrome" },
      { id: "tab-3", title: "Apple", url: "https://apple.com", browser: "Safari" },
    ]);

    render(ProcessDetailsModal, {
      props: {
        process: makeProc({
          name: "Google Chrome Helper (Renderer)",
          exec_name: "Google Chrome Helper (Renderer)",
          group: "Browser",
        }),
        onclose: vi.fn(),
      },
    });

    expect(screen.getByText("Browser Tabs (2)")).toBeInTheDocument();
    expect(screen.getByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("github.com")).toBeInTheDocument();
    expect(screen.getByText("Google")).toBeInTheDocument();
    expect(screen.getByText("google.com")).toBeInTheDocument();
    // Safari tab should NOT appear
    expect(screen.queryByText("Apple")).not.toBeInTheDocument();
  });

  it("shows Safari tabs for WebContent process", () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
      { id: "tab-2", title: "Apple", url: "https://apple.com", browser: "Safari" },
    ]);

    render(ProcessDetailsModal, {
      props: {
        process: makeProc({
          name: "com.apple.WebKit.WebContent",
          exec_name: "com.apple.WebKit.WebContent",
          group: "Browser",
        }),
        onclose: vi.fn(),
      },
    });

    expect(screen.getByText("Browser Tabs (1)")).toBeInTheDocument();
    expect(screen.getByText("Apple")).toBeInTheDocument();
    expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
  });

  it("does not show browser tabs for non-Browser group", () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
    ]);

    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ group: "Utilities" }),
        onclose: vi.fn(),
      },
    });

    expect(screen.queryByText(/Browser Tabs/)).not.toBeInTheDocument();
  });

  it("does not show browser tabs when no tabs available", () => {
    render(ProcessDetailsModal, {
      props: {
        process: makeProc({
          name: "Google Chrome Helper (Renderer)",
          exec_name: "Google Chrome Helper (Renderer)",
          group: "Browser",
        }),
        onclose: vi.fn(),
      },
    });

    expect(screen.queryByText(/Browser Tabs/)).not.toBeInTheDocument();
  });

  it("handles Browser-group processes that are neither Chrome nor Safari", () => {
    browserTabs.set([{ id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" }]);

    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ name: "Odd Browser Process", exec_name: "Odd Helper", group: "Browser" }),
        onclose: vi.fn(),
      },
    });

    expect(screen.queryByText(/Browser Tabs/)).not.toBeInTheDocument();
  });

  it("color-codes high RAM values", () => {
    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ ram_mb: 1500 }),
        onclose: vi.fn(),
      },
    });
    const ramValue = screen.getByText("1500.0 MB");
    expect(ramValue.style.color).toBe("var(--danger)");
  });

  it("color-codes high CPU values", () => {
    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ cpu_pct: 75.0 }),
        onclose: vi.fn(),
      },
    });
    const cpuValue = screen.getByText("75.0%");
    expect(cpuValue.style.color).toBe("var(--danger)");
  });

  it("color-codes medium RAM and CPU values", () => {
    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ pid: 0, ram_mb: 500, cpu_pct: 20 }),
        onclose: vi.fn(),
      },
    });

    expect(screen.getByText("500.0 MB").style.color).toBe("var(--yellow)");
    expect(screen.getByText("20.0%").style.color).toBe("var(--yellow)");
    expect(screen.getByText("PID 0")).toBeInTheDocument();
  });

  it("uses fallback formatting for empty group/uptime and low resource colors", () => {
    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ group: "", uptime: "", is_system: true, cpu_pct: 1.5, ram_mb: 120 }),
        onclose: vi.fn(),
      },
    });

    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    expect(screen.getByText("Yes")).toBeInTheDocument();
    expect(screen.getByText("120.0 MB").style.color).toBe("var(--fg)");
    expect(screen.getByText("1.5%").style.color).toBe("var(--fg)");
  });

  it("handles malformed browser tab URLs", () => {
    browserTabs.set([{ id: "tab-1", title: "Broken", url: "not-a-url", browser: "Chrome" }]);

    render(ProcessDetailsModal, {
      props: {
        process: makeProc({ name: "Google Chrome Helper", exec_name: "Google Chrome Helper", group: "Browser" }),
        onclose: vi.fn(),
      },
    });

    expect(screen.getByText("Broken")).toBeInTheDocument();
    const domainEl = document.querySelector(".tab-domain") as HTMLElement;
    expect(domainEl.textContent).toBe("");
  });
});

describe("close behavior", () => {
  it("close button calls onclose", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const closeBtn = screen.getByLabelText("Close");
    await fireEvent.click(closeBtn);
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it("backdrop click calls onclose", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const backdrop = screen.getByRole("presentation");
    await fireEvent.click(backdrop);
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it("Escape key calls onclose", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const backdrop = screen.getByRole("presentation");
    await fireEvent.keyDown(backdrop, { key: "Escape" });
    expect(onclose).toHaveBeenCalledTimes(1);
  });
});

describe("focus trap", () => {
  it("wraps Tab from last to first focusable element", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const closeBtn = screen.getByLabelText("Close");

    closeBtn.focus();
    expect(document.activeElement).toBe(closeBtn);

    const backdrop = screen.getByRole("presentation");
    await fireEvent.keyDown(backdrop, { key: "Tab" });
    expect(document.activeElement).toBe(closeBtn);
  });

  it("wraps Shift+Tab from first to last focusable element", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const closeBtn = screen.getByLabelText("Close");
    const askAiBtn = screen.getByText("Ask AI");

    closeBtn.focus();
    const backdrop = screen.getByRole("presentation");
    await fireEvent.keyDown(backdrop, { key: "Tab", shiftKey: true });
    // Last focusable is now the "Ask AI" button
    expect(document.activeElement).toBe(askAiBtn);
  });

  it("returns early when no focusable elements exist", async () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    const modal = screen.getByRole("dialog") as HTMLDivElement;
    const backdrop = screen.getByRole("presentation");
    vi.spyOn(modal, "querySelectorAll").mockReturnValue([] as unknown as NodeListOf<HTMLElement>);

    await fireEvent.keyDown(backdrop, { key: "Tab" });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("does not wrap focus when tabbing from non-edge position", async () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    const modal = screen.getByRole("dialog") as HTMLDivElement;
    const backdrop = screen.getByRole("presentation");
    const first = document.createElement("button");
    const last = document.createElement("button");
    modal.append(first, last);
    first.focus();
    vi.spyOn(modal, "querySelectorAll").mockReturnValue([first, last] as unknown as NodeListOf<HTMLElement>);

    await fireEvent.keyDown(backdrop, { key: "Tab" });
    expect(document.activeElement).toBe(first);
  });
});

describe("tab management", () => {
  const browserProc = makeProc({
    name: "Google Chrome Helper (Renderer)",
    exec_name: "Google Chrome Helper (Renderer)",
    group: "Browser",
  });

  it("shows close buttons on browser tabs", () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
    ]);
    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    expect(screen.getByTitle("Close this tab")).toBeInTheDocument();
  });

  it("closes a tab via the X button", async () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
      { id: "tab-2", title: "Google", url: "https://google.com", browser: "Chrome" },
    ]);
    mockInvoke.mockResolvedValueOnce(true);

    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    const closeButtons = screen.getAllByTitle("Close this tab");
    await fireEvent.click(closeButtons[0]);

    await waitFor(() => {
      expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
      expect(screen.getByText("Google")).toBeInTheDocument();
    });
  });

  it("shows select all / none buttons for browser tabs", () => {
    browserTabs.set([
      { id: "tab-1", title: "Tab A", url: "https://a.com", browser: "Chrome" },
    ]);
    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    expect(screen.getByTitle("Select all tabs")).toBeInTheDocument();
    expect(screen.getByTitle("Deselect all")).toBeInTheDocument();
  });

  it("selects and closes multiple tabs", async () => {
    browserTabs.set([
      { id: "tab-1", title: "Tab A", url: "https://a.com", browser: "Chrome" },
      { id: "tab-2", title: "Tab B", url: "https://b.com", browser: "Chrome" },
    ]);
    mockInvoke.mockResolvedValue(true);

    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    // Select all
    await fireEvent.click(screen.getByTitle("Select all tabs"));

    // "Close 2" button should appear
    const closeSelectedBtn = screen.getByTitle("Close 2 selected tab(s)");
    await fireEvent.click(closeSelectedBtn);

    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((call) => call[0] === "close_browser_tab");
      expect(calls).toHaveLength(2);
    });
  });

  it("focuses a tab in the browser when clicking title", async () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
    ]);
    mockInvoke.mockResolvedValueOnce(true);

    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    const titleBtn = screen.getByText("GitHub");
    await fireEvent.click(titleBtn);

    expect(mockInvoke).toHaveBeenCalledWith("focus_browser_tab", {
      tabId: "tab-1",
      tabUrl: "https://github.com",
      browser: "Chrome",
    });
  });

  it("filters tabs by search text", async () => {
    browserTabs.set([
      { id: "tab-1", title: "YouTube Music", url: "https://music.youtube.com", browser: "Chrome" },
      { id: "tab-2", title: "GitHub", url: "https://github.com", browser: "Chrome" },
    ]);

    render(ProcessDetailsModal, { props: { process: browserProc, onclose: vi.fn() } });

    const filterInput = screen.getByPlaceholderText("Filter tabs...");
    await fireEvent.input(filterInput, { target: { value: "youtube" } });

    expect(screen.getByText("YouTube Music")).toBeInTheDocument();
    expect(screen.queryByText("GitHub")).not.toBeInTheDocument();
  });
});

describe("AI analysis", () => {
  it("shows Ask AI button", () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    expect(screen.getByText("Ask AI")).toBeInTheDocument();
  });

  it("shows AI hint text", () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    expect(screen.getByText(/Click "Ask AI" to get insights/)).toBeInTheDocument();
  });

  it("calls analyze_context on AI button click", async () => {
    mockInvoke.mockResolvedValueOnce("This process uses moderate memory.");

    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });

    await fireEvent.click(screen.getByText("Ask AI"));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("analyze_context", expect.objectContaining({
        context: expect.stringContaining("TestApp"),
      }));
    });
  });

  it("displays AI response text", async () => {
    mockInvoke.mockResolvedValueOnce("This process is safe to close.");

    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    await fireEvent.click(screen.getByText("Ask AI"));

    await waitFor(() => {
      expect(screen.getByText("This process is safe to close.")).toBeInTheDocument();
    });
  });

  it("displays AI error message", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("API key missing"));

    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    await fireEvent.click(screen.getByText("Ask AI"));

    await waitFor(() => {
      expect(screen.getByText("API key missing")).toBeInTheDocument();
    });
  });
});
