import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { prStore, prCount, hasPrs } from './prStore';
import { prService } from '@/lib/services/prService';
import type { PullRequest } from '@/lib/services/prService';

vi.mock('@/lib/services/prService', () => ({
  prService: {
    checkGhCli: vi.fn(),
    listPrs: vi.fn(),
    getPrDetail: vi.fn(),
  },
}));

function makeMockPr(overrides: Partial<PullRequest> = {}): PullRequest {
  return {
    number: 1,
    title: 'test',
    author_login: 'dev',
    head_ref_name: 'test',
    state: 'OPEN',
    updated_at: '',
    additions: 0,
    deletions: 0,
    changed_files: 0,
    body: '',
    review_decision: null,
    status_check_rollup: [],
    labels: [],
    files: [],
    ...overrides,
  };
}

describe('prStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    prStore.clear();
  });

  describe('initial state', () => {
    it('should have empty prs array', () => {
      const state = get(prStore);
      expect(state.prs).toEqual([]);
    });

    it('should have null selectedPr', () => {
      const state = get(prStore);
      expect(state.selectedPr).toBeNull();
    });

    it('should have isLoading as false', () => {
      const state = get(prStore);
      expect(state.isLoading).toBe(false);
    });

    it('should have null error', () => {
      const state = get(prStore);
      expect(state.error).toBeNull();
    });

    it('should have ghAvailable as false', () => {
      const state = get(prStore);
      expect(state.ghAvailable).toBe(false);
    });
  });

  describe('refresh', () => {
    it('should set isLoading to true while fetching', () => {
      vi.mocked(prService.checkGhCli).mockImplementation(
        () => new Promise(() => {}) // Never resolves
      );

      prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.isLoading).toBe(true);
    });

    it('should load PRs on success', async () => {
      const mockPrs = [
        makeMockPr({ number: 1, title: 'PR 1' }),
        makeMockPr({ number: 2, title: 'PR 2' }),
      ];

      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue(mockPrs);

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.prs).toEqual(mockPrs);
      expect(state.ghAvailable).toBe(true);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
    });

    it('should set ghAvailable false when gh CLI not installed', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: false, authenticated: false });

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.ghAvailable).toBe(false);
      expect(state.prs).toEqual([]);
      expect(state.isLoading).toBe(false);
    });

    it('should set ghAvailable false when not authenticated', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: false });

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.ghAvailable).toBe(false);
      expect(state.prs).toEqual([]);
      expect(state.isLoading).toBe(false);
    });

    it('should set error on fetch failure', async () => {
      vi.mocked(prService.checkGhCli).mockRejectedValue(new Error('Network error'));

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.error).toBe('Network error');
      expect(state.isLoading).toBe(false);
    });

    it('should handle non-Error thrown values', async () => {
      vi.mocked(prService.checkGhCli).mockRejectedValue('string error');

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.error).toBe('string error');
      expect(state.isLoading).toBe(false);
    });

    it('should set error when listPrs fails', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockRejectedValue(new Error('List failed'));

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.error).toBe('List failed');
      expect(state.isLoading).toBe(false);
    });

    it('should clear previous error on new refresh', async () => {
      vi.mocked(prService.checkGhCli).mockRejectedValue(new Error('First error'));
      await prStore.refresh('/repo');

      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue([]);

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.error).toBeNull();
    });
  });

  describe('selectPr', () => {
    it('should set selectedPr on success', async () => {
      const mockPr = makeMockPr({ number: 42, title: 'Detailed PR' });
      vi.mocked(prService.getPrDetail).mockResolvedValue(mockPr);

      await prStore.selectPr('/repo', 42);

      const state = get(prStore);
      expect(state.selectedPr).toEqual(mockPr);
      expect(state.error).toBeNull();
    });

    it('should set error and clear selectedPr on failure', async () => {
      vi.mocked(prService.getPrDetail).mockRejectedValue(new Error('Not found'));

      // Set a selected PR first
      const mockPr = makeMockPr();
      vi.mocked(prService.getPrDetail).mockResolvedValueOnce(mockPr);
      await prStore.selectPr('/repo', 1);

      // Now fail
      vi.mocked(prService.getPrDetail).mockRejectedValue(new Error('Not found'));
      await prStore.selectPr('/repo', 999);

      const state = get(prStore);
      expect(state.selectedPr).toBeNull();
      expect(state.error).toBe('Not found');
    });

    it('should handle non-Error thrown values', async () => {
      vi.mocked(prService.getPrDetail).mockRejectedValue('string error');

      await prStore.selectPr('/repo', 1);

      const state = get(prStore);
      expect(state.selectedPr).toBeNull();
      expect(state.error).toBe('string error');
    });
  });

  describe('clearSelection', () => {
    it('should clear selectedPr', async () => {
      const mockPr = makeMockPr({ number: 1 });
      vi.mocked(prService.getPrDetail).mockResolvedValue(mockPr);
      await prStore.selectPr('/repo', 1);

      prStore.clearSelection();

      const state = get(prStore);
      expect(state.selectedPr).toBeNull();
    });

    it('should not affect other state when clearing selection', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue([makeMockPr()]);
      await prStore.refresh('/repo');

      prStore.clearSelection();

      const state = get(prStore);
      expect(state.prs).toHaveLength(1);
      expect(state.ghAvailable).toBe(true);
    });
  });

  describe('clear', () => {
    it('should reset to initial state', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue([makeMockPr()]);
      await prStore.refresh('/repo');

      prStore.clear();

      const state = get(prStore);
      expect(state.prs).toEqual([]);
      expect(state.selectedPr).toBeNull();
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.ghAvailable).toBe(false);
    });
  });

  describe('prCount derived store', () => {
    it('should return 0 when no PRs', () => {
      expect(get(prCount)).toBe(0);
    });

    it('should return count of loaded PRs', async () => {
      const mockPrs = [
        makeMockPr({ number: 1 }),
        makeMockPr({ number: 2 }),
        makeMockPr({ number: 3 }),
      ];
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue(mockPrs);

      await prStore.refresh('/repo');

      expect(get(prCount)).toBe(3);
    });
  });

  describe('hasPrs derived store', () => {
    it('should return false when no PRs', () => {
      expect(get(hasPrs)).toBe(false);
    });

    it('should return true when PRs are loaded', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue([makeMockPr()]);

      await prStore.refresh('/repo');

      expect(get(hasPrs)).toBe(true);
    });

    it('should return false after clearing store', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({ installed: true, authenticated: true });
      vi.mocked(prService.listPrs).mockResolvedValue([makeMockPr()]);
      await prStore.refresh('/repo');

      prStore.clear();

      expect(get(hasPrs)).toBe(false);
    });
  });
});
