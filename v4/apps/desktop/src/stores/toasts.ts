import { writable } from "svelte/store";

export type ToastLevel = "info" | "success" | "warning" | "error";

export interface Toast {
  id: string;
  level: ToastLevel;
  title: string;
  message?: string;
  duration: number; // ms, 0 = sticky
  createdAt: number;
}

let nextId = 0;

export const toasts = writable<Toast[]>([]);

/** Add a toast notification. Returns the toast id for manual dismissal. */
export function addToast(
  level: ToastLevel,
  title: string,
  message?: string,
  duration = 4000,
): string {
  const id = `toast-${++nextId}`;
  const toast: Toast = { id, level, title, message, duration, createdAt: Date.now() };
  toasts.update((t) => [...t, toast]);

  if (duration > 0) {
    setTimeout(() => dismissToast(id), duration);
  }
  return id;
}

/** Remove a toast by id. */
export function dismissToast(id: string): void {
  toasts.update((t) => t.filter((x) => x.id !== id));
}

/** Convenience wrappers. */
export const toast = {
  info: (title: string, message?: string, duration?: number) => addToast("info", title, message, duration),
  success: (title: string, message?: string, duration?: number) => addToast("success", title, message, duration),
  warning: (title: string, message?: string, duration?: number) => addToast("warning", title, message, duration),
  error: (title: string, message?: string, duration?: number) => addToast("error", title, message, duration ?? 6000),
};

export function _resetToasts(): void {
  toasts.set([]);
  nextId = 0;
}
