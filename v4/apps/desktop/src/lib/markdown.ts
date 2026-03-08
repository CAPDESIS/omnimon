/**
 * Shared lightweight Markdown-to-HTML renderer used by all chat components.
 * Handles: code blocks, inline code, headers, bold, italic, lists, paragraphs.
 */
export function renderMarkdown(text: string): string {
  let html = text
    // Escape HTML first
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    // Code blocks (``` ... ```)
    .replace(/```(\w*)\n([\s\S]*?)```/g, (_m, _lang, code) =>
      `<pre><code>${code.trim()}</code></pre>`)
    // Inline code
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    // Headers
    .replace(/^### (.+)$/gm, "<strong style='font-size:1.05em'>$1</strong>")
    .replace(/^## (.+)$/gm, "<strong style='font-size:1.1em;display:block;margin:6px 0 2px'>$1</strong>")
    .replace(/^# (.+)$/gm, "<strong style='font-size:1.2em;display:block;margin:8px 0 4px'>$1</strong>")
    // Bold
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    // Italic
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    // Unordered lists
    .replace(/^- (.+)$/gm, "<li>$1</li>")
    // Ordered lists
    .replace(/^\d+\. (.+)$/gm, "<li>$1</li>")
    // Line breaks (double newline = paragraph, single = br)
    .replace(/\n\n/g, "</p><p>")
    .replace(/\n/g, "<br>");

  // Wrap consecutive <li> in <ul>
  html = html.replace(/((?:<li>.*?<\/li>(?:<br>)?)+)/g, "<ul>$1</ul>");
  html = html.replace(/<ul>([\s\S]*?)<\/ul>/g, (_m, inner) =>
    "<ul>" + inner.replace(/<br>/g, "") + "</ul>");

  return `<p>${html}</p>`;
}
