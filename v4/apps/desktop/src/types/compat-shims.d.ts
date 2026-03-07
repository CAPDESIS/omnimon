declare module "@vitest/utils/display" {
  export function stringify(value: unknown, maxDepth?: number, options?: unknown): string;
}

declare module "@testing-library/svelte-core" {
  export class UnknownSvelteOptionsError extends Error {}
}
