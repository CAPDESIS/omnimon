import { render, screen, fireEvent } from "@testing-library/svelte";
import ProcessTable from "../ProcessTable.svelte";
import type { ProcessEntry } from "../../lib/types";
import { selectedPids, focusedPid, browserTabs, _resetForTest } from "../../stores/processes";
import { get } from "svelte/store";

function makeProc(overrides: Partial<ProcessEntry> = {}): ProcessEntry {
  return {
    pid: 1,
    name: "TestApp",
    exec_name: "/usr/bin/testapp",
    ram_mb: 50.3,
    cpu_pct: 5.1,
    uptime: "2m",
    group: "Utilities",
    is_system: false,
    idle: false,
    state: "R",
    ...overrides,
  };
}

beforeEach(() => {
  _resetForTest();
});

describe("rendering", () => {
  it("renders correct number of data rows", () => {
    const procs = [makeProc({ pid: 1 }), makeProc({ pid: 2 }), makeProc({ pid: 3 })];
    render(ProcessTable, { props: { processes: procs } });
    // Each row has a checkbox with aria-label "Select TestApp"
    const rows = screen.getAllByRole("row");
    // 1 header + 3 data rows
    expect(rows.length).toBe(4);
  });

  it("renders column headers", () => {
    render(ProcessTable, { props: { processes: [] } });
    expect(screen.getByText("Name")).toBeInTheDocument();
    expect(screen.getByText("Detail")).toBeInTheDocument();
    expect(screen.getByText("Group")).toBeInTheDocument();
    expect(screen.getByText("PID")).toBeInTheDocument();
    expect(screen.getByText("RAM")).toBeInTheDocument();
    expect(screen.getByText("CPU")).toBeInTheDocument();
    expect(screen.getByText("Time")).toBeInTheDocument();
    expect(screen.getByText("ST")).toBeInTheDocument();
  });

  it("formats RAM and CPU values", () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1, ram_mb: 123.456, cpu_pct: 7.89 })] } });
    expect(screen.getByText("123.5")).toBeInTheDocument();
    expect(screen.getByText("7.9")).toBeInTheDocument();
  });

  it("shows idle badge when process is idle", () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1, idle: true })] } });
    expect(screen.getByText("idle")).toBeInTheDocument();
  });

  it("does not show idle badge when process is not idle", () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1, idle: false })] } });
    expect(screen.queryByText("idle")).not.toBeInTheDocument();
  });

  it("renders strange process names safely", () => {
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 11, name: "weird 🚀 proc [beta] (test)" })],
      },
    });
    expect(screen.getByText("weird 🚀 proc [beta] (test)")).toBeInTheDocument();
  });

  it("shows tab count for Chrome Helper process", () => {
    browserTabs.set([
      { id: "tab-1", title: "GitHub", url: "https://github.com", browser: "Chrome" },
      { id: "tab-2", title: "Google", url: "https://google.com", browser: "Chrome" },
    ]);
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 20, name: "Google Chrome Helper (Renderer)", exec_name: "Google Chrome Helper (Renderer)", group: "Browser" })],
      },
    });
    expect(screen.getByText("2 Chrome tabs open")).toBeInTheDocument();
    expect(screen.getByText("Chrome")).toBeInTheDocument();
  });

  it("shows tab count for Safari WebContent process", () => {
    browserTabs.set([
      { id: "tab-1", title: "Apple", url: "https://apple.com", browser: "Safari" },
    ]);
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 21, name: "com.apple.WebKit.WebContent", exec_name: "com.apple.WebKit.WebContent", group: "Browser" })],
      },
    });
    expect(screen.getByText("1 Safari tab open")).toBeInTheDocument();
    expect(screen.getByText("Safari")).toBeInTheDocument();
  });

  it("detects generic Chrome Helper name", () => {
    browserTabs.set([{ id: "tab-1", title: "Docs", url: "https://docs.example.com", browser: "Chrome" }]);
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 212, name: "Chrome Helper", exec_name: "Chrome Helper", group: "Browser" })],
      },
    });
    expect(screen.getByText("1 Chrome tab open")).toBeInTheDocument();
    expect(screen.getByText("Chrome")).toBeInTheDocument();
  });

  it("detects generic WebContent name", () => {
    browserTabs.set([{ id: "tab-2", title: "Apple", url: "https://apple.com", browser: "Safari" }]);
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 213, name: "WebContent", exec_name: "Safari WebContent", group: "Browser" })],
      },
    });
    expect(screen.getByText("1 Safari tab open")).toBeInTheDocument();
    expect(screen.getByText("Safari")).toBeInTheDocument();
  });

  it("detects Brave, Edge, and Arc helpers", () => {
    browserTabs.set([
      { id: "b1", title: "Brave", url: "https://brave.com", browser: "Brave" },
      { id: "e1", title: "Edge", url: "https://microsoft.com", browser: "Edge" },
      { id: "a1", title: "Arc", url: "https://arc.net", browser: "Arc" },
    ]);

    const { rerender } = render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 31, name: "Brave", exec_name: "Brave Browser Helper", group: "Browser" })],
      },
    });
    expect(screen.getByText("1 Brave tab open")).toBeInTheDocument();
    expect(screen.getAllByText("Brave").length).toBeGreaterThan(0);

    rerender({ processes: [makeProc({ pid: 32, name: "Edge", exec_name: "Microsoft Edge Helper", group: "Browser" })] });
    expect(screen.getByText("1 Edge tab open")).toBeInTheDocument();
    expect(screen.getAllByText("Edge").length).toBeGreaterThan(0);

    rerender({ processes: [makeProc({ pid: 33, name: "Arc", exec_name: "Arc Helper", group: "Browser" })] });
    expect(screen.getByText("1 Arc tab open")).toBeInTheDocument();
    expect(screen.getAllByText("Arc").length).toBeGreaterThan(0);
  });

  it("uses exec_name for non-tab details", () => {
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 22, name: "Local App", exec_name: "/opt/local/bin/local-app" })],
      },
    });
    expect(screen.getByText("/opt/local/bin/local-app")).toBeInTheDocument();
  });

  it("falls back to em dash for empty uptime and empty group label", () => {
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 24, uptime: "", group: "" })],
      },
    });

    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("shows Browser group when browser not detected", () => {
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 23, name: "Renderer", group: "Browser", exec_name: "Renderer" })],
      },
    });
    expect(screen.getByText("Browser")).toBeInTheDocument();
  });

  it("falls back when browser is detected but no tab counts exist", () => {
    render(ProcessTable, {
      props: {
        processes: [
          makeProc({
            pid: 25,
            name: "Google Chrome Helper (Renderer)",
            exec_name: "Google Chrome Helper (Renderer)",
            group: "Browser",
          }),
        ],
      },
    });
    expect(screen.queryByText(/Chrome tab/)).not.toBeInTheDocument();
    expect(screen.getByText("Google Chrome Helper (Renderer)")).toBeInTheDocument();
  });

  it("disables checkbox for system processes", () => {
    render(ProcessTable, {
      props: {
        processes: [makeProc({ pid: 26, is_system: true })],
      },
    });
    expect(screen.getByRole("checkbox", { name: "Select TestApp" })).toBeDisabled();
  });
});

