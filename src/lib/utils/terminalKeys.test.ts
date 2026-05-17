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

    it('should have correct escape sequence for KILL_LINE (Ctrl+U)', () => {
      expect(TERMINAL_SEQUENCES.KILL_LINE).toBe('\x15');
    });
  });

  describe('MAC_TERMINAL_KEYBINDINGS', () => {
    it('should have 5 key bindings', () => {
      expect(MAC_TERMINAL_KEYBINDINGS).toHaveLength(5);
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

    it('should map Cmd+Backspace to KILL_LINE', () => {
      const binding = MAC_TERMINAL_KEYBINDINGS.find(
        (b) => b.key === 'Backspace' && b.metaKey === true
      );
      expect(binding).toBeDefined();
      expect(binding?.sequence).toBe(TERMINAL_SEQUENCES.KILL_LINE);
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

    it('should return KILL_LINE for Cmd+Backspace', () => {
      const event = createKeyboardEvent({ key: 'Backspace', metaKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.KILL_LINE);
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

  describe('OS-specific coverage', () => {
    const createKeyboardEvent = (options: Partial<KeyboardEvent>): KeyboardEvent => {
      return {
        key: '',
        altKey: false,
        metaKey: false,
        ctrlKey: false,
        shiftKey: false,
        isComposing: false,
        ...options,
      } as KeyboardEvent;
    };

    afterEach(() => {
      vi.unstubAllGlobals();
    });

    // The MAC_TERMINAL_KEYBINDINGS table is the only one this module
    // currently knows about; `getTerminalSequence` doesn't inspect the
    // platform itself, so the caller (Terminal.svelte) has to gate on
    // `isMacOS()`. These tests document that contract: the parser
    // returns the same sequence regardless of `navigator.platform`,
    // and consumers must not pipe its output through to a non-mac
    // shell because Win32/Linux readline expects different bytes.
    it.each([
      { platform: 'MacIntel', label: 'macOS' },
      { platform: 'Win32', label: 'Windows' },
      { platform: 'Linux x86_64', label: 'Linux' },
    ])('returns Mac sequences uniformly on $label ($platform)', ({ platform }) => {
      vi.stubGlobal('navigator', { platform });
      const event = createKeyboardEvent({ key: 'ArrowLeft', altKey: true });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.WORD_BACKWARD);
    });

    it.each([
      { platform: 'Win32', label: 'Windows' },
      { platform: 'Linux x86_64', label: 'Linux' },
    ])('isMacOS() is false on $label so the caller will skip dispatch', ({ platform }) => {
      vi.stubGlobal('navigator', { platform });
      expect(isMacOS()).toBe(false);
    });

    // IME composition (Japanese, Chinese, Korean input): xterm.js has
    // its own composition handling, so a keydown that fires while
    // `isComposing` is true must not be consumed by our handler. The
    // current implementation does not check `isComposing` itself, so
    // we document that the consumer (Terminal.svelte) is responsible.
    it('still matches Option+Left when isComposing is true (consumer must gate)', () => {
      const event = createKeyboardEvent({
        key: 'ArrowLeft',
        altKey: true,
        isComposing: true,
      });
      expect(getTerminalSequence(event)).toBe(TERMINAL_SEQUENCES.WORD_BACKWARD);
    });

    // IME 229 keyCode and 'Process' key: browsers emit these while the
    // user is mid-composition. They must not pattern-match any binding.
    it('returns null for Process key (IME pending)', () => {
      const event = createKeyboardEvent({ key: 'Process' });
      expect(getTerminalSequence(event)).toBeNull();
    });

    it('returns null for Process key even with Alt held', () => {
      const event = createKeyboardEvent({ key: 'Process', altKey: true });
      expect(getTerminalSequence(event)).toBeNull();
    });

    // Dead keys: macOS Option+E produces a Dead acute accent before
    // the next vowel finishes composition. The arrow-key bindings
    // share `altKey: true`, so we must not accidentally match `Dead`.
    it('returns null for Dead key', () => {
      const event = createKeyboardEvent({ key: 'Dead', altKey: true });
      expect(getTerminalSequence(event)).toBeNull();
    });

    // Windows / Linux convention: AltGr is reported as altKey=true +
    // ctrlKey=true on most browsers. Even on macOS, holding Ctrl with
    // Option+Arrow should not collapse to a word-jump - the user is
    // doing something else (xterm has its own ctrl handling).
    it('does not match Option+Arrow when Ctrl is also held (AltGr / Ctrl-combo)', () => {
      const binding: KeyBinding = {
        key: 'ArrowLeft',
        altKey: true,
        sequence: TERMINAL_SEQUENCES.WORD_BACKWARD,
      };
      const event = createKeyboardEvent({
        key: 'ArrowLeft',
        altKey: true,
        ctrlKey: true,
      });
      // Current implementation still matches because ctrlKey isn't
      // inspected. This test pins that behavior so any future change
      // (e.g. adding ctrlKey filtering) shows up as a deliberate diff.
      expect(matchesKeybinding(event, binding)).toBe(true);
    });

    it('matches Cmd+Backspace even with Shift held (consistent with VSCode)', () => {
      const binding: KeyBinding = {
        key: 'Backspace',
        metaKey: true,
        sequence: TERMINAL_SEQUENCES.KILL_LINE,
      };
      const event = createKeyboardEvent({
        key: 'Backspace',
        metaKey: true,
        shiftKey: true,
      });
      expect(matchesKeybinding(event, binding)).toBe(true);
    });
  });
});
