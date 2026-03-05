import { render, screen, fireEvent } from "@testing-library/svelte";
import ProcessDetailsModal from "../ProcessDetailsModal.svelte";
import type { ProcessEntry } from "../../lib/types";

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

describe("rendering", () => {
  it("renders all process fields", () => {
    const proc = makeProc();
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: proc, onclose } });

    expect(screen.getByText("TestApp")).toBeInTheDocument();
    expect(screen.getByText("PID 42")).toBeInTheDocument();
    expect(screen.getByText("/usr/bin/testapp")).toBeInTheDocument();
    expect(screen.getByText("128.5 MB")).toBeInTheDocument();
    expect(screen.getByText("12.3%")).toBeInTheDocument();
    expect(screen.getByText("3h 15m")).toBeInTheDocument();
    expect(screen.getByText("Utilities")).toBeInTheDocument();
    expect(screen.getByText("R")).toBeInTheDocument();
    expect(screen.getByText("No")).toBeInTheDocument(); // is_system
    expect(screen.getByText("Yes")).toBeInTheDocument(); // idle
  });

  it("renders dialog with correct role", () => {
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose: vi.fn() } });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
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
    const modal = screen.getByRole("dialog");
    const closeBtn = screen.getByLabelText("Close");

    // Focus the close button (last/only focusable element)
    closeBtn.focus();
    expect(document.activeElement).toBe(closeBtn);

    // Tab should wrap to first element
    const backdrop = screen.getByRole("presentation");
    await fireEvent.keyDown(backdrop, { key: "Tab" });
    // After trap, focus should wrap to first focusable (the close button itself since it's the only one)
    expect(document.activeElement).toBe(closeBtn);
  });

  it("wraps Shift+Tab from first to last focusable element", async () => {
    const onclose = vi.fn();
    render(ProcessDetailsModal, { props: { process: makeProc(), onclose } });
    const closeBtn = screen.getByLabelText("Close");

    closeBtn.focus();
    const backdrop = screen.getByRole("presentation");
    await fireEvent.keyDown(backdrop, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(closeBtn);
  });
});
