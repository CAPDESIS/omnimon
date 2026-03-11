/**
 * Shared constants used across the application.
 * Centralises magic numbers so they can be tuned from a single place.
 */

// ---------------------------------------------------------------------------
// App Version (injected from package.json at build time via Vite define)
// ---------------------------------------------------------------------------

/** Application version — single source of truth for the frontend. */
export const APP_VERSION: string = __APP_VERSION__;

// ---------------------------------------------------------------------------
// Process Table
// ---------------------------------------------------------------------------

/** Number of extra rows rendered above/below the visible viewport. */
export const PROCESS_TABLE_ROW_BUFFER = 3;

/** RAM thresholds (MB) for color coding in the process table. */
export const RAM_THRESHOLD_DANGER = 1024;
export const RAM_THRESHOLD_WARNING = 256;

/** CPU thresholds (%) for color coding in the process table. */
export const CPU_THRESHOLD_DANGER = 50;
export const CPU_THRESHOLD_WARNING = 10;

/** Energy impact score thresholds for color coding. */
export const ENERGY_THRESHOLD_DANGER = 60;
export const ENERGY_THRESHOLD_WARNING = 20;

// ---------------------------------------------------------------------------
// Network Map
// ---------------------------------------------------------------------------

/** Default height of the network map panel in pixels. */
export const NETWORK_PANEL_DEFAULT_HEIGHT = 280;

/** Default width of the network side panel in pixels. */
export const NETWORK_SIDE_PANEL_DEFAULT_WIDTH = 320;

/** Left margin for the canvas-based connection map (in CSS pixels). */
export const NETWORK_CANVAS_LEFT_MARGIN = 120;

/** Right-side inset for domain labels on the canvas (subtracted from width). */
export const NETWORK_CANVAS_RIGHT_INSET = 140;

// ---------------------------------------------------------------------------
// AI Defaults
// ---------------------------------------------------------------------------

/** Default AI provider used when the user has not configured one yet. */
export const AI_DEFAULT_PROVIDER = "openrouter";

/** Default AI model used when the user has not configured one yet. */
export const AI_DEFAULT_MODEL = "meta-llama/llama-3.2-3b-instruct:free";

/** Timeout (ms) for a single AI chat request before we abort. */
export const AI_CHAT_TIMEOUT_MS = 45_000;

// ---------------------------------------------------------------------------
// Byte Conversion
// ---------------------------------------------------------------------------

/** Bytes per megabyte (binary, 1 MB = 1 048 576 B). */
export const BYTES_PER_MB = 1_048_576;

/** Bytes per kilobyte (binary, 1 KB = 1 024 B). */
export const BYTES_PER_KB = 1024;
