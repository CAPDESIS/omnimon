import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { createRawSnippet, tick } from "svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import Button from "../Button.svelte";
import IconButton from "../IconButton.svelte";

function textSnippet(text: string) {
  return createRawSnippet(() => ({
    render: () => `<span>${text}</span>`,
  }));
}

describe("Button and IconButton anti-double-click guard", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("Button drops rapid double clicks within 500ms and allows click after cooldown", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    const onclick = vi.fn();
    render(Button, {
      props: {
        children: textSnippet("Save"),
        onclick,
      },
    });

    const btn = screen.getByRole("button", { name: "Save" });
    await fireEvent.click(btn);
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(499);
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(1);
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(2);
  });

  it("Button allows rapid repeat when cooldownMs={0}", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    const onclick = vi.fn();
    render(Button, {
      props: {
        children: textSnippet("A+"),
        cooldownMs: 0,
        onclick,
      },
    });

    const btn = screen.getByRole("button", { name: "A+" });
    await fireEvent.click(btn);
    await fireEvent.click(btn);
    await fireEvent.click(btn);
    expect(onclick).toHaveBeenCalledTimes(3);
  });

  it("Button enforces single-flight and disables while async onclick is in flight", async () => {
    let resolveClick!: () => void;
    const onclick = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveClick = resolve;
        }),
    );

    render(Button, {
      props: {
        children: textSnippet("Apply"),
        onclick,
      },
    });

    const btn = screen.getByRole("button", { name: "Apply" });
    btn.click();
    btn.click();

    expect(onclick).toHaveBeenCalledTimes(1);
    await tick();
    await waitFor(() => {
      expect(btn).toBeDisabled();
    });

    resolveClick();
    await waitFor(() => {
      expect(btn).not.toBeDisabled();
    });
  });

  it("IconButton drops rapid double clicks within 500ms and supports cooldownMs={0}", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
    const guardedClick = vi.fn();
    const rapidClick = vi.fn();

    render(IconButton, {
      props: {
        ariaLabel: "Close",
        children: textSnippet("×"),
        onclick: guardedClick,
      },
    });

    const closeBtn = screen.getByRole("button", { name: "Close" });
    await fireEvent.click(closeBtn);
    await fireEvent.click(closeBtn);
    expect(guardedClick).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(500);
    await fireEvent.click(closeBtn);
    expect(guardedClick).toHaveBeenCalledTimes(2);

    cleanup();
    render(IconButton, {
      props: {
        ariaLabel: "Step",
        cooldownMs: 0,
        children: textSnippet("+"),
        onclick: rapidClick,
      },
    });

    const stepBtn = screen.getByRole("button", { name: "Step" });
    await fireEvent.click(stepBtn);
    await fireEvent.click(stepBtn);
    expect(rapidClick).toHaveBeenCalledTimes(2);
  });
});
