import { render, screen, fireEvent } from "@testing-library/svelte";
import { writable } from "svelte/store";
import ToastContainer from "../ToastContainer.svelte";

const { mockToasts, mockDismiss } = vi.hoisted(() => {
  const { writable: w } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockToasts: w<Array<{ id: string; level: string; title: string; message?: string }>>([]),
    mockDismiss: vi.fn(),
  };
});

vi.mock("../../stores/toasts", () => ({
  toasts: mockToasts,
  dismissToast: mockDismiss,
}));

describe("ToastContainer", () => {
  beforeEach(() => {
    mockToasts.set([]);
    mockDismiss.mockClear();
  });

  it("renders nothing when no toasts", () => {
    render(ToastContainer);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("renders a toast with title", () => {
    mockToasts.set([{ id: "t1", level: "info", title: "Hello" }]);
    render(ToastContainer);
    expect(screen.getByText("Hello")).toBeInTheDocument();
  });

  it("renders toast message when provided", () => {
    mockToasts.set([{ id: "t2", level: "success", title: "Done", message: "All good" }]);
    render(ToastContainer);
    expect(screen.getByText("Done")).toBeInTheDocument();
    expect(screen.getByText("All good")).toBeInTheDocument();
  });

  it("renders correct icon for each level", () => {
    mockToasts.set([
      { id: "t1", level: "info", title: "Info" },
      { id: "t2", level: "success", title: "Success" },
      { id: "t3", level: "warning", title: "Warning" },
      { id: "t4", level: "error", title: "Error" },
    ]);
    render(ToastContainer);
    expect(screen.getByText("\u2139")).toBeInTheDocument();
    expect(screen.getByText("\u2713")).toBeInTheDocument();
    expect(screen.getByText("\u26A0")).toBeInTheDocument();
    expect(screen.getByText("\u2717")).toBeInTheDocument();
  });

  it("calls dismissToast when dismiss button clicked", async () => {
    mockToasts.set([{ id: "t1", level: "error", title: "Oops" }]);
    render(ToastContainer);
    const btn = screen.getByLabelText("Dismiss");
    await fireEvent.click(btn);
    expect(mockDismiss).toHaveBeenCalledWith("t1");
  });

  it("renders multiple toasts", () => {
    mockToasts.set([
      { id: "t1", level: "info", title: "First" },
      { id: "t2", level: "warning", title: "Second" },
    ]);
    render(ToastContainer);
    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("has aria-live polite for accessibility", () => {
    render(ToastContainer);
    const container = document.querySelector('[aria-live="polite"]');
    expect(container).toBeInTheDocument();
  });

  it("applies level-specific CSS class", () => {
    mockToasts.set([{ id: "t1", level: "error", title: "Err" }]);
    render(ToastContainer);
    const alert = screen.getByRole("alert");
    expect(alert.className).toContain("toast-error");
  });
});
