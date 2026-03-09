/**
 * Shared lightweight Markdown-to-HTML renderer used by all chat components.
 * Handles: code blocks, inline code, headers, bold, italic, lists, paragraphs.
 */
import { marked } from "marked";
import DOMPurify from "dompurify";

marked.setOptions({
  breaks: true,
  gfm: true,
});

export function renderMarkdown(raw: string): string {
  // Promise handling workaround for sync return type
  const html = marked.parse(raw) as string;
  return DOMPurify.sanitize(html);
}
