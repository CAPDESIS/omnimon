export const DEFAULT_ACTION_COOLDOWN_MS = 500;

export interface ActionGuardOptions {
  cooldownMs?: number;
  singleFlight?: boolean;
}

export interface ActionGuard {
  run<TArgs extends unknown[], TResult>(
    fn: (...args: TArgs) => TResult,
    ...args: TArgs
  ): TResult | undefined;
  wrap<TArgs extends unknown[], TResult>(
    fn: (...args: TArgs) => TResult,
  ): (...args: TArgs) => TResult | undefined;
  setCooldownMs(next: number): void;
  isLocked(): boolean;
  reset(): void;
}

function isPromiseLike<T>(value: unknown): value is PromiseLike<T> {
  return (
    value !== null &&
    (typeof value === "object" || typeof value === "function") &&
    typeof (value as PromiseLike<T>).then === "function"
  );
}

/**
 * Creates a re-entrancy and rapid-tap guard combining a cooldown window
 * with an async single-flight lock while a returned Promise is unsettled.
 */
export function createActionGuard(options: ActionGuardOptions = {}): ActionGuard {
  let cooldownMs = Math.max(0, options.cooldownMs ?? DEFAULT_ACTION_COOLDOWN_MS);
  const singleFlight = options.singleFlight ?? true;
  let inFlight = false;
  let lastTriggeredAt = -Infinity;

  function canRun(): boolean {
    if (singleFlight && inFlight) return false;
    if (cooldownMs > 0 && Date.now() - lastTriggeredAt < cooldownMs) return false;
    return true;
  }

  function run<TArgs extends unknown[], TResult>(
    fn: (...args: TArgs) => TResult,
    ...args: TArgs
  ): TResult | undefined {
    if (!canRun()) return undefined;
    if (cooldownMs > 0) {
      lastTriggeredAt = Date.now();
    }
    inFlight = true;
    try {
      const result = fn(...args);
      if (isPromiseLike(result)) {
        return Promise.resolve(result).finally(() => {
          inFlight = false;
        }) as unknown as TResult;
      }
      inFlight = false;
      return result;
    } catch (err) {
      inFlight = false;
      throw err;
    }
  }

  function wrap<TArgs extends unknown[], TResult>(
    fn: (...args: TArgs) => TResult,
  ): (...args: TArgs) => TResult | undefined {
    return (...args: TArgs) => run(fn, ...args);
  }

  return {
    run,
    wrap,
    setCooldownMs(next: number) {
      cooldownMs = Math.max(0, next);
    },
    isLocked: () => inFlight,
    reset() {
      inFlight = false;
      lastTriggeredAt = -Infinity;
    },
  };
}
