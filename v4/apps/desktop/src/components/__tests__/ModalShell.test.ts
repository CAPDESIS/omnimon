import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import ModalShell from "../ModalShell.svelte";

function modalBodySnippet() {
  return createRawSnippet(() => ({
    render: () => '<div data-testid="modal-inner"><h2 id="m-title">Title</h2></div>',
  }));
}

describe("ModalShell single-close behavior", () => {
  afterEach(() => {
    cleanup();
  });

  it("fires onclose only once when mousedown and click both hit the backdrop", async () => {
    const onclose = vi.fn();
    render(ModalShell, {
      props: {
        titleId: "m-title",
        onclose,
        children: modalBodySnippet(),
      },
    });

    const backdrop = screen.getByRole("presentation");
    await fireEvent.mouseDown(backdrop);
    await fireEvent.click(backdrop);
    await fireEvent.keyDown(backdrop, { key: "Escape" });

    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it("does not close when clicking inside the modal panel", async () => {
    const onclose = vi.fn();
    render(ModalShell, {
      props: {
        titleId: "m-title",
        onclose,
        children: modalBodySnippet(),
      },
    });

    const inner = screen.getByTestId("modal-inner");
    await fireEvent.mouseDown(inner);
    await fireEvent.click(inner);

    expect(onclose).not.toHaveBeenCalled();
  });

  it("respects closeOnBackdrop={false} and closeOnEscape={false}", async () => {
    const onclose = vi.fn();
    render(ModalShell, {
      props: {
        titleId: "m-title",
        closeOnBackdrop: false,
        closeOnEscape: false,
        onclose,
        children: modalBodySnippet(),
      },
    });

    const backdrop = screen.getByRole("presentation");
    await fireEvent.mouseDown(backdrop);
    await fireEvent.click(backdrop);
    await fireEvent.keyDown(backdrop, { key: "Escape" });

    expect(onclose).not.toHaveBeenCalled();
  });
});
