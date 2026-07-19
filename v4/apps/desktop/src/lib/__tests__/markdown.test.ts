// @vitest-environment jsdom
// DOMPurify >=3.4 requires a standards-compliant DOM to sanitize; happy-dom
// parses <script> inconsistently there, so this suite runs under jsdom.
import { describe, expect, it } from "vitest";

import { renderMarkdown } from "../markdown";

describe("renderMarkdown", () => {
  it("convierte bloques de codigo y listas a HTML", () => {
    const html = renderMarkdown("```ts\nconst x = 1\n```\n\n- item");

    expect(html).toContain("<pre>");
    expect(html).toContain("const x = 1");
    expect(html).toContain("</code></pre>");
    expect(html).toContain("<li>item</li>");
  });

  it("sanitiza scripts e inline handlers XSS", () => {
    const html = renderMarkdown('<img src=x onerror="alert(1)"><script>alert(2)</script>');

    expect(html).not.toContain("<script>");
    expect(html).not.toContain("onerror");
    expect(html).toContain("<img src=\"x\">");
  });

  it("preserva markdown seguro mientras limpia javascript URLs", () => {
    const html = renderMarkdown('[safe](https://example.com) [xss](javascript:alert(1))');

    expect(html).toContain('href="https://example.com"');
    expect(html).not.toContain("javascript:alert");
  });

  it("tolera entrada vacia o malformada", () => {
    expect(renderMarkdown("")).toBe("");
    expect(renderMarkdown("<div><strong>broken")).toContain("<div><strong>broken</strong></div>");
  });
});
