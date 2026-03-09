import { describe, expect, it, vi } from "vitest";

import { resizeInput, scrollToBottom } from "../chatUtils";

describe("chatUtils", () => {
  it("hace scroll al fondo en el siguiente animation frame", () => {
    const container = document.createElement("div");
    Object.defineProperty(container, "scrollHeight", { value: 320, configurable: true });
    container.scrollTop = 0;

    const raf = vi.spyOn(globalThis, "requestAnimationFrame").mockImplementation((cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });

    scrollToBottom(container);

    expect(container.scrollTop).toBe(320);
    raf.mockRestore();
  });

  it("tolera contenedor undefined", () => {
    expect(() => scrollToBottom(undefined)).not.toThrow();
  });

  it("redimensiona textarea hasta el maximo permitido", () => {
    const textarea = document.createElement("textarea");
    Object.defineProperty(textarea, "scrollHeight", { value: 260, configurable: true });

    resizeInput(textarea, 180);

    expect(textarea.style.height).toBe("180px");
  });

  it("tolera input undefined", () => {
    expect(() => resizeInput(undefined)).not.toThrow();
  });
});
