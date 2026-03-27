import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Store } from '@tauri-apps/plugin-store';

beforeEach(() => {
  vi.resetModules();
});

async function importModule() {
  return await import('./persistenceService');
}

describe('Input Stats Persistence', () => {
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

  describe('loadInputStats', () => {
    it('should return empty array when no stats are stored', async () => {
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual([]);
      expect(mockStore.reload).toHaveBeenCalled();
      expect(mockStore.get).toHaveBeenCalledWith('inputStats');
    });

    it('should return stored stats', async () => {
      const stats = [
        {
          text: 'hello',
          rawText: 'Hello',
          count: 3,
          lastUsed: 1000,
          firstSeen: 500,
          dismissedAt: null,
        },
      ];
      mockStore.get.mockResolvedValue(stats);
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual(stats);
    });

    it('should return empty array on error', async () => {
      mockStore.get.mockRejectedValue(new Error('fail'));
      const { loadInputStats } = await importModule();
      const result = await loadInputStats();
      expect(result).toEqual([]);
    });
  });

  describe('saveInputStats', () => {
    it('should save stats to store', async () => {
      const stats = [
        {
          text: 'hello',
          rawText: 'Hello',
          count: 3,
          lastUsed: 1000,
          firstSeen: 500,
          dismissedAt: null,
        },
      ];
      const { saveInputStats } = await importModule();
      await saveInputStats(stats);
      expect(mockStore.set).toHaveBeenCalledWith('inputStats', stats);
      expect(mockStore.save).toHaveBeenCalled();
    });
  });
});
