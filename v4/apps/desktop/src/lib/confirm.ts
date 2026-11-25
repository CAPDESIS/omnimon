import { writable, get } from "svelte/store";

export interface ConfirmDialogItem {
  label: string;
  detail?: string;
  icon?: string | null;
  subItems?: string[];
}

export interface ConfirmDialogState {
  open: boolean;
  message: string;
  items: ConfirmDialogItem[];
  resolve: ((value: boolean) => void) | null;
  onAskAi?: (() => void) | null;
}

export const confirmDialogState = writable<ConfirmDialogState>({
  open: false,
  message: "",
  items: [],
  resolve: null,
  onAskAi: null,
});

/**
 * Shows a custom in-app confirmation dialog.
 * Returns a Promise that resolves to true (confirm) or false (cancel).
 * Replaces window.confirm() which doesn't work reliably in Tauri v2 WKWebView.
 * Enforces single-flight: if a confirm dialog is already open, ignores the overlapping
 * request and resolves it to false without replacing the active prompt.
 */
export function confirmAction(message: string, items: ConfirmDialogItem[] = [], onAskAi?: () => void): Promise<boolean> {
  const prev = get(confirmDialogState);
  if (prev.open || prev.resolve) {
    return Promise.resolve(false);
  }

  return new Promise((resolve) => {
    confirmDialogState.set({
      open: true,
      message,
      items,
      resolve,
      onAskAi: onAskAi ?? null,
    });
  });
}

export function resolveConfirmDialog(value: boolean): void {
  const state = get(confirmDialogState);
  if (!state.open && !state.resolve) {
    return;
  }
  const resolve = state.resolve;
  confirmDialogState.set({ open: false, message: "", items: [], resolve: null, onAskAi: null });
  resolve?.(value);
}
