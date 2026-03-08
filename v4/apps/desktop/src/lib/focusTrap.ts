function isFocusable(element: HTMLElement): boolean {
  if (element.hasAttribute("disabled")) return false;
  if (element.getAttribute("aria-hidden") === "true") return false;
  return element.tabIndex >= 0;
}

export function getFocusableElements(container: HTMLElement | null | undefined): HTMLElement[] {
  if (!container) return [];
  return Array.from(
    container.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter(isFocusable);
}

export function focusFirstFocusable(container: HTMLElement | null | undefined): void {
  if (!container) return;
  const [first] = getFocusableElements(container);
  if (first) first.focus();
  else container.focus();
}

export function rememberActiveElement(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.activeElement instanceof HTMLElement ? document.activeElement : null;
}

export function restoreFocus(element: HTMLElement | null | undefined): void {
  if (!element || !element.isConnected) return;
  element.focus();
}

export function trapFocus(event: KeyboardEvent, container: HTMLElement | null | undefined): void {
  if (event.key !== "Tab" || !container) return;
  const focusable = getFocusableElements(container);
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];

  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
    return;
  }

  if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
