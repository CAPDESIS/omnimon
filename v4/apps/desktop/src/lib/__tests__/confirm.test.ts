import { describe, expect, it } from "vitest";

import { confirmAction, confirmDialogState, resolveConfirmDialog } from "../confirm";

describe("confirm", () => {
  it("abre el dialogo y resuelve true al confirmar", async () => {
    const promise = confirmAction("Delete item?");

    let snapshot;
    confirmDialogState.subscribe((value) => {
      snapshot = value;
    })();

    expect(snapshot).toMatchObject({ open: true, message: "Delete item?" });

    resolveConfirmDialog(true);
    await expect(promise).resolves.toBe(true);
  });
});
