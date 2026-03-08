export function renderMarkdown(text: string): string {
  let html = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/```(\w*)\n([\s\S]*?)```/g, (_m, _lang, code) =>
      `<pre><code>${code.trim()}</code></pre>`)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/^### (.+)$/gm, "<strong style='font-size:1.05em'>$1</strong>")
    .replace(/^## (.+)$/gm, "<strong style='font-size:1.1em;display:block;margin:6px 0 2px'>$1</strong>")
    .replace(/^# (.+)$/gm, "<strong style='font-size:1.2em;display:block;margin:8px 0 4px'>$1</strong>")
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    .replace(/^- (.+)$/gm, "<li>$1</li>")
    .replace(/^\d+\. (.+)$/gm, "<li>$1</li>")
    .replace(/\n\n/g, "</p><p>")
    .replace(/\n/g, "<br>");

  html = html.replace(/((?:<li>.*?<\/li>(?:<br>)?)+)/g, "<ul>$1</ul>");
  html = html.replace(/<ul>([\s\S]*?)<\/ul>/g, (_m, inner) =>
    "<ul>" + inner.replace(/<br>/g, "") + "</ul>");

  return `<p>${html}</p>`;
}
