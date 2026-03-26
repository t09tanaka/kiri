import { describe, it, expect } from 'vitest';
import { parseDiff, getAllChangedLines } from './diffParser';

describe('parseDiff', () => {
  describe('empty input', () => {
    it('should return empty arrays for empty string', () => {
      const result = parseDiff('');
      expect(result.addedLines).toEqual([]);
      expect(result.modifiedLines).toEqual([]);
      expect(result.deletedAtLines).toEqual([]);
    });

    it('should return empty arrays for undefined-like input', () => {
      const result = parseDiff('');
      expect(result.addedLines).toEqual([]);
    });
  });

  describe('added lines', () => {
    it('should parse single added line', () => {
      const diff = `@@ -1,0 +1,1 @@
+ new line`;
      const result = parseDiff(diff);
      expect(result.addedLines).toEqual([1]);
      expect(result.modifiedLines).toEqual([]);
      expect(result.deletedAtLines).toEqual([]);
    });

    it('should parse multiple added lines', () => {
      const diff = `@@ -1,0 +1,3 @@
+ line 1
+ line 2
+ line 3`;
      const result = parseDiff(diff);
      expect(result.addedLines).toEqual([1, 2, 3]);
    });

    it('should parse added lines with context', () => {
      const diff = `@@ -1,2 +1,3 @@
  context before
+ new line
  context after`;
      const result = parseDiff(diff);
      expect(result.addedLines).toEqual([2]);
    });
  });

  describe('deleted lines', () => {
    it('should parse single deleted line', () => {
      const diff = `@@ -1,1 +1,0 @@
- removed line`;
      const result = parseDiff(diff);
      expect(result.deletedAtLines).toEqual([1]);
      expect(result.addedLines).toEqual([]);
    });

    it('should parse deleted line with context', () => {
      const diff = `@@ -1,3 +1,2 @@
  context before
- removed line
  context after`;
      const result = parseDiff(diff);
      expect(result.deletedAtLines).toEqual([2]);
    });

    it('should handle multiple consecutive deletions as single deletion marker', () => {
      const diff = `@@ -1,4 +1,2 @@
  context
- deleted 1
- deleted 2
  more context`;
      const result = parseDiff(diff);
      // Both deletions occur at the same position (after line 1)
      expect(result.deletedAtLines).toEqual([2]);
    });
  });

  describe('modified lines (deletion followed by addition)', () => {
    it('should detect single modified line', () => {
      const diff = `@@ -1,3 +1,3 @@
  context before
- old line
+ new line
  context after`;
      const result = parseDiff(diff);
      expect(result.modifiedLines).toEqual([2]);
      expect(result.addedLines).toEqual([]);
      expect(result.deletedAtLines).toEqual([]);
    });

    it('should detect multiple modified lines', () => {
      const diff = `@@ -1,4 +1,4 @@
  context
- old 1
- old 2
+ new 1
+ new 2
  end`;
      const result = parseDiff(diff);
      expect(result.modifiedLines).toEqual([2, 3]);
    });

    it('should handle mixed modifications and additions', () => {
      const diff = `@@ -1,3 +1,4 @@
  context
- old line
+ new line
+ extra line
  end`;
      const result = parseDiff(diff);
      expect(result.modifiedLines).toEqual([2]);
      expect(result.addedLines).toEqual([3]);
    });
  });

  describe('multiple hunks', () => {
    it('should parse multiple hunks correctly', () => {
      const diff = `@@ -1,2 +1,3 @@
  line 1
+ added in first hunk
  line 2
@@ -10,2 +11,3 @@
  line 10
+ added in second hunk
  line 11`;
      const result = parseDiff(diff);
      expect(result.addedLines).toEqual([2, 12]);
    });

    it('should reset pending deletions on new hunk', () => {
      const diff = `@@ -1,2 +1,1 @@
  context
- deleted
@@ -10,1 +10,2 @@
  another context
+ added`;
      const result = parseDiff(diff);
      // The deletion from first hunk should be marked
      expect(result.deletedAtLines).toEqual([2]);
      expect(result.addedLines).toEqual([11]);
    });
  });

  describe('untracked files (all additions)', () => {
    it('should parse untracked file with no hunk header', () => {
      // Untracked files are returned with all lines prefixed with "+ "
      // but without hunk headers
      const diff = `+ line 1
+ line 2
+ line 3`;
      const result = parseDiff(diff);
      // Without hunk header, currentLineNumber starts at 0
      expect(result.addedLines).toEqual([1, 2, 3]);
    });
  });

  describe('edge cases', () => {
    it('should handle deletion at end of file', () => {
      const diff = `@@ -1,3 +1,2 @@
  line 1
  line 2
- deleted at end`;
      const result = parseDiff(diff);
      expect(result.deletedAtLines).toEqual([3]);
    });

    it('should handle hunk header with single line count', () => {
      // Format: @@ -1 +1,2 @@ (no comma means count of 1)
      const diff = `@@ -1 +1,2 @@
  existing
+ new line`;
      const result = parseDiff(diff);
      expect(result.addedLines).toEqual([2]);
    });

    it('should handle malformed hunk header that does not match regex', () => {
      const diff = `@@ malformed header @@
+ added line`;
      const result = parseDiff(diff);
      // Without matching hunk header, currentLineNumber stays at 0
      expect(result.addedLines).toEqual([1]);
    });

    it('should ignore lines that do not match any known prefix', () => {
      const diff = `@@ -1,2 +1,3 @@
  context
this line has no recognized prefix
+ added line`;
      const result = parseDiff(diff);
      // Unrecognized line is skipped (no line number increment), added line is 2
      expect(result.addedLines).toEqual([2]);
    });
  });
});

describe('getAllChangedLines', () => {
  it('should combine all changed lines into a set', () => {
    const parsed = {
      addedLines: [1, 3],
      modifiedLines: [5],
      deletedAtLines: [7],
    };
    const result = getAllChangedLines(parsed);
    expect(result).toEqual(new Set([1, 3, 5, 7]));
  });

  it('should return empty set for no changes', () => {
    const parsed = {
      addedLines: [],
      modifiedLines: [],
      deletedAtLines: [],
    };
    const result = getAllChangedLines(parsed);
    expect(result.size).toBe(0);
  });
});
