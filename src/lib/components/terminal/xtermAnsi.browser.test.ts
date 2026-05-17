/**
 * Fixture-driven tests for xterm.js rendering of edge-case PTY output.
 *
 * The kiri Terminal component delegates rendering to xterm.js; the
 * shapes most likely to break are ANSI SGR sequences, OSC sequences,
 * mixed line endings, super-long lines, and binary stdout sprayed
 * mid-stream. These tests pipe captured PTY-style fixtures into a
 * standalone xterm instance and assert it doesn't throw, produces a
 * cursor position consistent with the line count, and (where
 * meaningful) leaves the buffer's text content matching expectation.
 */

import { afterEach, describe, expect, it } from 'vitest';
import { Terminal } from '@xterm/xterm';

interface AnsiFixture {
  readonly name: string;
  readonly bytes: string;
  /**
   * Assertion run after every byte has been written. Receives the
   * Terminal so the test can inspect cursor / buffer state.
   */
  readonly assert: (t: Terminal) => void;
}

const FIXTURES: readonly AnsiFixture[] = [
  {
    name: 'plain ASCII line',
    bytes: 'hello world\r\n',
    assert(t) {
      expect(t.buffer.active.cursorY).toBe(1);
      const line = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(line).toBe('hello world');
    },
  },

  {
    name: 'SGR colour escapes do not push extra glyphs',
    bytes: '\x1b[31mred\x1b[0m\r\n',
    assert(t) {
      const line = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(line).toBe('red');
      expect(t.buffer.active.cursorY).toBe(1);
    },
  },

  {
    name: 'cursor up + clear line sequence',
    bytes: 'one\r\ntwo\r\n\x1b[A\x1b[2K',
    assert(t) {
      // After the writes the cursor was on row 2; \x1b[A moves it
      // back to row 1, \x1b[2K clears that row. Row 0 stays "one".
      expect(t.buffer.active.cursorY).toBe(1);
      const r0 = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      const r1 = t.buffer.active.getLine(1)?.translateToString(true) ?? '';
      expect(r0).toBe('one');
      expect(r1).toBe('');
    },
  },

  {
    name: 'mixed CR / LF / CRLF endings',
    bytes: 'a\nb\r\nc\rd\r\n',
    assert(t) {
      // \r alone returns to column 0 without scrolling, so `d` lands
      // on top of `c`. We don't pin the exact line count (xterm's
      // behavior on bare \n vs CR is well-defined but verbose);
      // the contract is "no throw, cursor on a real row".
      expect(t.buffer.active.cursorY).toBeGreaterThanOrEqual(0);
      expect(t.buffer.active.cursorY).toBeLessThanOrEqual(t.rows);
    },
  },

  {
    name: 'super-long line beyond column count wraps',
    bytes: 'x'.repeat(300) + '\r\n',
    assert(t) {
      // With cols=80, 300 chars wrap to 4 visual rows (75x4=300, with
      // CRLF tipping over to row 4). The bug we guard against is
      // xterm.js throwing on > 256 column input.
      expect(t.buffer.active.cursorY).toBeGreaterThanOrEqual(3);
    },
  },

  {
    name: 'binary stdout (null bytes + high-bit) must not throw',
    bytes: 'before\x00\x01\x02\xff\xfeafter\r\n',
    assert(t) {
      expect(t.buffer.active.cursorY).toBe(1);
      // Pretty hard to make a meaningful content assertion on
      // arbitrary bytes; the bar is "no exception, cursor advanced".
    },
  },

  {
    name: 'OSC 0 (set window title) is consumed and does not appear in buffer',
    bytes: '\x1b]0;my title\x07visible\r\n',
    assert(t) {
      const line = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(line).toBe('visible');
    },
  },

  {
    name: 'DCS sequence (Sixel-style) consumed without buffer dump',
    bytes: '\x1bPq\x1b\\after\r\n',
    assert(t) {
      const line = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(line).toBe('after');
    },
  },

  {
    name: 'CSI ? 2026 h (synchronized output mode begin) is silently accepted',
    bytes: '\x1b[?2026hcontent\x1b[?2026l\r\n',
    assert(t) {
      const line = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(line).toBe('content');
    },
  },

  {
    name: 'spammed CRs do not desync cursor row',
    bytes: '\r'.repeat(50) + 'tail\r\n',
    assert(t) {
      expect(t.buffer.active.cursorY).toBe(1);
      const r0 = t.buffer.active.getLine(0)?.translateToString(true) ?? '';
      expect(r0).toBe('tail');
    },
  },
];

describe('xterm.js handles edge-case PTY output', () => {
  let term: Terminal | null = null;
  let host: HTMLDivElement | null = null;

  afterEach(() => {
    if (term) {
      term.dispose();
      term = null;
    }
    if (host) {
      host.remove();
      host = null;
    }
  });

  function newTerminal(): Terminal {
    host = document.createElement('div');
    host.style.width = '800px';
    host.style.height = '480px';
    document.body.appendChild(host);

    const t = new Terminal({
      cols: 80,
      rows: 24,
      scrollback: 1000,
      allowProposedApi: true,
    });
    t.open(host);
    return t;
  }

  it.each(FIXTURES)('$name', async ({ bytes, assert }) => {
    term = newTerminal();
    await new Promise<void>((resolve) => {
      term!.write(bytes, () => resolve());
    });
    assert(term);
  });

  it('writes a 200KB log stream without throwing', async () => {
    term = newTerminal();
    const chunk = 'line ' + 'x'.repeat(75) + '\r\n';
    // 2500 chunks * ~80 bytes ≈ 200KB.
    const total = 2500;
    await new Promise<void>((resolve) => {
      let remaining = total;
      const drain = () => {
        const done = () => {
          remaining--;
          if (remaining === 0) {
            resolve();
          } else if (remaining % 100 === 0) {
            // Yield occasionally so the test isn't a single
            // long-running microtask the runner can't interrupt.
            setTimeout(drain, 0);
          } else {
            drain();
          }
        };
        term!.write(chunk, done);
      };
      drain();
    });
    // The terminal has scrollback=1000, so the buffer's total length
    // should be capped at rows + scrollback.
    expect(term.buffer.active.length).toBeLessThanOrEqual(1000 + term.rows);
  });
});
