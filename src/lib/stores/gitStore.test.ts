import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import {
  gitStore,
  gitStatusMap,
  currentBranch,
  getStatusIcon,
  getStatusColor,
  getDirectoryStatusColor,
  type GitFileStatus,
  type GitRepoInfo,
  type GitFileDiff,
} from './gitStore';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe('gitStore helper functions', () => {
  describe('getStatusIcon', () => {
    it('should return M for Modified', () => {
      expect(getStatusIcon('Modified')).toBe('M');
    });

    it('should return A for Added', () => {
      expect(getStatusIcon('Added')).toBe('A');
    });

    it('should return D for Deleted', () => {
      expect(getStatusIcon('Deleted')).toBe('D');
    });

    it('should return R for Renamed', () => {
      expect(getStatusIcon('Renamed')).toBe('R');
    });

    it('should return U for Untracked', () => {
      expect(getStatusIcon('Untracked')).toBe('U');
    });

    it('should return ? for Ignored', () => {
      expect(getStatusIcon('Ignored')).toBe('?');
    });

    it('should return ! for Conflicted', () => {
      expect(getStatusIcon('Conflicted')).toBe('!');
    });

    it('should return empty string for unknown status', () => {
      expect(getStatusIcon('Unknown' as GitFileStatus)).toBe('');
    });
  });

  describe('getStatusColor', () => {
    it('should return git-modified color for Modified', () => {
      expect(getStatusColor('Modified')).toBe('var(--git-modified)');
    });

    it('should return git-added color for Added', () => {
      expect(getStatusColor('Added')).toBe('var(--git-added)');
    });

    it('should return git-deleted color for Deleted', () => {
      expect(getStatusColor('Deleted')).toBe('var(--git-deleted)');
    });

    it('should return git-renamed color for Renamed', () => {
      expect(getStatusColor('Renamed')).toBe('var(--git-renamed)');
    });

    it('should return git-untracked color for Untracked', () => {
      expect(getStatusColor('Untracked')).toBe('var(--git-untracked)');
    });

    it('should return git-ignored color for Ignored', () => {
      expect(getStatusColor('Ignored')).toBe('var(--git-ignored)');
    });

    it('should return git-conflicted color for Conflicted', () => {
      expect(getStatusColor('Conflicted')).toBe('var(--git-conflicted)');
    });

    it('should return inherit for unknown status', () => {
      expect(getStatusColor('Unknown' as GitFileStatus)).toBe('inherit');
    });
  });

  describe('getDirectoryStatusColor', () => {
    it('should return empty string when no status entries', () => {
      const map = new Map<string, GitFileStatus>();

      expect(getDirectoryStatusColor('src', map)).toBe('');
    });

    it('should return color for file in directory', () => {
      const map = new Map<string, GitFileStatus>();
      map.set('src/file.ts', 'Modified');

      expect(getDirectoryStatusColor('src', map)).toBe('var(--git-modified)');
    });

    it('should return highest priority color when multiple statuses', () => {
      const map = new Map<string, GitFileStatus>();
      map.set('src/file1.ts', 'Modified');
      map.set('src/file2.ts', 'Conflicted');
      map.set('src/file3.ts', 'Added');

      // Conflicted has highest priority
      expect(getDirectoryStatusColor('src', map)).toBe('var(--git-conflicted)');
    });

    it('should only consider files in the directory', () => {
      const map = new Map<string, GitFileStatus>();
      map.set('src/file.ts', 'Modified');
      map.set('other/file.ts', 'Conflicted');

      expect(getDirectoryStatusColor('src', map)).toBe('var(--git-modified)');
    });

    it('should handle nested directories', () => {
      const map = new Map<string, GitFileStatus>();
      map.set('src/components/Button.svelte', 'Added');

      expect(getDirectoryStatusColor('src', map)).toBe('var(--git-added)');
      expect(getDirectoryStatusColor('src/components', map)).toBe('var(--git-added)');
    });

    it('should handle root directory (empty path)', () => {
      const map = new Map<string, GitFileStatus>();
      map.set('file.ts', 'Modified');
      map.set('src/other.ts', 'Added');

      expect(getDirectoryStatusColor('', map)).toBe('var(--git-modified)');
    });

    it('should correctly prioritize statuses', () => {
      // Test priority order: Conflicted > Deleted > Modified > Added > Renamed > Untracked > Ignored
      const testCases: [GitFileStatus[], string][] = [
        [['Ignored', 'Untracked'], 'var(--git-untracked)'],
        [['Untracked', 'Renamed'], 'var(--git-renamed)'],
        [['Renamed', 'Added'], 'var(--git-added)'],
        [['Added', 'Modified'], 'var(--git-modified)'],
        [['Modified', 'Deleted'], 'var(--git-deleted)'],
        [['Deleted', 'Conflicted'], 'var(--git-conflicted)'],
      ];

      for (const [statuses, expectedColor] of testCases) {
        const map = new Map<string, GitFileStatus>();
        statuses.forEach((status, i) => {
          map.set(`src/file${i}.ts`, status);
        });
        expect(getDirectoryStatusColor('src', map)).toBe(expectedColor);
      }
    });

    it('should handle unknown status with fallback priority 0', () => {
      const map = new Map<string, GitFileStatus>();
      // Cast unknown status to GitFileStatus to test the fallback ?? 0
      map.set('src/file1.ts', 'UnknownStatus' as GitFileStatus);
      map.set('src/file2.ts', 'Ignored'); // Priority 0

      // UnknownStatus has fallback priority 0, same as Ignored
      // First one encountered wins when priorities are equal
      const result = getDirectoryStatusColor('src', map);
      // The result should be based on whichever was processed first with priority > -1
      expect(result).toBe('inherit'); // UnknownStatus → getStatusColor returns 'inherit'
    });
  });
});

