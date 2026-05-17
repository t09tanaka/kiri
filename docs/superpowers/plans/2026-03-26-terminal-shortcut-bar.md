# Terminal Shortcut Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a shortcut button bar at the bottom of the terminal that appears when Claude or Codex is running, allowing quick phrase input with click (send + Enter) and Shift+click (input only).

**Architecture:** New shortcut store manages built-in + custom shortcuts with persistence via `kiri-settings.json`. A `TerminalShortcutBar` component renders pill-shaped buttons at the terminal bottom, with visibility driven by process name polling. A settings modal enables CRUD for custom shortcuts.

**Tech Stack:** Svelte 5, TypeScript, tauri-plugin-store (existing persistence layer)

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/lib/stores/shortcutStore.ts` | Shortcut state: built-in defaults, custom CRUD, persistence |
| Create | `src/lib/stores/shortcutStore.test.ts` | Unit tests for shortcut store |
| Create | `src/lib/components/terminal/TerminalShortcutBar.svelte` | Button bar UI with click/shift-click handlers |
| Create | `src/lib/components/terminal/TerminalShortcutBar.browser.test.ts` | Browser tests for the bar component |
| Create | `src/lib/components/terminal/TerminalShortcutSettings.svelte` | Settings modal for custom shortcut CRUD |
| Modify | `src/lib/services/persistenceService.ts` | Add shortcut persistence functions |
| Modify | `src/lib/components/terminal/Terminal.svelte` | Integrate shortcut bar into terminal layout |
| Modify | `src/lib/components/terminal/index.ts` | Export new components |

---

### Task 1: Shortcut Store — Types and Built-in Defaults

**Files:**
- Create: `src/lib/stores/shortcutStore.ts`
- Create: `src/lib/stores/shortcutStore.test.ts`

- [ ] **Step 1: Write failing tests for shortcut types and defaults**

```typescript
// src/lib/stores/shortcutStore.test.ts
import { describe, it, expect } from 'vitest';
import {
  BUILTIN_SHORTCUTS,
  type TerminalShortcut,
} from './shortcutStore';

