import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { worktreeStore, isWorktree, worktreeCount, isSubdirectoryOfRepo } from './worktreeStore';
import { worktreeService } from '@/lib/services/worktreeService';

vi.mock('@/lib/services/worktreeService', () => ({
  worktreeService: {
    list: vi.fn(),
    getContext: vi.fn(),
    create: vi.fn(),
    remove: vi.fn(),
    listBranches: vi.fn(),
  },
}));

describe('worktreeStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    worktreeStore.clear();
  });

  describe('initial state', () => {
    it('should have empty worktrees array', () => {
      const state = get(worktreeStore);
      expect(state.worktrees).toEqual([]);
    });

    it('should have null worktreeContext', () => {
      const state = get(worktreeStore);
      expect(state.worktreeContext).toBe(null);
    });

    it('should have null currentPath', () => {
      const state = get(worktreeStore);
      expect(state.currentPath).toBe(null);
    });

    it('should have isLoading as false', () => {
      const state = get(worktreeStore);
      expect(state.isLoading).toBe(false);
    });

    it('should have null error', () => {
      const state = get(worktreeStore);
      expect(state.error).toBe(null);
    });
  });

  describe('refresh', () => {
    it('should set isLoading to true while fetching', async () => {
      vi.mocked(worktreeService.list).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );
      vi.mocked(worktreeService.getContext).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );

      worktreeStore.refresh('/repo');

      const state = get(worktreeStore);
      expect(state.isLoading).toBe(true);
      expect(state.currentPath).toBe('/repo');
    });

    it('should update worktrees and context on success', async () => {
      const mockWorktrees = [
        {
          name: 'main',
          path: '/repo',
          branch: 'main',
          is_locked: false,
          is_main: true,
          is_valid: true,
        },
        {
          name: 'feature',
          path: '/repo-feature',
          branch: 'feature',
          is_locked: false,
          is_main: false,
          is_valid: true,
        },
      ];
      const mockContext = {
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      };

      vi.mocked(worktreeService.list).mockResolvedValue(mockWorktrees);
      vi.mocked(worktreeService.getContext).mockResolvedValue(mockContext);

      await worktreeStore.refresh('/repo');

      const state = get(worktreeStore);
      expect(state.worktrees).toEqual(mockWorktrees);
      expect(state.worktreeContext).toEqual(mockContext);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe(null);
    });

    it('should set error on failure', async () => {
      vi.mocked(worktreeService.list).mockRejectedValue(new Error('Failed to list'));
      vi.mocked(worktreeService.getContext).mockRejectedValue(new Error('Failed to get context'));

      await worktreeStore.refresh('/repo');

      const state = get(worktreeStore);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe('Failed to list');
    });

    it('should handle non-Error thrown values', async () => {
      vi.mocked(worktreeService.list).mockRejectedValue('string error');
      vi.mocked(worktreeService.getContext).mockRejectedValue('string error');

      await worktreeStore.refresh('/repo');

      const state = get(worktreeStore);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe('string error');
    });
  });

  describe('clear', () => {
    it('should reset to initial state', async () => {
      const mockWorktrees = [
        {
          name: 'main',
          path: '/repo',
          branch: 'main',
          is_locked: false,
          is_main: true,
          is_valid: true,
        },
      ];
      const mockContext = {
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      };

      vi.mocked(worktreeService.list).mockResolvedValue(mockWorktrees);
      vi.mocked(worktreeService.getContext).mockResolvedValue(mockContext);

      await worktreeStore.refresh('/repo');
      worktreeStore.clear();

      const state = get(worktreeStore);
      expect(state.worktrees).toEqual([]);
      expect(state.worktreeContext).toBe(null);
      expect(state.currentPath).toBe(null);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe(null);
    });
  });

  describe('worktreeCount derived store', () => {
    it('should count only linked worktrees (not main)', async () => {
      const mockWorktrees = [
        {
          name: 'main',
          path: '/repo',
          branch: 'main',
          is_locked: false,
          is_main: true,
          is_valid: true,
        },
        {
          name: 'feature-a',
          path: '/repo-a',
          branch: 'feature-a',
          is_locked: false,
          is_main: false,
          is_valid: true,
        },
        {
          name: 'feature-b',
          path: '/repo-b',
          branch: 'feature-b',
          is_locked: false,
          is_main: false,
          is_valid: true,
        },
      ];

      vi.mocked(worktreeService.list).mockResolvedValue(mockWorktrees);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(worktreeCount)).toBe(2);
    });

    it('should return 0 when no linked worktrees', async () => {
      const mockWorktrees = [
        {
          name: 'main',
          path: '/repo',
          branch: 'main',
          is_locked: false,
          is_main: true,
          is_valid: true,
        },
      ];

      vi.mocked(worktreeService.list).mockResolvedValue(mockWorktrees);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(worktreeCount)).toBe(0);
    });
  });

  describe('isWorktree derived store', () => {
    it('should return false when not in a worktree', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(isWorktree)).toBe(false);
    });

    it('should return true when in a worktree', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: true,
        main_repo_path: '/repo',
        worktree_name: 'feature',
      });

      await worktreeStore.refresh('/repo-feature');

      expect(get(isWorktree)).toBe(true);
    });

    it('should return false when context is null', () => {
      expect(get(isWorktree)).toBe(false);
    });
  });

  describe('isSubdirectoryOfRepo derived store', () => {
    it('should return false when at repo root', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(isSubdirectoryOfRepo)).toBe(false);
    });

    it('should return true when in subdirectory of repo', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo/packages/my-package');

      expect(get(isSubdirectoryOfRepo)).toBe(true);
    });

    it('should handle trailing slashes correctly', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: '/repo/',
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(isSubdirectoryOfRepo)).toBe(false);
    });

    it('should return false when currentPath is null', () => {
      expect(get(isSubdirectoryOfRepo)).toBe(false);
    });

    it('should return false when worktreeContext is null', () => {
      expect(get(isSubdirectoryOfRepo)).toBe(false);
    });

    it('should return false when main_repo_path is null', async () => {
      vi.mocked(worktreeService.list).mockResolvedValue([]);
      vi.mocked(worktreeService.getContext).mockResolvedValue({
        is_worktree: false,
        main_repo_path: null,
        worktree_name: null,
      });

      await worktreeStore.refresh('/repo');

      expect(get(isSubdirectoryOfRepo)).toBe(false);
    });
  });
});
