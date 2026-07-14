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

/**
 * Smallest pane a divider drag may produce, in px. A pane this size still
 * shows its close button with room to spare, and anything narrower is too
 * small to read. Splits can go under it — the header keeps degrading, down to
 * close alone — but a drag should never be the thing that gets there.
 */
export const MIN_PANE_SIZE_PX = 72;