describe('gitStore async methods', () => {
  beforeEach(() => {
    gitStore.clear();
    mockInvoke.mockReset();
  });

  describe('refresh', () => {
    it('should set isLoading to true during refresh', async () => {
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);

      const refreshPromise = gitStore.refresh('/test/repo');

      // Check isLoading is true during the async operation
      const stateBeforeResolve = get(gitStore);
      expect(stateBeforeResolve.isLoading).toBe(true);
      expect(stateBeforeResolve.error).toBeNull();

      await refreshPromise;
    });

    it('should update repoInfo on successful refresh', async () => {
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [
          { path: 'src/file.ts', status: 'Modified' },
          { path: 'src/new.ts', status: 'Added' },
        ],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);

      await gitStore.refresh('/test/repo');

      const state = get(gitStore);
      expect(state.repoInfo).toEqual(mockRepoInfo);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(mockInvoke).toHaveBeenCalledWith('get_git_status', { path: '/test/repo' });
    });

    it('should handle error during refresh', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Git not found'));

      await gitStore.refresh('/test/repo');

      const state = get(gitStore);
      expect(state.repoInfo).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe('Git not found');
    });

    it('should handle non-Error object during refresh', async () => {
      mockInvoke.mockRejectedValueOnce('String error');

      await gitStore.refresh('/test/repo');

      const state = get(gitStore);
      expect(state.repoInfo).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe('String error');
    });
  });

  describe('loadAllDiffs', () => {
    it('should return early with error when no repoInfo', async () => {
      await gitStore.loadAllDiffs();

      const state = get(gitStore);
      expect(state.isDiffsLoading).toBe(false);
      expect(state.error).toBe('Repository root not found');
      expect(mockInvoke).not.toHaveBeenCalled();
    });

    it('should set isDiffsLoading during load', async () => {
      // First set up repoInfo
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      // Now test loadAllDiffs
      const mockDiffs: GitFileDiff[] = [];
      mockInvoke.mockResolvedValueOnce(mockDiffs);

      const loadPromise = gitStore.loadAllDiffs();
      await loadPromise;

      const state = get(gitStore);
      expect(state.isDiffsLoading).toBe(false);
    });

    it('should load diffs successfully', async () => {
      // First set up repoInfo
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      // Now test loadAllDiffs
      const mockDiffs: GitFileDiff[] = [
        { path: 'src/file.ts', status: 'Modified', diff: '+line1\n-line2' },
        { path: 'src/new.ts', status: 'Added', diff: '+new content' },
      ];
      mockInvoke.mockResolvedValueOnce(mockDiffs);

      await gitStore.loadAllDiffs();

      const state = get(gitStore);
      expect(state.allDiffs).toEqual(mockDiffs);
      expect(state.isDiffsLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(mockInvoke).toHaveBeenCalledWith('get_all_git_diffs', {
        repoPath: '/test/repo',
      });
    });

    it('should handle error during loadAllDiffs', async () => {
      // First set up repoInfo
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      // Now test loadAllDiffs with error
      mockInvoke.mockRejectedValueOnce(new Error('Diff failed'));

      await gitStore.loadAllDiffs();

      const state = get(gitStore);
      expect(state.allDiffs).toEqual([]);
      expect(state.isDiffsLoading).toBe(false);
      expect(state.error).toBe('Diff failed');
    });

    it('should handle non-Error object during loadAllDiffs', async () => {
      // First set up repoInfo
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      // Now test loadAllDiffs with string error
      mockInvoke.mockRejectedValueOnce('String diff error');

      await gitStore.loadAllDiffs();

      const state = get(gitStore);
      expect(state.allDiffs).toEqual([]);
      expect(state.isDiffsLoading).toBe(false);
      expect(state.error).toBe('String diff error');
    });
  });
});

