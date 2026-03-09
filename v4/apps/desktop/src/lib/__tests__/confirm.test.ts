import { get } from "svelte/store";
import { describe, expect, it } from "vitest";

import { confirmAction, confirmDialogState, resolveConfirmDialog } from "../confirm";

describe("confirm", () => {
  it("abre el dialogo y resuelve true al confirmar", async () => {
    const promise = confirmAction("Delete item?");

    expect(get(confirmDialogState)).toMatchObject({ open: true, message: "Delete item?" });

    resolveConfirmDialog(true);
    await expect(promise).resolves.toBe(true);
    expect(get(confirmDialogState)).toMatchObject({ open: false, message: "", resolve: null });
  });

  it("resuelve false al cancelar", async () => {
    const promise = confirmAction("Cancel item?");
    resolveConfirmDialog(false);

    await expect(promise).resolves.toBe(false);
  });

  it("cierra el dialogo previo cuando se abre uno nuevo", async () => {
    const first = confirmAction("First dialog");
    const second = confirmAction("Second dialog");

    await expect(first).resolves.toBe(false);
    expect(get(confirmDialogState)).toMatchObject({ open: true, message: "Second dialog" });

    resolveConfirmDialog(true);
    await expect(second).resolves.toBe(true);
  });

  it("ignora resolver cuando no hay dialogo abierto", () => {
    resolveConfirmDialog(true);
    expect(get(confirmDialogState)).toMatchObject({ open: false, message: "", resolve: null });
  });
});
