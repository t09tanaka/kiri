import type { Terminal as TerminalType } from '@xterm/xterm';
import type { FitAddon as FitAddonType } from '@xterm/addon-fit';
import { MIN_PANE_SIZE_PX } from './terminalConstants';

/**
 * Re-distribute the visible panes of a split around a divider dragged to
 * `position` (a percentage of the container along the split axis).
 *
 * Panes are sized purely by percentage, so nothing in the layout stops a drag
 * from collapsing one to zero. The floor is MIN_PANE_SIZE_PX, converted into
 * this container's percentage terms: past it the divider stops following the
 * cursor (signalled by a `null` return) instead of squeezing a pane out of
 * existence. Once a split holds so many panes that the floor cannot be met at
 * all, it is dropped — enforcing it there would freeze the divider outright.
 */
export function distributeDividerDrag(
  sizes: number[],
  dragIndex: number,
  position: number,
  containerSize: number
): number[] | null {
  const count = sizes.length;
  const minSize = containerSize > 0 ? (MIN_PANE_SIZE_PX / containerSize) * 100 : 0;
  const enforceMinSize = minSize * count <= 100;

  const next: number[] = [];

  // Common 2-pane case: split directly at the divider position.
  if (count === 2 && dragIndex === 0) {
    const lower = enforceMinSize ? minSize : 0;
    const first = Math.max(lower, Math.min(100 - lower, position));
    next.push(first, 100 - first);
  } else {
    let beforeTotal = 0;
    let afterTotal = 0;
    for (let i = 0; i <= dragIndex; i++) beforeTotal += sizes[i];
    for (let i = dragIndex + 1; i < count; i++) afterTotal += sizes[i];

    for (let i = 0; i < count; i++) {
      next.push(
        i <= dragIndex
          ? (sizes[i] / beforeTotal) * position
          : (sizes[i] / afterTotal) * (100 - position)
      );
    }
  }

  if (enforceMinSize && next.some((size) => size < minSize)) return null;
  return next;
}

/**
 * Reserve 1 row for the PTY to prevent Ink full-height flickering.
 * Ref: https://github.com/vadimdemedes/ink/issues/450 — when Ink renders
 * at exactly terminal height, scrolling kicks in and the bottom row
 * flickers between draws.
 */
export const PTY_ROW_MARGIN = 1;

/**
 * Apply the row margin used by the PTY backend. We clamp to a sane
 * minimum so panes that briefly collapse to near-zero rows don't send
 * a 0/negative size to the shell (which crashes Ink-based apps).
 */
export function applyPtyRowMargin(rows: number): number {
  return Math.max(rows - PTY_ROW_MARGIN, 10);
}

/**
 * Fit xterm to its container, guarding against 0-size fits that happen
 * momentarily while tabs switch or panes close. Returns true if a fit
 * was attempted.
 *
 * When `pinToBottom` is set, the viewport is scrolled to the bottom after
 * a dimension change. Growing or shrinking a pane reflows the buffer and
 * can leave the viewport showing stale rows instead of the live prompt;
 * pinning keeps the latest output in view on every resize.
 */
export function fitTerminalToContainer(
  terminal: TerminalType,
  fitAddon: FitAddonType,
  container: HTMLElement,
  options: { pinToBottom?: boolean } = {}
): boolean {
  const rect = container.getBoundingClientRect();
  if (rect.width < 2 || rect.height < 2) {
    return false;
  }

  try {
    const dimensions = fitAddon.proposeDimensions();
    if (!dimensions) return false;

    const { cols, rows } = dimensions;
    if (terminal.cols !== cols || terminal.rows !== rows) {
      terminal.resize(cols, rows);
      if (options.pinToBottom) {
        terminal.scrollToBottom();
      }
    }
    return true;
  } catch (e) {
    console.warn('FitAddon.fit() error:', e);
    return false;
  }
}

/**
 * Wait for the initial layout to settle, then size xterm to the actual
 * container. Used during first-time terminal creation so the PTY is
 * spawned with the correct initial dimensions — critical for Ink-based
 * apps like Claude Code which depend on the first SIGWINCH being right.
 *
 * Falls back to a second attempt if the first pass produced suspiciously
 * small dimensions, which happens when fonts are not yet loaded or the
 * container is mid-layout.
 */
export function waitForInitialLayout(
  terminal: TerminalType,
  fitAddon: FitAddonType,
  container: HTMLElement
): Promise<void> {
  return new Promise((resolve) => {
    document.fonts.ready.then(() => {
      setTimeout(() => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            const dimensions = fitAddon.proposeDimensions();
            if (dimensions) {
              terminal.resize(dimensions.cols, dimensions.rows);
            }

            const containerRect = container.getBoundingClientRect();
            // ~18px per row (fontSize 15px * lineHeight 1.2). Half that is
            // a conservative minimum — we only retry when the result is
            // less than half of what the container could fit.
            const estimatedRowHeight = 18;
            const expectedMinRows =
              Math.floor((containerRect.height - 24) / estimatedRowHeight) * 0.5;

            if (terminal.cols < 40 || terminal.rows < 10 || terminal.rows < expectedMinRows) {
              setTimeout(() => {
                const dims = fitAddon.proposeDimensions();
                if (dims) {
                  terminal.resize(dims.cols, dims.rows);
                }
                resolve();
              }, 100);
              return;
            }
            resolve();
          });
        });
      }, 100);
    });
  });
}
