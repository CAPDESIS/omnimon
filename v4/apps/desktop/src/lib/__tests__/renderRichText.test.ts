import { renderMarkdown } from "../renderRichText";

describe("renderMarkdown", () => {
  it("renders bold and lists", () => {
    const html = renderMarkdown("**Hello**\n- one\n- two");
    expect(html).toContain("<strong>Hello</strong>");
    expect(html).toContain("<ul>");
    expect(html).toContain("<li>one</li>");
  });

  it("escapes raw HTML", () => {
    const html = renderMarkdown("<script>alert(1)</script>");
    expect(html).toContain("&lt;script&gt;");
  });
});
