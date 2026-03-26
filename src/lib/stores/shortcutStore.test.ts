import { describe, expect, it } from 'vitest';
import { BUILTIN_SHORTCUTS, createShortcutStore, isAiProcess } from './shortcutStore.svelte';

describe('BUILTIN_SHORTCUTS', () => {
  it('has 3 items', () => {
    expect(BUILTIN_SHORTCUTS).toHaveLength(3);
  });

  it('has correct ids', () => {
    expect(BUILTIN_SHORTCUTS.map((s) => s.id)).toEqual([
      'builtin-ok',
      'builtin-continue',
      'builtin-lgtm',
    ]);
  });

  it('has correct labels', () => {
    expect(BUILTIN_SHORTCUTS.map((s) => s.label)).toEqual(['OK', 'Continue', 'LGTM']);
  });

  it('has correct text', () => {
    expect(BUILTIN_SHORTCUTS.map((s) => s.text)).toEqual(['OK', 'continue', 'LGTM']);
  });

  it('all have builtin: true', () => {
    expect(BUILTIN_SHORTCUTS.every((s) => s.builtin)).toBe(true);
  });
});

describe('createShortcutStore', () => {
  it('initializes with builtins only', () => {
    const store = createShortcutStore();
    expect(store.allShortcuts()).toEqual(BUILTIN_SHORTCUTS);
    expect(store.customShortcuts()).toEqual([]);
  });

  it('adds a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');

    const custom = store.customShortcuts();
    expect(custom).toHaveLength(1);
    expect(custom[0].label).toBe('Yes');
    expect(custom[0].text).toBe('yes');
    expect(custom[0].builtin).toBe(false);
    expect(custom[0].id).toMatch(/^custom-/);
  });

  it('allShortcuts includes builtins and custom', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');

    const all = store.allShortcuts();
    expect(all).toHaveLength(4);
    expect(all.slice(0, 3)).toEqual(BUILTIN_SHORTCUTS);
    expect(all[3].label).toBe('Yes');
  });

  it('updates a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');
    const id = store.customShortcuts()[0].id;

    store.updateShortcut(id, 'No', 'no');

    const custom = store.customShortcuts();
    expect(custom).toHaveLength(1);
    expect(custom[0].label).toBe('No');
    expect(custom[0].text).toBe('no');
    expect(custom[0].id).toBe(id);
  });

  it('cannot update a builtin shortcut', () => {
    const store = createShortcutStore();
    store.updateShortcut('builtin-ok', 'Changed', 'changed');

    const all = store.allShortcuts();
    const ok = all.find((s) => s.id === 'builtin-ok');
    expect(ok?.label).toBe('OK');
    expect(ok?.text).toBe('OK');
  });

  it('removes a custom shortcut', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');
    store.addShortcut('No', 'no');
    const id = store.customShortcuts()[0].id;

    store.removeShortcut(id);

    expect(store.customShortcuts()).toHaveLength(1);
    expect(store.customShortcuts()[0].label).toBe('No');
  });

  it('cannot remove a builtin shortcut', () => {
    const store = createShortcutStore();
    store.removeShortcut('builtin-ok');

    expect(store.allShortcuts()).toHaveLength(3);
  });

  it('setCustomShortcuts replaces all custom shortcuts', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');

    store.setCustomShortcuts([
      { id: 'custom-1', label: 'A', text: 'a', builtin: false },
      { id: 'custom-2', label: 'B', text: 'b', builtin: false },
    ]);

    expect(store.customShortcuts()).toHaveLength(2);
    expect(store.customShortcuts()[0].label).toBe('A');
    expect(store.customShortcuts()[1].label).toBe('B');
  });

  it('returns copies to prevent external mutation', () => {
    const store = createShortcutStore();
    store.addShortcut('Yes', 'yes');

    const all1 = store.allShortcuts();
    all1.push({ id: 'fake', label: 'Fake', text: 'fake', builtin: false });

    expect(store.allShortcuts()).toHaveLength(4); // 3 builtin + 1 custom
  });
});

describe('isAiProcess', () => {
  it('returns true for "claude"', () => {
    expect(isAiProcess('claude')).toBe(true);
  });

  it('returns true for "codex"', () => {
    expect(isAiProcess('codex')).toBe(true);
  });

  it('is case-insensitive', () => {
    expect(isAiProcess('Claude')).toBe(true);
    expect(isAiProcess('CLAUDE')).toBe(true);
    expect(isAiProcess('Codex')).toBe(true);
    expect(isAiProcess('CODEX')).toBe(true);
  });

  it('returns false for "zsh"', () => {
    expect(isAiProcess('zsh')).toBe(false);
  });

  it('returns false for "Terminal"', () => {
    expect(isAiProcess('Terminal')).toBe(false);
  });

  it('returns false for empty string', () => {
    expect(isAiProcess('')).toBe(false);
  });
});
