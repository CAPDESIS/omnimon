import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createActionGuard, DEFAULT_ACTION_COOLDOWN_MS } from "../actionGuard";

describe("createActionGuard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-01-01T00:00:00.000Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("blocks rapid synchronous calls during the default 500ms cooldown", () => {
    const guard = createActionGuard();
    const fn = vi.fn((n: number) => n * 2);

    expect(guard.run(fn, 2)).toBe(4);
    expect(guard.run(fn, 3)).toBeUndefined();
    expect(fn).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(DEFAULT_ACTION_COOLDOWN_MS - 1);
    expect(guard.run(fn, 4)).toBeUndefined();
    expect(fn).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(1);
    expect(guard.run(fn, 5)).toBe(10);
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("allows immediate rapid repeat when cooldownMs is 0", () => {
    const guard = createActionGuard({ cooldownMs: 0 });
    const fn = vi.fn((x: string) => x.toUpperCase());

    expect(guard.run(fn, "a")).toBe("A");
    expect(guard.run(fn, "b")).toBe("B");
    expect(guard.run(fn, "c")).toBe("C");
    expect(fn).toHaveBeenCalledTimes(3);
  });

  it("enforces single-flight lock while an async promise is unsettled even past cooldownMs", async () => {
    let resolvePending!: (value: string) => void;
    const guard = createActionGuard({ cooldownMs: 500 });
    const asyncFn = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          resolvePending = resolve;
        }),
    );

    const first = guard.run(asyncFn);
    expect(guard.isLocked()).toBe(true);
    expect(guard.run(asyncFn)).toBeUndefined();

    vi.advanceTimersByTime(2000);
    expect(guard.run(asyncFn)).toBeUndefined();
    expect(asyncFn).toHaveBeenCalledTimes(1);

    resolvePending("done");
    await expect(first).resolves.toBe("done");
    expect(guard.isLocked()).toBe(false);

    expect(guard.run(asyncFn)).toBeInstanceOf(Promise);
    expect(asyncFn).toHaveBeenCalledTimes(2);
  });

  it("releases single-flight lock when async promise rejects or sync function throws", async () => {
    const guard = createActionGuard({ cooldownMs: 0 });

    const failingAsync = guard.wrap(async () => {
      throw new Error("boom");
    });
    await expect(failingAsync()).rejects.toThrow("boom");
    expect(guard.isLocked()).toBe(false);

    const failingSync = guard.wrap(() => {
      throw new Error("sync-boom");
    });
    expect(() => failingSync()).toThrow("sync-boom");
    expect(guard.isLocked()).toBe(false);
  });

  it("resets lock and cooldown state via reset()", () => {
    const guard = createActionGuard({ cooldownMs: 500 });
    const fn = vi.fn();

    guard.run(fn);
    expect(guard.run(fn)).toBeUndefined();
    guard.reset();
    guard.run(fn);
    expect(fn).toHaveBeenCalledTimes(2);
  });
});
