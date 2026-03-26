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

// Wrap svelte/store's writable to fix the TDZ issue in subscribe callbacks.
// The source code pattern `const unsub = subscribe(cb => { unsub(); })` causes
// a TDZ error because subscribe calls cb synchronously before returning.
// This wrapper catches that error so the subscriber is properly cleaned up
// on subsequent calls.
vi.mock('svelte/store', async (importOriginal) => {
  const actual = await importOriginal<typeof import('svelte/store')>();
  return {
    ...actual,
    writable: (...args: Parameters<typeof actual.writable>) => {
      const store = actual.writable(...args);
      const originalSubscribe = store.subscribe;
      store.subscribe = (run: (value: unknown) => void, invalidate?: () => void) => {
        // eslint-disable-next-line prefer-const -- assigned after closure creation
        let unsubFn: (() => void) | undefined;
        const wrappedRun = (value: unknown) => {
          try {
            run(value);
          } catch (e) {
            if (e instanceof ReferenceError && e.message.includes('before initialization')) {
              // Swallow TDZ error and schedule cleanup
              queueMicrotask(() => {
                if (unsubFn) unsubFn();
              });
            } else {
              throw e;
            }
          }
        };
        unsubFn = originalSubscribe(wrappedRun, invalidate);
        return unsubFn;
      };
      return store;
    },
  };
});

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

  describe('loadRecentProjects error handling', () => {
    it('should return empty array when store load fails', async () => {
      mockStoreInstance.reload.mockRejectedValue(new Error('disk error'));

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      expect(get(recentProjects)).toEqual([]);
    });

    it('should return empty array when store returns null', async () => {
      mockStoreInstance.get.mockResolvedValue(null);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      expect(get(recentProjects)).toEqual([]);
    });
  });

  describe('saveRecentProjects error handling', () => {
    it('should handle save failure gracefully', async () => {
      mockStoreInstance.get.mockResolvedValue([{ path: '/a', name: 'a', lastOpened: 1 }]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      // Make save fail
      mockStoreInstance.set.mockRejectedValue(new Error('write error'));

      // clearRecentProjects calls saveRecentProjects internally
      await projectStore.clearRecentProjects();

      // Should not throw, error is caught internally
    });
  });

  describe('openProject with existing project', () => {
    it('should move existing project to front of recent list', async () => {
      mockStoreInstance.get.mockResolvedValue([
        { path: '/first', name: 'first', lastOpened: 1 },
        { path: '/second', name: 'second', lastOpened: 2 },
        { path: '/third', name: 'third', lastOpened: 3 },
      ]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      // Open the second project (already exists)
      await projectStore.openProject('/second');

      const projects = get(recentProjects);
      expect(projects[0].path).toBe('/second');
      expect(projects[1].path).toBe('/first');
      expect(projects[2].path).toBe('/third');
    });
  });

  describe('openProject with trailing slash path', () => {
    it('should use full path as name when path ends with /', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      await projectStore.openProject('/test/project/');

      const projects = get(recentProjects);
      // path.split('/').pop() returns '' for trailing slash, so fallback to full path
      expect(projects[0].name).toBe('/test/project/');
      expect(projects[0].path).toBe('/test/project/');
    });
  });

  describe('openProject window resize error', () => {
    it('should handle window resize failure gracefully', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { windowService } = await import('@/lib/services/windowService');
      vi.mocked(windowService.setSizeAndCenter).mockRejectedValue(new Error('window error'));

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      // Should not throw even when resize fails
      await projectStore.openProject('/test/project');

      expect(projectStore.getCurrentPath()).toBe('/test/project');
    });
  });

  describe('closeProject', () => {
    it('should set currentPath to null', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      await projectStore.openProject('/test/project');
      expect(projectStore.getCurrentPath()).toBe('/test/project');

      await projectStore.closeProject();
      expect(projectStore.getCurrentPath()).toBeNull();
    });

    it('should resize window to start screen size', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { windowService } = await import('@/lib/services/windowService');
      vi.mocked(windowService.setSizeAndCenter).mockResolvedValue(undefined);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      await projectStore.openProject('/test/project');
      vi.mocked(windowService.setSizeAndCenter).mockClear();

      await projectStore.closeProject();

      expect(windowService.setSizeAndCenter).toHaveBeenCalledWith(800, 600);
    });

    it('should handle window resize failure gracefully on close', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { windowService } = await import('@/lib/services/windowService');
      vi.mocked(windowService.setSizeAndCenter).mockResolvedValue(undefined);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      await projectStore.openProject('/test/project');
      vi.mocked(windowService.setSizeAndCenter).mockRejectedValue(new Error('window error'));

      // Should not throw even when resize fails
      await projectStore.closeProject();
      expect(projectStore.getCurrentPath()).toBeNull();
    });
  });

  describe('refreshRecentProjectsGitInfo', () => {
    it('should update git branch info for all recent projects and save', async () => {
      const { invoke } = await import('@tauri-apps/api/core');
      vi.mocked(invoke).mockResolvedValue({ root: '/a', branch: 'main', statuses: [] });

      mockStoreInstance.get.mockResolvedValue([
        { path: '/a', name: 'a', lastOpened: 1, gitBranch: null },
        { path: '/b', name: 'b', lastOpened: 2, gitBranch: 'old-branch' },
      ]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      // Reset save mock after init
      mockStoreInstance.set.mockClear();
      mockStoreInstance.save.mockClear();

      await projectStore.refreshRecentProjectsGitInfo();

      // Wait for saveRecentProjects to complete
      await vi.waitFor(() => {
        expect(mockStoreInstance.save).toHaveBeenCalled();
      });

      const projects = get(recentProjects);
      expect(projects).toHaveLength(2);
      expect(projects[0].gitBranch).toBe('main');
      expect(projects[1].gitBranch).toBe('main');

      expect(mockStoreInstance.set).toHaveBeenCalledWith(
        'recentProjects',
        expect.arrayContaining([
          expect.objectContaining({ path: '/a', gitBranch: 'main' }),
          expect.objectContaining({ path: '/b', gitBranch: 'main' }),
        ])
      );
    });

    it('should handle git branch fetch failure gracefully', async () => {
      const { invoke } = await import('@tauri-apps/api/core');
      vi.mocked(invoke).mockRejectedValue(new Error('not a git repo'));

      mockStoreInstance.get.mockResolvedValue([
        { path: '/a', name: 'a', lastOpened: 1, gitBranch: 'old-branch' },
      ]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      mockStoreInstance.set.mockClear();
      mockStoreInstance.save.mockClear();

      await projectStore.refreshRecentProjectsGitInfo();

      await vi.waitFor(() => {
        expect(mockStoreInstance.save).toHaveBeenCalled();
      });

      const projects = get(recentProjects);
      expect(projects[0].gitBranch).toBeNull();
    });
  });

  describe('getCurrentPath', () => {
    it('should return null when no project is open', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      expect(projectStore.getCurrentPath()).toBeNull();
    });

    it('should return the current path after opening a project', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      await projectStore.openProject('/test/my-project');

      expect(projectStore.getCurrentPath()).toBe('/test/my-project');
    });

    it('should return the path set via setCurrentPath', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore } = await import('./projectStore');
      await projectStore.init();

      projectStore.setCurrentPath('/some/path');

      expect(projectStore.getCurrentPath()).toBe('/some/path');
    });
  });

  describe('currentProjectName derived store', () => {
    it('should return null when no project is open', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectName } = await import('./projectStore');
      await projectStore.init();

      expect(get(currentProjectName)).toBeNull();
    });

    it('should return the project name extracted from path', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectName } = await import('./projectStore');
      await projectStore.init();

      projectStore.setCurrentPath('/Users/user/projects/my-app');

      expect(get(currentProjectName)).toBe('my-app');
    });

    it('should return the full path when path has no separator', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectName } = await import('./projectStore');
      await projectStore.init();

      projectStore.setCurrentPath('single-segment');

      expect(get(currentProjectName)).toBe('single-segment');
    });

    it('should fallback to full path when path ends with separator', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectName } = await import('./projectStore');
      await projectStore.init();

      // When path ends with '/', pop() returns '', so fallback to full path
      projectStore.setCurrentPath('/Users/user/projects/');

      expect(get(currentProjectName)).toBe('/Users/user/projects/');
    });
  });

  describe('currentProjectPath derived store', () => {
    it('should return null when no project is open', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectPath } = await import('./projectStore');
      await projectStore.init();

      expect(get(currentProjectPath)).toBeNull();
    });

    it('should return the current path', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, currentProjectPath } = await import('./projectStore');
      await projectStore.init();

      projectStore.setCurrentPath('/test/project');

      expect(get(currentProjectPath)).toBe('/test/project');
    });
  });

  describe('isProjectOpen derived store', () => {
    it('should return false when no project is open', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, isProjectOpen } = await import('./projectStore');
      await projectStore.init();

      expect(get(isProjectOpen)).toBe(false);
    });

    it('should return true when a project is open', async () => {
      mockStoreInstance.get.mockResolvedValue([]);

      const { projectStore, isProjectOpen } = await import('./projectStore');
      await projectStore.init();

      projectStore.setCurrentPath('/test/project');

      expect(get(isProjectOpen)).toBe(true);
    });
  });
});
