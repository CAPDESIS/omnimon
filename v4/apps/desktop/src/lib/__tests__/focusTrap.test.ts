import { describe, expect, it } from "vitest";

import {
  focusFirstFocusable,
  getFocusableElements,
  rememberActiveElement,
  restoreFocus,
  trapFocus,
} from "../focusTrap";

function buildContainer(): HTMLDivElement {
  const container = document.createElement("div");
  const first = document.createElement("button");
  first.textContent = "first";
  const hidden = document.createElement("button");
  hidden.setAttribute("aria-hidden", "true");
  const disabled = document.createElement("button");
  disabled.setAttribute("disabled", "true");
  const last = document.createElement("button");
  last.textContent = "last";
  container.append(first, hidden, disabled, last);
  document.body.append(container);
  return container;
}

describe("focusTrap", () => {
  it("obtiene solo elementos focusables visibles", () => {
    const container = buildContainer();

    const focusables = getFocusableElements(container);

    expect(focusables).toHaveLength(2);
    expect(focusables[0].textContent).toBe("first");
    expect(focusables[1].textContent).toBe("last");
    container.remove();
  });

  it("enfoca el primer elemento o el contenedor si no hay focusables", () => {
    const container = buildContainer();
    focusFirstFocusable(container);
    expect(document.activeElement).toBe(container.querySelector("button"));
    container.remove();

    const empty = document.createElement("div");
    empty.tabIndex = -1;
    document.body.append(empty);
    focusFirstFocusable(empty);
    expect(document.activeElement).toBe(empty);
    empty.remove();
  });

  it("recuerda y restaura el foco", () => {
    const button = document.createElement("button");
    document.body.append(button);
    button.focus();

    expect(rememberActiveElement()).toBe(button);

    const other = document.createElement("button");
    document.body.append(other);
    other.focus();
    restoreFocus(button);
    expect(document.activeElement).toBe(button);

    button.remove();
    other.remove();
  });

  it("hace wrap del foco con Tab y Shift+Tab", () => {
    const container = buildContainer();
    const [first, last] = getFocusableElements(container);

    last.focus();
    const tabEvent = new KeyboardEvent("keydown", { key: "Tab" });
    Object.defineProperty(tabEvent, "preventDefault", { value: vi.fn() });
    trapFocus(tabEvent, container);
    expect(document.activeElement).toBe(first);

    first.focus();
    const shiftTabEvent = new KeyboardEvent("keydown", { key: "Tab", shiftKey: true });
    Object.defineProperty(shiftTabEvent, "preventDefault", { value: vi.fn() });
    trapFocus(shiftTabEvent, container);
    expect(document.activeElement).toBe(last);

    container.remove();
  });
});
