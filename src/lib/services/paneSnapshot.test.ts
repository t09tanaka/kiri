import { describe, expect, it } from 'vitest';
import { snapshotBuffer, type SnapshotBuffer } from './paneSnapshot';

/** Build a fake xterm buffer from an array of row strings. */
function fakeBuffer(rows: string[]): SnapshotBuffer {
  return {
    length: rows.length,
    getLine: (i: number) =>
      i >= 0 && i < rows.length ? { translateToString: () => rows[i] } : undefined,
  };
}

describe('snapshotBuffer', () => {
  it('returns the last N rows joined by newline', () => {
    const buf = fakeBuffer(['a', 'b', 'c', 'd']);
    expect(snapshotBuffer(buf, 2)).toBe('c\nd');
  });

  it('drops trailing blank rows so it ends at the last content line', () => {
    const buf = fakeBuffer(['prompt', 'status line', '', '   ', '']);
    expect(snapshotBuffer(buf, 10)).toBe('prompt\nstatus line');
  });

  it('preserves interior blank lines', () => {
    const buf = fakeBuffer(['a', '', 'b']);
    expect(snapshotBuffer(buf, 10)).toBe('a\n\nb');
  });

  it('caps at the available rows when fewer than requested', () => {
    const buf = fakeBuffer(['only']);
    expect(snapshotBuffer(buf, 40)).toBe('only');
  });

  it('returns at least one row even when lines <= 0', () => {
    const buf = fakeBuffer(['x', 'y']);
    expect(snapshotBuffer(buf, 0)).toBe('y');
  });

  it('returns empty string for an all-blank buffer', () => {
    const buf = fakeBuffer(['', '  ', '']);
    expect(snapshotBuffer(buf, 5)).toBe('');
  });
});
