import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { peekStore, isPeekOpen } from './peekStore';

describe('peekStore', () => {
  beforeEach(() => {
    peekStore.close();
  });

  describe('initial state', () => {
    it('should have correct initial values', () => {
      const state = get(peekStore);

      expect(state).toEqual({
        isOpen: false,
        filePath: null,
        lineNumber: undefined,
        columnNumber: undefined,
      });
    });
  });

  describe('open', () => {
    it('should open peek with file path', () => {
      peekStore.open('/path/to/file.ts');

      const state = get(peekStore);
      expect(state.isOpen).toBe(true);
      expect(state.filePath).toBe('/path/to/file.ts');
      expect(state.lineNumber).toBeUndefined();
      expect(state.columnNumber).toBeUndefined();
    });

    it('should open peek with file path and line number', () => {
      peekStore.open('/path/to/file.ts', 42);

      const state = get(peekStore);
      expect(state.isOpen).toBe(true);
      expect(state.filePath).toBe('/path/to/file.ts');
      expect(state.lineNumber).toBe(42);
      expect(state.columnNumber).toBeUndefined();
    });

    it('should open peek with file path, line number, and column number', () => {
      peekStore.open('/path/to/file.ts', 42, 10);

      const state = get(peekStore);
      expect(state.isOpen).toBe(true);
      expect(state.filePath).toBe('/path/to/file.ts');
      expect(state.lineNumber).toBe(42);
      expect(state.columnNumber).toBe(10);
    });

    it('should replace previous state when opening new file', () => {
      peekStore.open('/path/to/first.ts', 10);
      peekStore.open('/path/to/second.ts', 20, 5);

      const state = get(peekStore);
      expect(state.filePath).toBe('/path/to/second.ts');
      expect(state.lineNumber).toBe(20);
      expect(state.columnNumber).toBe(5);
    });
  });

  describe('close', () => {
    it('should close peek and reset state', () => {
      peekStore.open('/path/to/file.ts', 42, 10);
      peekStore.close();

      const state = get(peekStore);
      expect(state).toEqual({
        isOpen: false,
        filePath: null,
        lineNumber: undefined,
        columnNumber: undefined,
      });
    });
  });

  describe('subscribe', () => {
    it('should allow subscribing to store updates', () => {
      let lastValue: unknown = null;
      const unsubscribe = peekStore.subscribe((value) => {
        lastValue = value;
      });

      peekStore.open('/path/to/file.ts');

      expect(lastValue).toMatchObject({
        isOpen: true,
        filePath: '/path/to/file.ts',
      });

      unsubscribe();
    });
  });
});

describe('isPeekOpen', () => {
  beforeEach(() => {
    peekStore.close();
  });

  it('should be false when peek is closed', () => {
    expect(get(isPeekOpen)).toBe(false);
  });

  it('should be true when peek is open', () => {
    peekStore.open('/path/to/file.ts');

    expect(get(isPeekOpen)).toBe(true);
  });

  it('should become false when peek is closed', () => {
    peekStore.open('/path/to/file.ts');
    peekStore.close();

    expect(get(isPeekOpen)).toBe(false);
  });
});
