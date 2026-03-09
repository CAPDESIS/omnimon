import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import ConfirmDialog from "../ConfirmDialog.svelte";

const { mockConfirmDialogState, mockResolveConfirmDialog } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockConfirmDialogState: writable({
      open: false,
      message: "",
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
});
