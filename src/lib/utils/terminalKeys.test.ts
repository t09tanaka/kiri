import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  TERMINAL_SEQUENCES,
  MAC_TERMINAL_KEYBINDINGS,
  matchesKeybinding,
  getTerminalSequence,
  isMacOS,
  type KeyBinding,
} from './terminalKeys';

describe('terminalKeys', () => {
  describe('TERMINAL_SEQUENCES', () => {
    it('should have correct escape sequence for WORD_BACKWARD (ESC b)', () => {
      expect(TERMINAL_SEQUENCES.WORD_BACKWARD).toBe('\x1bb');
    });

    it('should have correct escape sequence for WORD_FORWARD (ESC f)', () => {
      expect(TERMINAL_SEQUENCES.WORD_FORWARD).toBe('\x1bf');
    });

    it('should have correct escape sequence for LINE_START (Ctrl+A)', () => {
      expect(TERMINAL_SEQUENCES.LINE_START).toBe('\x01');
    });

    it('should have correct escape sequence for LINE_END (Ctrl+E)', () => {
      expect(TERMINAL_SEQUENCES.LINE_END).toBe('\x05');
    });
  });

  describe('MAC_TERMINAL_KEYBINDINGS', () => {
    it('should have 4 key bindings', () => {
      expect(MAC_TERMINAL_KEYBINDINGS).toHaveLength(4);
    });

    it('should map Option+Left to WORD_BACKWARD', () => {
      const binding = MAC_TERMINAL_KEYBINDINGS.find(
        (b) => b.key === 'ArrowLeft' && b.altKey === true
      );
      expect(binding).toBeDefined();
      expect(binding?.sequence).toBe(TERMINAL_SEQUENCES.WORD_BACKWARD);
    });

    it('should map Option+Right to WORD_FORWARD', () => {
      const binding = MAC_TERMINAL_KEYBINDINGS.find(
        (b) => b.key === 'ArrowRight' && b.altKey === true
      );
      expect(binding).toBeDefined();
      expect(binding?.sequence).toBe(TERMINAL_SEQUENCES.WORD_FORWARD);
    });

    it('should map Cmd+Left to LINE_START', () => {
      const binding = MAC_TERMINAL_KEYBINDINGS.find(
        (b) => b.key === 'ArrowLeft' && b.metaKey === true
      );
      expect(binding).toBeDefined();
      expect(binding?.sequence).toBe(TERMINAL_SEQUENCES.LINE_START);
    });

    it('should map Cmd+Right to LINE_END', () => {
      const binding = MAC_TERMINAL_KEYBINDINGS.find(
        (b) => b.key === 'ArrowRight' && b.metaKey === true
      );
      expect(binding).toBeDefined();
      expect(binding?.sequence).toBe(TERMINAL_SEQUENCES.LINE_END);
    });
  });

  describe('matchesKeybinding', () => {
    const createKeyboardEvent = (options: Partial<KeyboardEvent>): KeyboardEvent => {
      return {
        key: '',
        altKey: false,
        metaKey: false,
        ctrlKey: false,
        shiftKey: false,
        ...options,
      } as KeyboardEvent;
    };

    it('should match when key and altKey match', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', altKey: true, sequence: '\x1bb' };
      const event = createKeyboardEvent({ key: 'ArrowLeft', altKey: true });
      expect(matchesKeybinding(event, binding)).toBe(true);
    });

    it('should match when key and metaKey match', () => {
      const binding: KeyBinding = { key: 'ArrowRight', metaKey: true, sequence: '\x05' };
      const event = createKeyboardEvent({ key: 'ArrowRight', metaKey: true });
      expect(matchesKeybinding(event, binding)).toBe(true);
    });

    it('should not match when key differs', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', altKey: true, sequence: '\x1bb' };
      const event = createKeyboardEvent({ key: 'ArrowRight', altKey: true });
      expect(matchesKeybinding(event, binding)).toBe(false);
    });

    it('should not match when altKey expected but not pressed', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', altKey: true, sequence: '\x1bb' };
      const event = createKeyboardEvent({ key: 'ArrowLeft', altKey: false });
      expect(matchesKeybinding(event, binding)).toBe(false);
    });

    it('should not match when altKey pressed but not expected', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', sequence: '\x1bb' };
      const event = createKeyboardEvent({ key: 'ArrowLeft', altKey: true });
      expect(matchesKeybinding(event, binding)).toBe(false);
    });

    it('should not match when metaKey expected but not pressed', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', metaKey: true, sequence: '\x01' };
      const event = createKeyboardEvent({ key: 'ArrowLeft', metaKey: false });
      expect(matchesKeybinding(event, binding)).toBe(false);
    });

    it('should not match when metaKey pressed but not expected', () => {
      const binding: KeyBinding = { key: 'ArrowLeft', sequence: '\x1bb' };
      const event = createKeyboardEvent({ key: 'ArrowLeft', metaKey: true });
      expect(matchesKeybinding(event, binding)).toBe(false);
    });

    it('should match binding without modifier keys', () => {
      const binding: KeyBinding = { key: 'Enter', sequence: '\n' };
      const event = createKeyboardEvent({ key: 'Enter' });
      expect(matchesKeybinding(event, binding)).toBe(true);
    });
  });

  describe('getTerminalSequence', () => {
    const createKeyboardEvent = (options: Partial<KeyboardEvent>): KeyboardEvent => {
      return {
        key: '',
        altKey: false,
        metaKey: false,
        ctrlKey: false,
        shiftKey: false,
        ...options,
      } as KeyboardEvent;
    };

    it('should return WORD_BACKWARD for Option+Left', () => {
      const event = createKeyboardEvent({ key: 'ArrowLeft', altKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.WORD_BACKWARD);
    });

    it('should return WORD_FORWARD for Option+Right', () => {
      const event = createKeyboardEvent({ key: 'ArrowRight', altKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.WORD_FORWARD);
    });

    it('should return LINE_START for Cmd+Left', () => {
      const event = createKeyboardEvent({ key: 'ArrowLeft', metaKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.LINE_START);
    });

    it('should return LINE_END for Cmd+Right', () => {
      const event = createKeyboardEvent({ key: 'ArrowRight', metaKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.LINE_END);
    });

    it('should return null for plain ArrowLeft', () => {
      const event = createKeyboardEvent({ key: 'ArrowLeft' });
      expect(getTerminalSequence(event)).toBeNull();
    });

    it('should return null for plain ArrowRight', () => {
      const event = createKeyboardEvent({ key: 'ArrowRight' });
      expect(getTerminalSequence(event)).toBeNull();
    });

    it('should return null for unrelated key', () => {
      const event = createKeyboardEvent({ key: 'a', altKey: true });
      expect(getTerminalSequence(event)).toBeNull();
    });

    it('should return null for Ctrl+Arrow (not Option/Cmd)', () => {
      const event = createKeyboardEvent({ key: 'ArrowLeft', ctrlKey: true });
      expect(getTerminalSequence(event)).toBeNull();
    });
  });

  describe('isMacOS', () => {
    beforeEach(() => {
      // Reset navigator mock before each test
      vi.stubGlobal('navigator', { platform: '' });
    });

    afterEach(() => {
      vi.unstubAllGlobals();
    });

    it('should return true for MacIntel platform', () => {
      vi.stubGlobal('navigator', { platform: 'MacIntel' });
      expect(isMacOS()).toBe(true);
    });

    it('should return true for MacPPC platform', () => {
      vi.stubGlobal('navigator', { platform: 'MacPPC' });
      expect(isMacOS()).toBe(true);
    });

    it('should return true for iPhone platform', () => {
      vi.stubGlobal('navigator', { platform: 'iPhone' });
      expect(isMacOS()).toBe(true);
    });

    it('should return true for iPad platform', () => {
      vi.stubGlobal('navigator', { platform: 'iPad' });
      expect(isMacOS()).toBe(true);
    });

    it('should return true for iPod platform', () => {
      vi.stubGlobal('navigator', { platform: 'iPod' });
      expect(isMacOS()).toBe(true);
    });

    it('should return false for Win32 platform', () => {
      vi.stubGlobal('navigator', { platform: 'Win32' });
      expect(isMacOS()).toBe(false);
    });

    it('should return false for Linux platform', () => {
      vi.stubGlobal('navigator', { platform: 'Linux x86_64' });
      expect(isMacOS()).toBe(false);
    });

    it('should return false when navigator is undefined', () => {
      // @ts-expect-error - Testing undefined navigator
      vi.stubGlobal('navigator', undefined);
      expect(isMacOS()).toBe(false);
    });
  });
});
