import { describe, expect, it } from "vitest";

import { categoryLabel } from "../processIcons";

describe("processIcons", () => {
  it("retorna etiquetas humanas para categorias", () => {
    expect(categoryLabel("system")).toBe("System");
    expect(categoryLabel("files")).toBe("Files");
    expect(categoryLabel("default")).toBe("Application");
  });
});
