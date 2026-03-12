import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import ConfirmDialog from "../ConfirmDialog.svelte";

const { mockConfirmDialogState, mockResolveConfirmDialog } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  type ConfirmItem = { label: string; detail?: string; icon?: string | null };
  return {
    mockConfirmDialogState: writable({
      open: false,
      message: "",
      items: [] as ConfirmItem[],
      resolve: null,
    }),
    mockResolveConfirmDialog: vi.fn(),
  };
});

vi.mock("../../lib/confirm", () => ({
  confirmDialogState: mockConfirmDialogState,
  resolveConfirmDialog: mockResolveConfirmDialog,
}));

describe("ConfirmDialog", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockConfirmDialogState.set({
      open: true,
      message: "Delete process Chrome?",
      items: [],
      resolve: null,
    });
    mockResolveConfirmDialog.mockClear();
  });

  it("renderiza con titulo y mensaje", () => {
    render(ConfirmDialog);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(screen.getByText("Delete process Chrome?")).toBeInTheDocument();
    expect(screen.getByText("Yes")).toBeInTheDocument();
    expect(screen.getByText("No")).toBeInTheDocument();
  });

  it("llama onConfirm al aceptar", async () => {
    render(ConfirmDialog);

    await fireEvent.click(screen.getByText("Yes"));

    expect(mockResolveConfirmDialog).toHaveBeenCalledWith(true);
  });

  it("llama onCancel al cancelar", async () => {
    render(ConfirmDialog);

    await fireEvent.click(screen.getByText("No"));

    expect(mockResolveConfirmDialog).toHaveBeenCalledWith(false);
  });

  it("se cierra con Escape", async () => {
    render(ConfirmDialog);

    await fireEvent.keyDown(window, { key: "Escape" });

    expect(mockResolveConfirmDialog).toHaveBeenCalledWith(false);
  });

  it("muestra lista de procesos cuando hay items", () => {
    mockConfirmDialogState.set({
      open: true,
      message: "Kill 2 processes?",
      items: [
        { label: "Chrome", detail: "PID 1234 · 5.2% CPU · 300 MB", icon: null },
        { label: "Firefox", detail: "PID 5678 · 3.1% CPU · 200 MB", icon: null },
      ],
      resolve: null,
    });

    render(ConfirmDialog);

    expect(screen.getByText("Kill 2 processes?")).toBeInTheDocument();
    expect(screen.getByText("Chrome")).toBeInTheDocument();
    expect(screen.getByText("Firefox")).toBeInTheDocument();
    expect(screen.getByText("PID 1234 · 5.2% CPU · 300 MB")).toBeInTheDocument();
    expect(screen.getByText("PID 5678 · 3.1% CPU · 200 MB")).toBeInTheDocument();
  });

  it("no muestra lista cuando items esta vacio", () => {
    render(ConfirmDialog);

    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });
});
