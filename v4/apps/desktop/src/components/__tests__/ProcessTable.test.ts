import { render, screen, fireEvent } from "@testing-library/svelte";
import ProcessTable from "../ProcessTable.svelte";
import type { ProcessEntry } from "../../lib/types";
import { selectedPids, focusedPid, _resetForTest } from "../../stores/processes";
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
    expect(screen.getByText("PID")).toBeInTheDocument();
    expect(screen.getByText("RAM")).toBeInTheDocument();
    expect(screen.getByText("CPU")).toBeInTheDocument();
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
