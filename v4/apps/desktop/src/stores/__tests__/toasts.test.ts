import { describe, it, expect, beforeEach, vi } from "vitest";
import { get } from "svelte/store";
import { toasts, addToast, dismissToast, toast, _resetToasts } from "../toasts";

describe("toasts store", () => {
  beforeEach(() => {
    _resetToasts();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts with no toasts", () => {
    expect(get(toasts)).toEqual([]);
  });

  it("addToast creates a toast and returns an id", () => {
    const id = addToast("info", "Hello");
    expect(id).toMatch(/^toast-/);
    const list = get(toasts);
    expect(list).toHaveLength(1);
    expect(list[0].level).toBe("info");
    expect(list[0].title).toBe("Hello");
    expect(list[0].duration).toBe(4000);
  });

  it("addToast with message and custom duration", () => {
    addToast("error", "Oops", "Something went wrong", 10000);
    const t = get(toasts)[0];
    expect(t.message).toBe("Something went wrong");
    expect(t.duration).toBe(10000);
  });

  it("dismissToast removes by id", () => {
    const id1 = addToast("info", "First");
    const id2 = addToast("warning", "Second");
    expect(get(toasts)).toHaveLength(2);
    dismissToast(id1);
    const remaining = get(toasts);
    expect(remaining).toHaveLength(1);
    expect(remaining[0].id).toBe(id2);
  });

  it("auto-dismisses after duration", () => {
    addToast("success", "Quick", undefined, 2000);
    expect(get(toasts)).toHaveLength(1);
    vi.advanceTimersByTime(2000);
    expect(get(toasts)).toHaveLength(0);
  });

  it("sticky toast (duration 0) does not auto-dismiss", () => {
    addToast("error", "Sticky", undefined, 0);
    expect(get(toasts)).toHaveLength(1);
    vi.advanceTimersByTime(60000);
    expect(get(toasts)).toHaveLength(1);
  });

  it("convenience wrappers set correct levels", () => {
    toast.info("Info");
    toast.success("Success");
    toast.warning("Warning");
    toast.error("Error");
    const list = get(toasts);
    expect(list.map((t) => t.level)).toEqual(["info", "success", "warning", "error"]);
  });

  it("error toast defaults to 6s duration", () => {
    toast.error("Err");
    expect(get(toasts)[0].duration).toBe(6000);
  });
});
