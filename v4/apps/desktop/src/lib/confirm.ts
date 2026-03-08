import { writable, get } from "svelte/store";

export interface ConfirmDialogState {
  open: boolean;
  message: string;
  resolve: ((value: boolean) => void) | null;
}

export const confirmDialogState = writable<ConfirmDialogState>({
  open: false,
  message: "",
  resolve: null,
});

/**
 * Shows a custom in-app confirmation dialog.
 * Returns a Promise that resolves to true (confirm) or false (cancel).
 * Replaces window.confirm() which doesn't work reliably in Tauri v2 WKWebView.
 */
export function confirmAction(message: string): Promise<boolean> {
  // Close any previous dialog first
  const prev = get(confirmDialogState);
  if (prev.resolve) {
    prev.resolve(false);
  }

  return new Promise((resolve) => {
    confirmDialogState.set({
      open: true,
      message,
      resolve,
    });
  });
}

export function resolveConfirmDialog(value: boolean): void {
  const state = get(confirmDialogState);
  if (state.resolve) {
    state.resolve(value);
  }
  confirmDialogState.set({ open: false, message: "", resolve: null });
}
