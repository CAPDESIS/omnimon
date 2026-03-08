/**
 * Shared utilities for chat components (AIChat, AiCommandBar, ContextAiChat).
 */

/** Base chat message type used across all chat surfaces. */
export interface ChatMessage {
  role: "user" | "assistant" | "system" | "tool";
  text: string;
}

/**
 * Scrolls a container element to the bottom on the next animation frame.
 * Useful for auto-scrolling chat message lists after new messages arrive.
 */
export function scrollToBottom(container: HTMLElement | undefined): void {
  requestAnimationFrame(() => {
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  });
}

/**
 * Auto-resizes a textarea to fit its content up to a maximum height.
 * Resets to 0px first so shrinking works when text is deleted.
 *
 * @param inputRef  The textarea element to resize.
 * @param maxHeight Maximum height in pixels (default 180).
 */
export function resizeInput(inputRef: HTMLTextAreaElement | undefined, maxHeight = 180): void {
  if (!inputRef) return;
  inputRef.style.height = "0px";
  inputRef.style.height = `${Math.min(inputRef.scrollHeight, maxHeight)}px`;
}