describe("sorting", () => {
  it("default sort is ram_mb descending", () => {
    const procs = [
      makeProc({ pid: 1, ram_mb: 10 }),
      makeProc({ pid: 2, ram_mb: 100 }),
      makeProc({ pid: 3, ram_mb: 50 }),
    ];
    render(ProcessTable, { props: { processes: procs } });
    const cells = screen.getAllByRole("cell");
    // Find PID cells — they are at indices 2, 9, 16 (7 cols per row: check, name, pid, ram, cpu, uptime, state)
    const pidCells = cells.filter((c) => /^\d+$/.test(c.textContent?.trim() ?? ""));
    const pidOrder = pidCells.map((c) => c.textContent?.trim());
    expect(pidOrder).toEqual(["2", "3", "1"]);
  });

  it("clicking Name header changes sort", async () => {
    const procs = [
      makeProc({ pid: 1, name: "Zebra" }),
      makeProc({ pid: 2, name: "Alpha" }),
    ];
    render(ProcessTable, { props: { processes: procs } });
    const nameHeader = screen.getByText("Name");
    await fireEvent.click(nameHeader);
    // Name sort defaults to ascending
    const ramHeader = screen.getByText("RAM");
    expect(ramHeader.closest("th")?.getAttribute("aria-sort")).toBe("none");
    expect(nameHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");
  });

  it("clicking same header toggles direction", async () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1 })] } });
    const ramHeader = screen.getByText("RAM");
    // Default is ram_mb descending
    expect(ramHeader.closest("th")?.getAttribute("aria-sort")).toBe("descending");
    await fireEvent.click(ramHeader);
    expect(ramHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");
  });

  it("clicking Group header starts ascending sort", async () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1 })] } });
    const groupHeader = screen.getByText("Group");
    await fireEvent.click(groupHeader);
    expect(groupHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");
  });

  it("clicking ST header starts descending sort", async () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1, state: "S" }), makeProc({ pid: 2, state: "R" })] } });
    const stateHeader = screen.getByText("ST");
    await fireEvent.click(stateHeader);
    expect(stateHeader.closest("th")?.getAttribute("aria-sort")).toBe("descending");
  });

  it("toggles all sortable headers through ascending and descending states", async () => {
    render(ProcessTable, {
      props: {
        processes: [
          makeProc({ pid: 10, cpu_pct: 55, ram_mb: 1400, uptime: "1m", state: "R" }),
          makeProc({ pid: 20, cpu_pct: 12, ram_mb: 400, uptime: "2m", state: "S" }),
        ],
      },
    });

    const nameHeader = screen.getByText("Name");
    const groupHeader = screen.getByText("Group");
    const cpuHeader = screen.getByText("CPU");
    const timeHeader = screen.getByText("Time");
    const pidHeader = screen.getByText("PID");
    const stateHeader = screen.getByText("ST");

    await fireEvent.click(nameHeader);
    await fireEvent.click(nameHeader);
    expect(nameHeader.closest("th")?.getAttribute("aria-sort")).toBe("descending");

    await fireEvent.click(groupHeader);
    await fireEvent.click(groupHeader);
    expect(groupHeader.closest("th")?.getAttribute("aria-sort")).toBe("descending");

    await fireEvent.click(cpuHeader);
    await fireEvent.click(cpuHeader);
    expect(cpuHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");

    await fireEvent.click(timeHeader);
    await fireEvent.click(timeHeader);
    expect(timeHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");

    await fireEvent.click(pidHeader);
    await fireEvent.click(pidHeader);
    expect(pidHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");

    await fireEvent.click(stateHeader);
    await fireEvent.click(stateHeader);
    expect(stateHeader.closest("th")?.getAttribute("aria-sort")).toBe("ascending");
  });
});

describe("grouping", () => {
  it("renders group headers when grouping is enabled", () => {
    const procs = [
      makeProc({ pid: 1, name: "Chrome" }),
      makeProc({ pid: 2, name: "Chrome" }),
      makeProc({ pid: 3, name: "Safari" }),
    ];
    render(ProcessTable, { props: { processes: procs, grouping: true } });
    // Chrome group has 2 procs so gets a group header
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBeGreaterThanOrEqual(1);
  });

  it("collapse/expand toggles group rows", async () => {
    const procs = [
      makeProc({ pid: 1, name: "Chrome", ram_mb: 100 }),
      makeProc({ pid: 2, name: "Chrome", ram_mb: 200 }),
    ];
    render(ProcessTable, { props: { processes: procs, grouping: true } });

    // Initially expanded - both child rows visible
    let rows = screen.getAllByRole("row");
    const initialCount = rows.length;

    // Click group header to collapse
    const groupButton = screen.getByRole("button");
    await fireEvent.click(groupButton);

    rows = screen.getAllByRole("row");
    expect(rows.length).toBeLessThan(initialCount);
  });

  it("shows group count in header", () => {
    const procs = [
      makeProc({ pid: 1, name: "Chrome", ram_mb: 100 }),
      makeProc({ pid: 2, name: "Chrome", ram_mb: 200 }),
    ];
    render(ProcessTable, { props: { processes: procs, grouping: true } });
    // Group meta shows "2 · 300 MB · 10.2%"
    expect(screen.getByText(/2\s+·\s+300\s+MB/)).toBeInTheDocument();
  });

  it("supports keyboard toggle for group headers", async () => {
    const procs = [
      makeProc({ pid: 1, name: "Chrome", ram_mb: 100 }),
      makeProc({ pid: 2, name: "Chrome", ram_mb: 200 }),
    ];
    render(ProcessTable, { props: { processes: procs, grouping: true } });

    const groupButton = screen.getByRole("button");
    expect(groupButton).toHaveAttribute("aria-expanded", "true");

    await fireEvent.keyDown(groupButton, { key: "Enter" });
    expect(groupButton).toHaveAttribute("aria-expanded", "false");

    await fireEvent.keyDown(groupButton, { key: " " });
    expect(groupButton).toHaveAttribute("aria-expanded", "true");
  });

  it("ignores unrelated key presses on group header", async () => {
    const procs = [
      makeProc({ pid: 1, name: "Chrome", ram_mb: 100 }),
      makeProc({ pid: 2, name: "Chrome", ram_mb: 200 }),
    ];
    render(ProcessTable, { props: { processes: procs, grouping: true } });

    const groupButton = screen.getByRole("button");
    await fireEvent.keyDown(groupButton, { key: "A" });
    expect(groupButton).toHaveAttribute("aria-expanded", "true");
  });
});

describe("selection", () => {
  it("click selects a row", async () => {
    const procs = [makeProc({ pid: 42 })];
    render(ProcessTable, { props: { processes: procs } });
    const row = screen.getAllByRole("row")[1]; // skip header
    await fireEvent.click(row);
    expect(get(selectedPids).has(42)).toBe(true);
    expect(get(focusedPid)).toBe(42);
  });

  it("double-click calls oninspect", async () => {
    const spy = vi.fn();
    const procs = [makeProc({ pid: 42 })];
    render(ProcessTable, { props: { processes: procs, oninspect: spy } });
    const row = screen.getAllByRole("row")[1];
    await fireEvent.dblClick(row);
    expect(spy).toHaveBeenCalledWith(expect.objectContaining({ pid: 42 }));
  });

  it("double-click without oninspect does not throw", async () => {
    const procs = [makeProc({ pid: 99 })];
    render(ProcessTable, { props: { processes: procs } });
    const row = screen.getAllByRole("row")[1];
    await expect(fireEvent.dblClick(row)).resolves.toBe(true);
  });

  it("checkbox toggles selection without row click side effects", async () => {
    const procs = [makeProc({ pid: 55 })];
    render(ProcessTable, { props: { processes: procs } });
    const checkbox = screen.getByRole("checkbox", { name: "Select TestApp" });

    await fireEvent.click(checkbox);

    expect(get(selectedPids).has(55)).toBe(true);
    expect(get(focusedPid)).toBeNull();
  });

  it("renders checkbox as checked when PID is preselected", () => {
    selectedPids.set(new Set([56]));
    render(ProcessTable, { props: { processes: [makeProc({ pid: 56 })] } });
    expect(screen.getByRole("checkbox", { name: "Select TestApp" })).toBeChecked();
  });
});

describe("virtual scrolling", () => {
  it("renders spacer rows for long lists", async () => {
    const procs = Array.from({ length: 120 }, (_, i) => makeProc({ pid: i + 1, ram_mb: 500 - i }));
    render(ProcessTable, { props: { processes: procs } });

    expect(document.querySelectorAll("tr.spacer").length).toBeGreaterThan(0);

    const wrap = document.querySelector(".table-wrap") as HTMLDivElement;
    wrap.scrollTop = 420;
    await fireEvent.scroll(wrap);

    const spacerCells = Array.from(document.querySelectorAll("tr.spacer td"));
    expect(spacerCells.some((el) => (el as HTMLElement).style.height !== "0px")).toBe(true);
  });

  it("throttles scroll events while RAF callback is pending", async () => {
    const procs = Array.from({ length: 80 }, (_, i) => makeProc({ pid: i + 1 }));
    let rafCb: FrameRequestCallback | undefined;
    const rafSpy = vi
      .spyOn(globalThis, "requestAnimationFrame")
      .mockImplementation((cb: FrameRequestCallback): number => {
        rafCb = cb;
        return 1;
      });
    const cancelSpy = vi.spyOn(globalThis, "cancelAnimationFrame").mockImplementation(() => {});

    render(ProcessTable, { props: { processes: procs } });
    const wrap = document.querySelector(".table-wrap") as HTMLDivElement;

    await fireEvent.scroll(wrap);
    await fireEvent.scroll(wrap);
    expect(rafSpy).toHaveBeenCalledTimes(1);

    rafCb?.(16);

    rafSpy.mockRestore();
    cancelSpy.mockRestore();
  });

  it("updates container height from ResizeObserver callback", () => {
    const procs = Array.from({ length: 10 }, (_, i) => makeProc({ pid: i + 1 }));
    let resizeCb: ResizeObserverCallback | undefined;
    const observe = vi.fn();
    const disconnect = vi.fn();

    class ResizeObserverMock {
      constructor(cb: ResizeObserverCallback) {
        resizeCb = cb;
      }
      observe = observe;
      disconnect = disconnect;
    }

    vi.stubGlobal("ResizeObserver", ResizeObserverMock);

    const { unmount } = render(ProcessTable, { props: { processes: procs } });
    const wrap = document.querySelector(".table-wrap") as HTMLDivElement;
    const entry = [{ contentRect: { height: 420 } } as ResizeObserverEntry];
    resizeCb?.(entry, {} as ResizeObserver);

    expect(observe).toHaveBeenCalledWith(wrap);
    unmount();
    expect(disconnect).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("renders top spacer after scrolling with immediate RAF", async () => {
    const procs = Array.from({ length: 120 }, (_, i) => makeProc({ pid: i + 1, ram_mb: 200 - i }));
    const rafSpy = vi
      .spyOn(globalThis, "requestAnimationFrame")
      .mockImplementation((cb: FrameRequestCallback): number => {
        cb(0);
        return 1;
      });
    const cancelSpy = vi.spyOn(globalThis, "cancelAnimationFrame").mockImplementation(() => {});

    render(ProcessTable, { props: { processes: procs } });
    const wrap = document.querySelector(".table-wrap") as HTMLDivElement;
    wrap.scrollTop = 520;
    await fireEvent.scroll(wrap);

    const firstSpacerCell = document.querySelector("tr.spacer td") as HTMLTableCellElement;
    expect(Number.parseInt(firstSpacerCell.style.height, 10)).toBeGreaterThan(0);

    rafSpy.mockRestore();
    cancelSpy.mockRestore();
  });

  it("does not render spacer rows for short lists", () => {
    render(ProcessTable, { props: { processes: [makeProc({ pid: 1 })] } });
    expect(document.querySelectorAll("tr.spacer").length).toBe(0);
  });

  it("re-evaluates grouping paths when grouping prop changes", async () => {
    const procs = [makeProc({ pid: 1, name: "Same" }), makeProc({ pid: 2, name: "Same" })];
    const view = render(ProcessTable, { props: { processes: procs, grouping: true } });
    expect(screen.getByRole("button")).toBeInTheDocument();

    await view.rerender({ processes: procs, grouping: false });
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});

describe("XSS safety", () => {
  it("renders malicious process name as escaped text", () => {
    const procs = [makeProc({ pid: 1, name: '<script>alert("xss")</script>' })];
    render(ProcessTable, { props: { processes: procs } });
    // The script tag should be visible as text, not executed
    expect(screen.getByText('<script>alert("xss")</script>')).toBeInTheDocument();
    // No actual script element in the DOM
    expect(document.querySelector("script")).toBeNull();
  });
});
