/**
 * Runtime numeric constants for the Terminal component.
 *
 * Visual constants (padding, sizing) live as CSS custom properties in
 * src/app.css so they can be themed alongside the rest of the design
 * tokens. Anything that's only meaningful to the JS runtime (poll
 * intervals, debounce windows) stays here.
 */

/** Delay after the last resize event before we consider the layout stable. */
export const RESIZE_STABILITY_DELAY_MS = 50;

/** Debounce window applied to resize events to coalesce rapid window resizes. */
export const RESIZE_DEBOUNCE_MS = 16;
