import { writable, derived } from 'svelte/store';
import { worktreeService } from '@/lib/services/worktreeService';
import type { WorktreeInfo, WorktreeContext } from '@/lib/services/worktreeService';

export type { WorktreeInfo, WorktreeContext };

interface WorktreeState {
  worktrees: WorktreeInfo[];
  worktreeContext: WorktreeContext | null;
  currentPath: string | null;
  isLoading: boolean;
  error: string | null;
}

const initialState: WorktreeState = {
  worktrees: [],
  worktreeContext: null,
  currentPath: null,
  isLoading: false,
  error: null,
};

function createWorktreeStore() {
  const { subscribe, set, update } = writable<WorktreeState>(initialState);

  return {
    subscribe,

    /**
     * Refresh worktree list and context for a repository
     */
    refresh: async (repoPath: string) => {
      update((state) => ({ ...state, isLoading: true, error: null, currentPath: repoPath }));
      try {
        const [worktrees, worktreeContext] = await Promise.all([
          worktreeService.list(repoPath),
          worktreeService.getContext(repoPath),
        ]);
        update((state) => ({
          ...state,
          worktrees,
          worktreeContext,
          isLoading: false,
        }));
      } catch (e) {
        update((state) => ({
          ...state,
          isLoading: false,
          error: e instanceof Error ? e.message : String(e),
        }));
      }
    },

    /**
     * Clear worktree state
     */
    clear: () => set(initialState),
  };
}

export const worktreeStore = createWorktreeStore();

// Derived stores
// Count only linked worktrees (excluding main working tree)
export const worktreeCount = derived(
  worktreeStore,
  ($store) => $store.worktrees.filter((wt) => !wt.is_main).length
);

export const isWorktree = derived(
  worktreeStore,
  ($store) => $store.worktreeContext?.is_worktree ?? false
);

/**
 * Check if current path is a subdirectory of the git repository
 * (not the repo root itself)
 */
export const isSubdirectoryOfRepo = derived(worktreeStore, ($store) => {
  if (!$store.currentPath || !$store.worktreeContext?.main_repo_path) {
    return false;
  }
  // Normalize paths by removing trailing slashes for comparison
  const currentPath = $store.currentPath.replace(/\/$/, '');
  const repoRoot = $store.worktreeContext.main_repo_path.replace(/\/$/, '');
  return currentPath !== repoRoot;
});
