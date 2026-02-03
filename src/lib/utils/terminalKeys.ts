/**
 * Terminal keyboard navigation utilities for macOS VSCode-compatible behavior
 *
 * Implements word and line navigation shortcuts that match VSCode's terminal:
 * - Option + Left/Right: Move by word (using readline ESC b/f sequences)
 * - Cmd + Left/Right: Move to line start/end (using Ctrl+A/E)
 */

/**
 * Terminal escape sequences for cursor navigation
 */
export const TERMINAL_SEQUENCES = {
  /** ESC b - Move backward one word */
  WORD_BACKWARD: '\x1bb',
  /** ESC f - Move forward one word */
  WORD_FORWARD: '\x1bf',
  /** Ctrl+A - Move to line start */
  LINE_START: '\x01',
  /** Ctrl+E - Move to line end */
  LINE_END: '\x05',
} as const;

/**
 * Key binding definition for terminal navigation
 */
export interface KeyBinding {
  /** The key identifier (e.g., 'ArrowLeft', 'ArrowRight') */
  key: string;
  /** Whether Alt/Option key must be pressed */
  altKey?: boolean;
  /** Whether Meta/Cmd key must be pressed */
  metaKey?: boolean;
  /** The escape sequence to send to the terminal */
  sequence: string;
}

/**
 * macOS terminal key bindings matching VSCode behavior
 */
export const MAC_TERMINAL_KEYBINDINGS: KeyBinding[] = [
  // Option + Left: Move backward one word
  { key: 'ArrowLeft', altKey: true, sequence: TERMINAL_SEQUENCES.WORD_BACKWARD },
  // Option + Right: Move forward one word
  { key: 'ArrowRight', altKey: true, sequence: TERMINAL_SEQUENCES.WORD_FORWARD },
  // Cmd + Left: Move to line start
  { key: 'ArrowLeft', metaKey: true, sequence: TERMINAL_SEQUENCES.LINE_START },
  // Cmd + Right: Move to line end
  { key: 'ArrowRight', metaKey: true, sequence: TERMINAL_SEQUENCES.LINE_END },
];

/**
 * Check if a keyboard event matches a key binding
 */
export function matchesKeybinding(event: KeyboardEvent, binding: KeyBinding): boolean {
  // Key must match
  if (event.key !== binding.key) {
    return false;
  }

  // Alt/Option key check
  if (binding.altKey && !event.altKey) {
    return false;
  }
  if (!binding.altKey && event.altKey) {
    return false;
  }

  // Meta/Cmd key check
  if (binding.metaKey && !event.metaKey) {
    return false;
  }
  if (!binding.metaKey && event.metaKey) {
    return false;
  }

  return true;
}

/**
 * Get the terminal sequence for a keyboard event (macOS only)
 *
 * @param event - The keyboard event to check
 * @returns The terminal escape sequence if the event matches a binding, null otherwise
 */
export function getTerminalSequence(event: KeyboardEvent): string | null {
  for (const binding of MAC_TERMINAL_KEYBINDINGS) {
    if (matchesKeybinding(event, binding)) {
      return binding.sequence;
    }
  }
  return null;
}

/**
 * Check if the current platform is macOS
 */
export function isMacOS(): boolean {
  // Use navigator.platform for reliable detection
  // navigator.userAgentData is newer but not universally supported
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform);
}
