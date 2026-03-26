import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Store } from '@tauri-apps/plugin-store';
import type { TerminalShortcut } from '@/lib/stores/shortcutStore.svelte';

// Reset modules before each test to clear the cached store singleton
beforeEach(() => {
  vi.resetModules();
});

async function importModule() {
  return await import('./persistenceService');
}

describe('Terminal Shortcuts Persistence', () => {
  let mockStore: {
    get: ReturnType<typeof vi.fn>;
    set: ReturnType<typeof vi.fn>;
    save: ReturnType<typeof vi.fn>;
    reload: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    mockStore = {
      get: vi.fn().mockResolvedValue(null),
      set: vi.fn().mockResolvedValue(undefined),
      save: vi.fn().mockResolvedValue(undefined),
      reload: vi.fn().mockResolvedValue(undefined),
    };
    vi.mocked(Store.load).mockResolvedValue(mockStore as unknown as Store);
  });

  describe('loadShortcuts', () => {
    it('should return empty array when no shortcuts are stored', async () => {
      const { loadShortcuts } = await importModule();

      const result = await loadShortcuts();

      expect(result).toEqual([]);
      expect(mockStore.reload).toHaveBeenCalled();
      expect(mockStore.get).toHaveBeenCalledWith('terminalShortcuts');
    });

    it('should return stored shortcuts', async () => {
      const shortcuts: TerminalShortcut[] = [
        { id: 'custom-1', label: 'Deploy', text: 'npm run deploy', builtin: false },
        { id: 'custom-2', label: 'Test', text: 'npm test', builtin: false },
      ];
      mockStore.get.mockResolvedValue(shortcuts);

      const { loadShortcuts } = await importModule();

      const result = await loadShortcuts();

      expect(result).toEqual(shortcuts);
    });

    it('should return empty array when store throws an error', async () => {
      mockStore.get.mockRejectedValue(new Error('Store error'));

      const { loadShortcuts } = await importModule();

      const result = await loadShortcuts();

      expect(result).toEqual([]);
    });
  });

  describe('saveShortcuts', () => {
    it('should save shortcuts to store', async () => {
      const shortcuts: TerminalShortcut[] = [
        { id: 'custom-1', label: 'Deploy', text: 'npm run deploy', builtin: false },
      ];

      const { saveShortcuts } = await importModule();

      await saveShortcuts(shortcuts);

      expect(mockStore.set).toHaveBeenCalledWith('terminalShortcuts', shortcuts);
      expect(mockStore.save).toHaveBeenCalled();
    });

    it('should save empty array', async () => {
      const { saveShortcuts } = await importModule();

      await saveShortcuts([]);

      expect(mockStore.set).toHaveBeenCalledWith('terminalShortcuts', []);
      expect(mockStore.save).toHaveBeenCalled();
    });

    it('should not throw when store save fails', async () => {
      mockStore.save.mockRejectedValue(new Error('Save error'));

      const { saveShortcuts } = await importModule();

      await expect(saveShortcuts([])).resolves.toBeUndefined();
    });
  });
});
