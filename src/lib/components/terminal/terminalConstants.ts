/**
 * Runtime numeric constants for the Terminal component.
 *
 * Visual constants (padding, sizing) live as CSS custom properties in
 * src/app.css so they can be themed alongside the rest of the design
 * tokens. Anything that's only meaningful to the JS runtime (poll
 * intervals, debounce windows) stays here.
 */

/** How often we sample the foreground process name to drive the AI shortcut bar. */
export const PROCESS_POLL_INTERVAL_MS = 5000;

/** Delay after the last resize event before we consider the layout stable. */
export const RESIZE_STABILITY_DELAY_MS = 50;

/** Debounce window applied to resize events to coalesce rapid window resizes. */
export const RESIZE_DEBOUNCE_MS = 16;
