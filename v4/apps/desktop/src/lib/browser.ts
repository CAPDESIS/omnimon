import type { ProcessEntry, BrowserName } from "./types";

/**
 * Detect which browser a process belongs to based on its group,
 * executable name, and process name.
 *
 * Returns the browser name if the process is a known browser,
 * or null otherwise.
 */
export function detectBrowser(proc: ProcessEntry): BrowserName | null {
  if (proc.group !== "Browser") return null;
  if (proc.exec_name.includes("Google Chrome") || proc.name.includes("Chrome")) return "Chrome";
  if (proc.name === "com.apple.WebKit.WebContent" || proc.exec_name.includes("Safari") || proc.name.includes("Safari")) return "Safari";
  if (proc.exec_name.includes("Brave Browser") || proc.name.includes("Brave")) return "Brave";
  if (proc.exec_name.includes("Microsoft Edge") || proc.name.includes("Edge")) return "Edge";
  if (proc.exec_name.includes("Arc") || proc.name.includes("Arc")) return "Arc";
  if (proc.exec_name.includes("firefox") || proc.name.includes("firefox")) return "Firefox";
  return null;
}
