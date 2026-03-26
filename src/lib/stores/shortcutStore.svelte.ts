export type ShortcutType = 'reply' | 'command';

export interface TerminalShortcut {
  id: string;
  label: string;
  text: string;
  builtin: boolean;
  type: ShortcutType;
}

export const BUILTIN_SHORTCUTS: TerminalShortcut[] = [
  { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true, type: 'reply' },
  { id: 'builtin-continue', label: 'Continue', text: 'continue', builtin: true, type: 'reply' },
  { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true, type: 'reply' },
];

let nextId = 1;
function generateId(): string {
  return `custom-${Date.now()}-${nextId++}`;
}

/**
 * Creates a non-reactive shortcut store for unit testing and pure logic.
 */
export function createShortcutStore() {
  let customShortcuts: TerminalShortcut[] = [];

  return {
    allShortcuts(): TerminalShortcut[] {
      return [...BUILTIN_SHORTCUTS, ...customShortcuts];
    },

    customShortcuts(): TerminalShortcut[] {
      return [...customShortcuts];
    },

    addShortcut(label: string, text: string, type: ShortcutType = 'reply'): void {
      customShortcuts = [
        ...customShortcuts,
        { id: generateId(), label, text, builtin: false, type },
      ];
    },

    updateShortcut(id: string, label: string, text: string): void {
      if (id.startsWith('builtin-')) return;
      customShortcuts = customShortcuts.map((s) => (s.id === id ? { ...s, label, text } : s));
    },

    removeShortcut(id: string): void {
      if (id.startsWith('builtin-')) return;
      customShortcuts = customShortcuts.filter((s) => s.id !== id);
    },

    setCustomShortcuts(shortcuts: TerminalShortcut[]): void {
      customShortcuts = [...shortcuts];
    },
  };
}

const AI_PROCESS_NAMES = ['claude', 'codex'];

/**
 * Determines whether a process name corresponds to an AI assistant.
 */
export function isAiProcess(processName: string): boolean {
  return AI_PROCESS_NAMES.includes(processName.toLowerCase());
}

/**
 * Reactive Svelte 5 state class for use in components.
 */
class ShortcutState {
  customShortcuts = $state<TerminalShortcut[]>([]);

  get allShortcuts(): TerminalShortcut[] {
    return [...BUILTIN_SHORTCUTS, ...this.customShortcuts];
  }

  addShortcut(label: string, text: string, type: ShortcutType = 'reply'): void {
    this.customShortcuts = [
      ...this.customShortcuts,
      { id: generateId(), label, text, builtin: false, type },
    ];
  }

  updateShortcut(id: string, label: string, text: string): void {
    if (id.startsWith('builtin-')) return;
    this.customShortcuts = this.customShortcuts.map((s) =>
      s.id === id ? { ...s, label, text } : s
    );
  }

  removeShortcut(id: string): void {
    if (id.startsWith('builtin-')) return;
    this.customShortcuts = this.customShortcuts.filter((s) => s.id !== id);
  }

  setCustomShortcuts(shortcuts: TerminalShortcut[]): void {
    this.customShortcuts = [...shortcuts];
  }
}

export const shortcutState = new ShortcutState();
