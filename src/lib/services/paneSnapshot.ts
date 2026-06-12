/**
 * Extract a pane's current on-screen text from an xterm buffer.
 *
 * Lives apart from the Svelte/xterm wiring so the row-selection logic is
 * unit-testable against a minimal fake buffer. The CLI's `term status`
 * uses this (via the cli bridge) to report what an agent is doing.
 */

/** Minimal slice of xterm's `IBufferLine` this module needs. */
export interface SnapshotBufferLine {
  translateToString(trimRight?: boolean): string;
}

/** Minimal slice of xterm's `IBuffer` this module needs. */
export interface SnapshotBuffer {
  readonly length: number;
  getLine(index: number): SnapshotBufferLine | undefined;
}

/**
 * Return the trailing `lines` non-blank-anchored rows of `buffer`, joined
 * by `\n`.
 *
 * Trailing fully-empty rows (xterm pads the buffer below the cursor) are
 * dropped so the snapshot ends at the last line with content — typically
 * an agent's status line or prompt. Interior blank lines are preserved.
 * Returns an empty string for a buffer with no content at all.
 */
export function snapshotBuffer(buffer: SnapshotBuffer, lines: number): string {
  const total = buffer.length;
  let lastNonEmpty = -1;
  for (let i = total - 1; i >= 0; i--) {
    const text = buffer.getLine(i)?.translateToString(true) ?? '';
    if (text.trim() !== '') {
      lastNonEmpty = i;
      break;
    }
  }
  if (lastNonEmpty < 0) return '';

  const count = Math.max(1, lines);
  const start = Math.max(0, lastNonEmpty - count + 1);
  const rows: string[] = [];
  for (let i = start; i <= lastNonEmpty; i++) {
    rows.push(buffer.getLine(i)?.translateToString(true) ?? '');
  }
  return rows.join('\n');
}
