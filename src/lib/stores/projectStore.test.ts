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

  describe('recent menu event emitting', () => {
    it('should emit update-recent-menu after openProject', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      // Clear any init calls
      const { eventService } = await import('@/lib/services/eventService');
      vi.mocked(eventService.emit).mockClear();

      await projectStore.openProject('/test/project');

      expect(eventService.emit).toHaveBeenCalledWith(
        'update-recent-menu',
        expect.arrayContaining([expect.objectContaining({ path: '/test/project' })])
      );
    });

    it('should emit update-recent-menu after clearRecentProjects', async () => {
      mockStoreInstance.get.mockResolvedValue([{ path: '/a', name: 'a', lastOpened: 1 }]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      const { eventService } = await import('@/lib/services/eventService');
      vi.mocked(eventService.emit).mockClear();

      await projectStore.clearRecentProjects();

      expect(eventService.emit).toHaveBeenCalledWith('update-recent-menu', []);
    });

    it('should emit at most MAX_RECENT_MENU_ITEMS items', async () => {
      const projects = Array.from({ length: 10 }, (_, i) => ({
        path: `/project-${i}`,
        name: `project-${i}`,
        lastOpened: i,
      }));
      mockStoreInstance.get.mockResolvedValue(projects);

      const { projectStore, MAX_RECENT_MENU_ITEMS } = await import('./projectStore');
      await projectStore.init();

      const { eventService } = await import('@/lib/services/eventService');
      const emitCalls = vi
        .mocked(eventService.emit)
        .mock.calls.filter((call) => call[0] === 'update-recent-menu');
      expect(emitCalls.length).toBeGreaterThan(0);
      const lastCall = emitCalls[emitCalls.length - 1];
      expect((lastCall[1] as unknown[]).length).toBeLessThanOrEqual(MAX_RECENT_MENU_ITEMS);
    });

    it('should emit update-recent-menu on init', async () => {
      mockStoreInstance.get.mockResolvedValue([{ path: '/a', name: 'a', lastOpened: 1 }]);

      const { projectStore } = await import('./projectStore');

      const { eventService } = await import('@/lib/services/eventService');
      vi.mocked(eventService.emit).mockClear();

      await projectStore.init();

      expect(eventService.emit).toHaveBeenCalledWith(
        'update-recent-menu',
        expect.arrayContaining([expect.objectContaining({ path: '/a' })])
      );
    });

    it('should emit update-recent-menu after removeProject', async () => {
      mockStoreInstance.get.mockResolvedValue([
        { path: '/a', name: 'a', lastOpened: 1 },
        { path: '/b', name: 'b', lastOpened: 2 },
      ]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      const { eventService } = await import('@/lib/services/eventService');
      vi.mocked(eventService.emit).mockClear();

      await projectStore.removeProject('/a');

      expect(eventService.emit).toHaveBeenCalledWith(
        'update-recent-menu',
        expect.arrayContaining([expect.objectContaining({ path: '/b' })])
      );
    });
  });
});
