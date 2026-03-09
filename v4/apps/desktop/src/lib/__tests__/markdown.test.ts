import { describe, expect, it } from "vitest";

import { renderMarkdown } from "../markdown";

describe("renderMarkdown", () => {
  it("convierte bloques de codigo y listas a HTML", () => {
    const html = renderMarkdown("```ts\nconst x = 1\n```\n\n- item");

    expect(html).toContain("<pre><code>const x = 1</code></pre>");
    expect(html).toContain("<ul><li>item</li></ul>");
  });
});
