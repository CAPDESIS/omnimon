import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import InfoPopover from "../InfoPopover.svelte";

describe("InfoPopover", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("cancels pending blur timeout when toggle is invoked", async () => {
    render(InfoPopover, {
      props: {
        label: "Info",
        content: "Helpful details",
      },
    });

    const trigger = screen.getByRole("button", { name: "Info" });

    await fireEvent.click(trigger);
    expect(screen.getByRole("tooltip")).toHaveTextContent("Helpful details");

    vi.advanceTimersByTime(500);
    await fireEvent.click(trigger);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    // Schedule blur close in 120ms, then open via toggle 50ms later; the stale timer must not close it.
    vi.advanceTimersByTime(450);
    await fireEvent.blur(trigger);
    vi.advanceTimersByTime(50);
    await fireEvent.click(trigger);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();

    vi.advanceTimersByTime(150);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();
  });
});