describe('shortcutStore', () => {
  describe('BUILTIN_SHORTCUTS', () => {
    it('should have exactly 3 built-in shortcuts', () => {
      expect(BUILTIN_SHORTCUTS).toHaveLength(3);
    });

    it('should contain OK, Continue, and LGTM', () => {
      const labels = BUILTIN_SHORTCUTS.map((s) => s.label);
      expect(labels).toEqual(['OK', 'Continue', 'LGTM']);
    });

    it('should have correct text values', () => {
      const texts = BUILTIN_SHORTCUTS.map((s) => s.text);
      expect(texts).toEqual(['OK', 'continue', 'LGTM']);
    });

    it('should all be marked as builtin', () => {
      expect(BUILTIN_SHORTCUTS.every((s) => s.builtin)).toBe(true);
    });

    it('should have unique IDs prefixed with builtin-', () => {
      const ids = BUILTIN_SHORTCUTS.map((s) => s.id);
      expect(ids.every((id) => id.startsWith('builtin-'))).toBe(true);
      expect(new Set(ids).size).toBe(ids.length);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Implement types and built-in defaults**

```typescript
// src/lib/stores/shortcutStore.ts

/**
 * Terminal shortcut definition
 */
export interface TerminalShortcut {
  id: string;
  label: string;
  text: string;
  builtin: boolean;
}

/**
 * Built-in shortcuts (always present, not editable)
 */
export const BUILTIN_SHORTCUTS: TerminalShortcut[] = [
  { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true },
  { id: 'builtin-continue', label: 'Continue', text: 'continue', builtin: true },
  { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true },
];
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/shortcutStore.ts src/lib/stores/shortcutStore.test.ts
git commit -m "feat(shortcuts): add terminal shortcut types and built-in defaults"
```

---

### Task 2: Shortcut Store — CRUD Operations

**Files:**
- Modify: `src/lib/stores/shortcutStore.ts`
- Modify: `src/lib/stores/shortcutStore.test.ts`

- [ ] **Step 1: Write failing tests for CRUD**

Add to `src/lib/stores/shortcutStore.test.ts`:

```typescript
import {
  BUILTIN_SHORTCUTS,
  createShortcutStore,
  type TerminalShortcut,
} from './shortcutStore';

describe('createShortcutStore', () => {
  it('should initialize with only built-in shortcuts', () => {
    const store = createShortcutStore();
    expect(store.allShortcuts()).toEqual(BUILTIN_SHORTCUTS);
  });

  it('should add a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Fix', 'fix it');
    const all = store.allShortcuts();
    expect(all).toHaveLength(4);
    const custom = all[3];
    expect(custom.label).toBe('Fix');
    expect(custom.text).toBe('fix it');
    expect(custom.builtin).toBe(false);
  });

  it('should update a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Fix', 'fix it');
    const customId = store.allShortcuts()[3].id;
    store.updateShortcut(customId, 'Fix All', 'fix everything');
    const updated = store.allShortcuts().find((s) => s.id === customId);
    expect(updated?.label).toBe('Fix All');
    expect(updated?.text).toBe('fix everything');
  });

  it('should not update a built-in shortcut', () => {
    const store = createShortcutStore();
    store.updateShortcut('builtin-ok', 'Changed', 'changed');
    const ok = store.allShortcuts().find((s) => s.id === 'builtin-ok');
    expect(ok?.label).toBe('OK');
  });

  it('should remove a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Fix', 'fix it');
    const customId = store.allShortcuts()[3].id;
    store.removeShortcut(customId);
    expect(store.allShortcuts()).toHaveLength(3);
  });

  it('should not remove a built-in shortcut', () => {
    const store = createShortcutStore();
    store.removeShortcut('builtin-ok');
    expect(store.allShortcuts()).toHaveLength(3);
  });

  it('should return custom shortcuts only via customShortcuts()', () => {
    const store = createShortcutStore();
    store.addShortcut('Fix', 'fix it');
    store.addShortcut('Test', 'run tests');
    const customs = store.customShortcuts();
    expect(customs).toHaveLength(2);
    expect(customs.every((s) => !s.builtin)).toBe(true);
  });

  it('should set custom shortcuts from loaded data', () => {
    const store = createShortcutStore();
    const loaded: TerminalShortcut[] = [
      { id: 'custom-1', label: 'Retry', text: 'retry', builtin: false },
    ];
    store.setCustomShortcuts(loaded);
    expect(store.allShortcuts()).toHaveLength(4);
    expect(store.customShortcuts()).toEqual(loaded);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: FAIL — `createShortcutStore` not exported

- [ ] **Step 3: Implement CRUD operations**

Add to `src/lib/stores/shortcutStore.ts`:

```typescript
let nextId = 1;

function generateId(): string {
  return `custom-${Date.now()}-${nextId++}`;
}

/**
 * Create a shortcut store with CRUD operations.
 * Separated from module-level state for testability.
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

    addShortcut(label: string, text: string): void {
      customShortcuts = [
        ...customShortcuts,
        { id: generateId(), label, text, builtin: false },
      ];
    },

    updateShortcut(id: string, label: string, text: string): void {
      if (id.startsWith('builtin-')) return;
      customShortcuts = customShortcuts.map((s) =>
        s.id === id ? { ...s, label, text } : s
      );
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/shortcutStore.ts src/lib/stores/shortcutStore.test.ts
git commit -m "feat(shortcuts): add CRUD operations for shortcut store"
```

---

### Task 3: Shortcut Store — Reactive Svelte 5 State and AI Process Detection

**Files:**
- Modify: `src/lib/stores/shortcutStore.ts`
- Modify: `src/lib/stores/shortcutStore.test.ts`

- [ ] **Step 1: Write failing tests for AI process detection**

Add to `src/lib/stores/shortcutStore.test.ts`:

```typescript
import { isAiProcess } from './shortcutStore';

describe('isAiProcess', () => {
  it('should return true for "claude"', () => {
    expect(isAiProcess('claude')).toBe(true);
  });

  it('should return true for "codex"', () => {
    expect(isAiProcess('codex')).toBe(true);
  });

  it('should be case-insensitive', () => {
    expect(isAiProcess('Claude')).toBe(true);
    expect(isAiProcess('CODEX')).toBe(true);
  });

  it('should return false for "zsh"', () => {
    expect(isAiProcess('zsh')).toBe(false);
  });

  it('should return false for "Terminal"', () => {
    expect(isAiProcess('Terminal')).toBe(false);
  });

  it('should return false for empty string', () => {
    expect(isAiProcess('')).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: FAIL — `isAiProcess` not exported

- [ ] **Step 3: Implement AI process detection and reactive state**

Add to `src/lib/stores/shortcutStore.ts`:

```typescript
const AI_PROCESS_NAMES = ['claude', 'codex'];

/**
 * Check if a process name matches a known AI CLI tool
 */
export function isAiProcess(processName: string): boolean {
  return AI_PROCESS_NAMES.includes(processName.toLowerCase());
}

/**
 * Module-level reactive shortcut store for use in Svelte components.
 * Uses Svelte 5 $state runes for reactivity.
 */
class ShortcutState {
  customShortcuts = $state<TerminalShortcut[]>([]);

  get allShortcuts(): TerminalShortcut[] {
    return [...BUILTIN_SHORTCUTS, ...this.customShortcuts];
  }

  addShortcut(label: string, text: string): void {
    this.customShortcuts = [
      ...this.customShortcuts,
      { id: generateId(), label, text, builtin: false },
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/shortcutStore.ts src/lib/stores/shortcutStore.test.ts
git commit -m "feat(shortcuts): add AI process detection and reactive Svelte 5 state"
```

---

### Task 4: Persistence — Load/Save Shortcuts

**Files:**
- Modify: `src/lib/services/persistenceService.ts`
- Create test for persistence functions (add to `shortcutStore.test.ts`)

- [ ] **Step 1: Write failing tests for persistence**

Add to `src/lib/stores/shortcutStore.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock tauri-plugin-store
vi.mock('@tauri-apps/plugin-store', () => {
  const mockData = new Map<string, unknown>();
  return {
    Store: {
      load: vi.fn().mockResolvedValue({
        get: vi.fn((key: string) => Promise.resolve(mockData.get(key))),
        set: vi.fn((key: string, value: unknown) => {
          mockData.set(key, value);
          return Promise.resolve();
        }),
        save: vi.fn().mockResolvedValue(undefined),
        reload: vi.fn().mockResolvedValue(undefined),
      }),
    },
  };
});

import { loadShortcuts, saveShortcuts } from '../services/persistenceService';
import type { TerminalShortcut } from './shortcutStore';

describe('shortcut persistence', () => {
  it('should return empty array when no shortcuts saved', async () => {
    const shortcuts = await loadShortcuts();
    expect(shortcuts).toEqual([]);
  });

  it('should save and load custom shortcuts', async () => {
    const custom: TerminalShortcut[] = [
      { id: 'custom-1', label: 'Fix', text: 'fix it', builtin: false },
    ];
    await saveShortcuts(custom);
    const loaded = await loadShortcuts();
    expect(loaded).toEqual(custom);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: FAIL — `loadShortcuts` / `saveShortcuts` not exported from persistenceService

- [ ] **Step 3: Implement persistence functions**

Add to the end of `src/lib/services/persistenceService.ts`:

```typescript
// ============================================================================
// Terminal Shortcuts
// ============================================================================

import type { TerminalShortcut } from '@/lib/stores/shortcutStore';

/**
 * Load custom terminal shortcuts from settings
 */
export async function loadShortcuts(): Promise<TerminalShortcut[]> {
  try {
    const s = await getStore();
    await s.reload();
    const shortcuts = await s.get<TerminalShortcut[]>('terminalShortcuts');
    return shortcuts ?? [];
  } catch (error) {
    console.error('Failed to load shortcuts:', error);
    return [];
  }
}

/**
 * Save custom terminal shortcuts to settings
 */
export async function saveShortcuts(shortcuts: TerminalShortcut[]): Promise<void> {
  try {
    const s = await getStore();
    await s.set('terminalShortcuts', shortcuts);
    await s.save();
  } catch (error) {
    console.error('Failed to save shortcuts:', error);
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/shortcutStore.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/persistenceService.ts src/lib/stores/shortcutStore.test.ts
git commit -m "feat(shortcuts): add persistence for custom terminal shortcuts"
```

---

### Task 5: TerminalShortcutBar Component

**Files:**
- Create: `src/lib/components/terminal/TerminalShortcutBar.svelte`
- Create: `src/lib/components/terminal/TerminalShortcutBar.browser.test.ts`

- [ ] **Step 1: Write failing browser test**

```typescript
// src/lib/components/terminal/TerminalShortcutBar.browser.test.ts
import { render, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import TerminalShortcutBar from './TerminalShortcutBar.svelte';

describe('TerminalShortcutBar', () => {
  const defaultProps = {
    visible: true,
    shortcuts: [
      { id: 'builtin-ok', label: 'OK', text: 'OK', builtin: true },
      { id: 'builtin-continue', label: 'Continue', text: 'continue', builtin: true },
      { id: 'builtin-lgtm', label: 'LGTM', text: 'LGTM', builtin: true },
    ],
    onSend: vi.fn(),
    onSettingsClick: vi.fn(),
  };

  it('should render shortcut buttons when visible', () => {
    const { getByText } = render(TerminalShortcutBar, { props: defaultProps });
    expect(getByText('OK')).toBeTruthy();
    expect(getByText('Continue')).toBeTruthy();
    expect(getByText('LGTM')).toBeTruthy();
  });

  it('should not render when not visible', () => {
    const { queryByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, visible: false },
    });
    expect(queryByText('OK')).toBeNull();
  });

  it('should call onSend with text and withEnter=true on click', async () => {
    const onSend = vi.fn();
    const { getByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, onSend },
    });
    await fireEvent.click(getByText('OK'));
    expect(onSend).toHaveBeenCalledWith('OK', true);
  });

  it('should call onSend with text and withEnter=false on shift+click', async () => {
    const onSend = vi.fn();
    const { getByText } = render(TerminalShortcutBar, {
      props: { ...defaultProps, onSend },
    });
    await fireEvent.click(getByText('OK'), { shiftKey: true });
    expect(onSend).toHaveBeenCalledWith('OK', false);
  });

  it('should render settings button', () => {
    const { getByTitle } = render(TerminalShortcutBar, { props: defaultProps });
    expect(getByTitle('Shortcut Settings')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test:browser -- src/lib/components/terminal/TerminalShortcutBar.browser.test.ts`
Expected: FAIL — component not found

- [ ] **Step 3: Implement TerminalShortcutBar component**

```svelte
<!-- src/lib/components/terminal/TerminalShortcutBar.svelte -->
<script lang="ts">
  import type { TerminalShortcut } from '@/lib/stores/shortcutStore';

  interface Props {
    visible: boolean;
    shortcuts: TerminalShortcut[];
    onSend: (text: string, withEnter: boolean) => void;
    onSettingsClick: () => void;
  }

  let { visible, shortcuts, onSend, onSettingsClick }: Props = $props();

  function handleClick(event: MouseEvent, shortcut: TerminalShortcut) {
    const withEnter = !event.shiftKey;
    onSend(shortcut.text, withEnter);
  }
</script>

{#if visible}
  <div class="shortcut-bar">
    <div class="shortcut-buttons">
      {#each shortcuts as shortcut (shortcut.id)}
        <button
          class="shortcut-btn"
          onclick={(e) => handleClick(e, shortcut)}
          title="{shortcut.label} (Shift+click: input only)"
        >
          {shortcut.label}
        </button>
      {/each}
    </div>
    <button
      class="settings-btn"
      onclick={onSettingsClick}
      title="Shortcut Settings"
      aria-label="Shortcut Settings"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .shortcut-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-glass, rgba(13, 17, 23, 0.7));
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border-top: 1px solid rgba(125, 211, 252, 0.1);
    animation: slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    z-index: 10;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .shortcut-buttons {
    display: flex;
    gap: 6px;
    flex: 1;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .shortcut-buttons::-webkit-scrollbar {
    display: none;
  }

  .shortcut-btn {
    flex-shrink: 0;
    padding: 4px 12px;
    font-size: 11px;
    font-family: 'IBM Plex Mono', monospace;
    color: var(--text-secondary, #8b949e);
    background: rgba(125, 211, 252, 0.06);
    border: 1px solid rgba(125, 211, 252, 0.15);
    border-radius: 12px;
    cursor: pointer;
    transition: all 180ms ease;
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .shortcut-btn:hover {
    color: var(--accent-color, #7dd3fc);
    background: rgba(125, 211, 252, 0.12);
    border-color: rgba(125, 211, 252, 0.3);
    box-shadow: 0 0 8px rgba(125, 211, 252, 0.1);
  }

  .shortcut-btn:active {
    transform: scale(0.95);
  }

  .settings-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 4px);
    color: var(--text-muted, #484f58);
    cursor: pointer;
    transition: all 180ms ease;
  }

  .settings-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color, #7dd3fc);
  }
</style>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test:browser -- src/lib/components/terminal/TerminalShortcutBar.browser.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/TerminalShortcutBar.svelte src/lib/components/terminal/TerminalShortcutBar.browser.test.ts
git commit -m "feat(shortcuts): add TerminalShortcutBar component with Mist theme styling"
```

---

### Task 6: TerminalShortcutSettings Modal

**Files:**
- Create: `src/lib/components/terminal/TerminalShortcutSettings.svelte`

- [ ] **Step 1: Implement settings modal**

```svelte
<!-- src/lib/components/terminal/TerminalShortcutSettings.svelte -->
<script lang="ts">
  import type { TerminalShortcut } from '@/lib/stores/shortcutStore';

  interface Props {
    open: boolean;
    shortcuts: TerminalShortcut[];
    onClose: () => void;
    onAdd: (label: string, text: string) => void;
    onUpdate: (id: string, label: string, text: string) => void;
    onRemove: (id: string) => void;
  }

  let { open, shortcuts, onClose, onAdd, onUpdate, onRemove }: Props = $props();

  let newLabel = $state('');
  let newText = $state('');
  let editingId = $state<string | null>(null);
  let editLabel = $state('');
  let editText = $state('');

  function handleAdd() {
    const label = newLabel.trim();
    const text = newText.trim();
    if (!label || !text) return;
    onAdd(label, text);
    newLabel = '';
    newText = '';
  }

  function startEdit(shortcut: TerminalShortcut) {
    editingId = shortcut.id;
    editLabel = shortcut.label;
    editText = shortcut.text;
  }

  function saveEdit() {
    if (!editingId) return;
    const label = editLabel.trim();
    const text = editText.trim();
    if (!label || !text) return;
    onUpdate(editingId, label, text);
    editingId = null;
  }

  function cancelEdit() {
    editingId = null;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (editingId) {
        cancelEdit();
      } else {
        onClose();
      }
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={onClose} onkeydown={handleKeydown}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-content" onclick={(e) => e.stopPropagation()} onkeydown={handleKeydown}>
      <div class="modal-glow"></div>
      <div class="modal-header">
        <h3>Shortcut Settings</h3>
        <button class="close-btn" onclick={onClose} aria-label="Close">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div class="modal-body">
        <!-- Built-in shortcuts (read-only) -->
        <div class="section-label">Built-in</div>
        {#each shortcuts.filter((s) => s.builtin) as shortcut (shortcut.id)}
          <div class="shortcut-row readonly">
            <span class="shortcut-label">{shortcut.label}</span>
            <span class="shortcut-text">{shortcut.text}</span>
          </div>
        {/each}

        <!-- Custom shortcuts -->
        <div class="section-label">Custom</div>
        {#each shortcuts.filter((s) => !s.builtin) as shortcut (shortcut.id)}
          <div class="shortcut-row">
            {#if editingId === shortcut.id}
              <input
                class="edit-input"
                bind:value={editLabel}
                placeholder="Label"
                spellcheck="false"
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
              />
              <input
                class="edit-input"
                bind:value={editText}
                placeholder="Text to send"
                spellcheck="false"
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                onkeydown={(e) => e.key === 'Enter' && saveEdit()}
              />
              <button class="action-btn save" onclick={saveEdit} title="Save">✓</button>
              <button class="action-btn cancel" onclick={cancelEdit} title="Cancel">✕</button>
            {:else}
              <span class="shortcut-label">{shortcut.label}</span>
              <span class="shortcut-text">{shortcut.text}</span>
              <button class="action-btn edit" onclick={() => startEdit(shortcut)} title="Edit">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                </svg>
              </button>
              <button class="action-btn delete" onclick={() => onRemove(shortcut.id)} title="Delete">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            {/if}
          </div>
        {/each}

        {#if shortcuts.filter((s) => !s.builtin).length === 0}
          <div class="empty-hint">No custom shortcuts yet</div>
        {/if}

        <!-- Add new shortcut -->
        <div class="add-row">
          <input
            class="edit-input"
            bind:value={newLabel}
            placeholder="Label"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
          />
          <input
            class="edit-input"
            bind:value={newText}
            placeholder="Text to send"
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            onkeydown={(e) => e.key === 'Enter' && handleAdd()}
          />
          <button
            class="add-btn"
            onclick={handleAdd}
            disabled={!newLabel.trim() || !newText.trim()}
          >
            Add
          </button>
        </div>
      </div>

      <div class="modal-footer">
        <span class="footer-item">
          <kbd>Shift+Click</kbd>
          <span>input only</span>
        </span>
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal-content {
    position: relative;
    width: 420px;
    max-height: 80vh;
    background: var(--bg-secondary, #161b22);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: var(--radius-xl, 16px);
    overflow: hidden;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color, #7dd3fc), transparent);
    opacity: 0.6;
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start, rgba(125, 211, 252, 0.3)), var(--gradient-end, rgba(196, 181, 253, 0.3)));
    border-radius: calc(var(--radius-xl, 16px) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    z-index: -1;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 12px;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary, #e6edf3);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 4px);
    color: var(--text-muted, #484f58);
    cursor: pointer;
    transition: all 180ms ease;
  }

  .close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .modal-body {
    padding: 0 20px 16px;
    overflow-y: auto;
    max-height: 50vh;
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted, #484f58);
    margin: 12px 0 6px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm, 4px);
    transition: background 180ms ease;
  }

  .shortcut-row:hover {
    background: rgba(125, 211, 252, 0.04);
  }

  .shortcut-row.readonly {
    opacity: 0.5;
  }

  .shortcut-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary, #e6edf3);
    min-width: 80px;
  }

  .shortcut-text {
    flex: 1;
    font-size: 11px;
    font-family: 'IBM Plex Mono', monospace;
    color: var(--text-muted, #484f58);
  }

  .edit-input {
    flex: 1;
    padding: 4px 8px;
    font-size: 12px;
    font-family: 'IBM Plex Mono', monospace;
    color: var(--text-primary, #e6edf3);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: var(--radius-sm, 4px);
    outline: none;
    transition: border-color 180ms ease;
  }

  .edit-input:focus {
    border-color: var(--accent-color, #7dd3fc);
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 4px);
    color: var(--text-muted, #484f58);
    cursor: pointer;
    transition: all 180ms ease;
    font-size: 12px;
  }

  .action-btn.edit:hover,
  .action-btn.save:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color, #7dd3fc);
  }

  .action-btn.delete:hover,
  .action-btn.cancel:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .empty-hint {
    font-size: 11px;
    color: var(--text-muted, #484f58);
    padding: 8px;
    text-align: center;
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 8px;
    background: rgba(125, 211, 252, 0.03);
    border: 1px dashed rgba(125, 211, 252, 0.15);
    border-radius: var(--radius-sm, 4px);
  }

  .add-btn {
    flex-shrink: 0;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent-color, #7dd3fc);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: 12px;
    cursor: pointer;
    transition: all 180ms ease;
  }

  .add-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.2);
  }

  .add-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 20px;
    border-top: 1px solid rgba(125, 211, 252, 0.08);
    background: rgba(0, 0, 0, 0.2);
  }

  .footer-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-muted, #484f58);
  }

  .footer-item kbd {
    padding: 1px 5px;
    font-size: 10px;
    font-family: 'IBM Plex Mono', monospace;
    color: var(--text-secondary, #8b949e);
    background: rgba(125, 211, 252, 0.08);
    border: 1px solid rgba(125, 211, 252, 0.15);
    border-radius: 3px;
  }
</style>
```

- [ ] **Step 2: Verify it compiles**

Run: `npm run check`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/terminal/TerminalShortcutSettings.svelte
git commit -m "feat(shortcuts): add settings modal for custom shortcut CRUD"
```

---

### Task 7: Integrate Shortcut Bar into Terminal Component

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`
- Modify: `src/lib/components/terminal/index.ts`

- [ ] **Step 1: Add process name polling and shortcut bar to Terminal.svelte**

Add imports at the top of the `<script>` block (after existing imports):

```typescript
import TerminalShortcutBar from './TerminalShortcutBar.svelte';
import TerminalShortcutSettings from './TerminalShortcutSettings.svelte';
import { shortcutState, isAiProcess } from '@/lib/stores/shortcutStore';
import { loadShortcuts, saveShortcuts } from '@/lib/services/persistenceService';
```

Add state variables (near existing `let isFocused` and `let memoryDisplay`):

```typescript
let processName = $state('');
let showShortcutSettings = $state(false);
const isAiRunning = $derived(isAiProcess(processName));
```

Extend the existing memory polling interval to also update process name. In the `updateMemoryDisplay` function, add after the `memoryDisplay` assignment:

```typescript
processName = info.name;
```

Add handler functions (near existing handlers):

```typescript
function handleShortcutSend(text: string, withEnter: boolean) {
  if (terminalId === null) return;
  terminalService.writeTerminal(terminalId, withEnter ? text + '\r' : text);
  // Refocus terminal after clicking shortcut
  terminal?.focus();
}

async function handleShortcutAdd(label: string, text: string) {
  shortcutState.addShortcut(label, text);
  await saveShortcuts(shortcutState.customShortcuts);
}

async function handleShortcutUpdate(id: string, label: string, text: string) {
  shortcutState.updateShortcut(id, label, text);
  await saveShortcuts(shortcutState.customShortcuts);
}

async function handleShortcutRemove(id: string) {
  shortcutState.removeShortcut(id);
  await saveShortcuts(shortcutState.customShortcuts);
}
```

Add shortcut loading to `initTerminal` (at the end of the function, after the startup command block):

```typescript
// Load custom shortcuts
const customShortcuts = await loadShortcuts();
shortcutState.setCustomShortcuts(customShortcuts);
```

- [ ] **Step 2: Add bar to template**

Insert after the `terminal-padding` div and before the `terminal-glow` div (around line 936):

```svelte
  <TerminalShortcutBar
    visible={isAiRunning}
    shortcuts={shortcutState.allShortcuts}
    onSend={handleShortcutSend}
    onSettingsClick={() => (showShortcutSettings = true)}
  />
  <TerminalShortcutSettings
    open={showShortcutSettings}
    shortcuts={shortcutState.allShortcuts}
    onClose={() => (showShortcutSettings = false)}
    onAdd={handleShortcutAdd}
    onUpdate={handleShortcutUpdate}
    onRemove={handleShortcutRemove}
  />
```

- [ ] **Step 3: Update index.ts exports**

Add to `src/lib/components/terminal/index.ts`:

```typescript
export { default as TerminalShortcutBar } from './TerminalShortcutBar.svelte';
export { default as TerminalShortcutSettings } from './TerminalShortcutSettings.svelte';
```

- [ ] **Step 4: Run type check and tests**

Run: `npm run check && npm run test && npm run test:browser`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte src/lib/components/terminal/index.ts
git commit -m "feat(shortcuts): integrate shortcut bar into terminal with AI process detection"
```

---

### Task 8: Final Verification

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

Run: `npm run test && npm run test:browser && npm run test:rust`
Expected: All pass

- [ ] **Step 2: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: No errors

- [ ] **Step 3: Start app and verify visually**

Run: `npm run tauri dev`

Verify:
1. Open terminal, run `claude` — shortcut bar appears at bottom
2. Click "OK" — sends "OK" + Enter to terminal
3. Shift+click "OK" — sends "OK" without Enter
4. Click gear icon — settings modal opens
5. Add custom shortcut — appears in bar
6. Close and reopen app — custom shortcuts persist
7. Exit `claude` — shortcut bar disappears

- [ ] **Step 4: Commit all remaining changes (if any)**

```bash
git add -A
git commit -m "feat(shortcuts): terminal shortcut bar for AI process interaction"
```
