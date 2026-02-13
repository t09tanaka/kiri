import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

const mockStoreInstance = {
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
  delete: vi.fn(),
  reload: vi.fn(),
};

vi.mock('@tauri-apps/plugin-store', () => ({
  Store: {
    load: vi.fn().mockResolvedValue(mockStoreInstance),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/lib/services/windowService', () => ({
  windowService: {
    setGeometry: vi.fn().mockResolvedValue(undefined),
    setSizeAndCenter: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@/lib/services/eventService', () => ({
  eventService: {
    emit: vi.fn().mockResolvedValue(undefined),
  },
}));

describe('projectStore', () => {
  beforeEach(() => {
    vi.resetModules();
    mockStoreInstance.get.mockReset();
    mockStoreInstance.set.mockReset().mockResolvedValue(undefined);
    mockStoreInstance.save.mockReset().mockResolvedValue(undefined);
    mockStoreInstance.reload.mockReset().mockResolvedValue(undefined);
  });

  describe('MAX_RECENT_MENU_ITEMS', () => {
    it('should be exported with value 5', async () => {
      const { MAX_RECENT_MENU_ITEMS } = await import('./projectStore');
      expect(MAX_RECENT_MENU_ITEMS).toBe(5);
    });
  });

  describe('clearRecentProjects', () => {
    it('should clear all recent projects', async () => {
      mockStoreInstance.get.mockResolvedValue([
        { path: '/a', name: 'a', lastOpened: 1 },
        { path: '/b', name: 'b', lastOpened: 2 },
      ]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      expect(get(recentProjects).length).toBe(2);

      await projectStore.clearRecentProjects();

      // Wait for the fire-and-forget saveRecentProjects to complete
      await vi.waitFor(() => {
        expect(mockStoreInstance.save).toHaveBeenCalled();
      });

      expect(get(recentProjects).length).toBe(0);
      expect(mockStoreInstance.set).toHaveBeenCalledWith('recentProjects', []);
    });
  });
});
