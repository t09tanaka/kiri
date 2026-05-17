import type { Terminal as TerminalType } from '@xterm/xterm';
import type { FitAddon as FitAddonType } from '@xterm/addon-fit';

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
 */
export function fitTerminalToContainer(
  terminal: TerminalType,
  fitAddon: FitAddonType,
  container: HTMLElement
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