describe('gitStore state management', () => {
  beforeEach(() => {
    gitStore.clear();
  });

  describe('clear', () => {
    it('should reset store to initial state', () => {
      gitStore.clear();

      const state = get(gitStore);
      expect(state.repoInfo).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.allDiffs).toEqual([]);
      expect(state.isDiffsLoading).toBe(false);
    });
  });

  describe('clearDiffs', () => {
    it('should clear diffs and currentVisibleFile', () => {
      gitStore.clearDiffs();

      const state = get(gitStore);
      expect(state.allDiffs).toEqual([]);
      expect(state.isDiffsLoading).toBe(false);
      expect(state.currentVisibleFile).toBeNull();
    });
  });

  describe('setCurrentVisibleFile', () => {
    it('should set current visible file', () => {
      gitStore.setCurrentVisibleFile('/path/to/file.ts');

      const state = get(gitStore);
      expect(state.currentVisibleFile).toBe('/path/to/file.ts');
    });

    it('should set to null', () => {
      gitStore.setCurrentVisibleFile('/path/to/file.ts');
      gitStore.setCurrentVisibleFile(null);

      const state = get(gitStore);
      expect(state.currentVisibleFile).toBeNull();
    });
  });
});

describe('gitStore derived stores', () => {
  beforeEach(() => {
    gitStore.clear();
    mockInvoke.mockReset();
  });

  describe('gitStatusMap', () => {
    it('should return empty map when no repo info', () => {
      const map = get(gitStatusMap);
      expect(map.size).toBe(0);
    });

    it('should populate map from repoInfo statuses', async () => {
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'main',
        statuses: [
          { path: 'src/file.ts', status: 'Modified' },
          { path: 'src/new.ts', status: 'Added' },
          { path: 'src/deleted.ts', status: 'Deleted' },
        ],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      const map = get(gitStatusMap);
      expect(map.size).toBe(3);
      expect(map.get('src/file.ts')).toBe('Modified');
      expect(map.get('src/new.ts')).toBe('Added');
      expect(map.get('src/deleted.ts')).toBe('Deleted');
    });
  });

  describe('currentBranch', () => {
    it('should return null when no repo info', () => {
      expect(get(currentBranch)).toBeNull();
    });

    it('should return branch from repoInfo', async () => {
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: 'feature-branch',
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      expect(get(currentBranch)).toBe('feature-branch');
    });

    it('should return null when branch is null in repoInfo', async () => {
      const mockRepoInfo: GitRepoInfo = {
        root: '/test/repo',
        branch: null,
        statuses: [],
      };
      mockInvoke.mockResolvedValueOnce(mockRepoInfo);
      await gitStore.refresh('/test/repo');

      expect(get(currentBranch)).toBeNull();
    });
  });
});
