import { writable, derived } from 'svelte/store';
import type { CommitInfo, CommitDiffResult } from '@/lib/services/gitService';

export interface CommitHistoryState {
  isOpen: boolean;
  projectPath: string | null;
  commits: CommitInfo[];
  selectedCommitHash: string | null;
  selectedCommitDiff: CommitDiffResult | null;
  isLoadingLog: boolean;
  isLoadingMore: boolean;
  isLoadingDiff: boolean;
  isPushing: boolean;
  hasMore: boolean;
  error: string | null;
}

const initialState: CommitHistoryState = {
  isOpen: false,
  projectPath: null,
  commits: [],
  selectedCommitHash: null,
  selectedCommitDiff: null,
  isLoadingLog: false,
  isLoadingMore: false,
  isLoadingDiff: false,
  isPushing: false,
  hasMore: true,
  error: null,
};

function createCommitHistoryStore() {
  const { subscribe, set, update } = writable<CommitHistoryState>(initialState);

  return {
    subscribe,

    /**
     * Open the commit history modal with the specified project path
     */
    open: (projectPath: string) => {
      update((s) => ({
        ...s,
        isOpen: true,
        projectPath,
        error: null,
      }));
    },

    /**
     * Close the commit history modal and reset state
     */
    close: () => set(initialState),

    /**
     * Set the commits list after loading
     */
    setCommits: (commits: CommitInfo[], pageSize: number = 50) => {
      update((s) => ({
        ...s,
        commits,
        isLoadingLog: false,
        hasMore: commits.length >= pageSize,
      }));
    },

    /**
     * Append commits for infinite scroll
     */
    appendCommits: (newCommits: CommitInfo[], pageSize: number = 50) => {
      update((s) => ({
        ...s,
        commits: [...s.commits, ...newCommits],
        isLoadingMore: false,
        hasMore: newCommits.length >= pageSize,
      }));
    },

    /**
     * Set loading state for loading more commits
     */
    setLoadingMore: (loading: boolean) => {
      update((s) => ({ ...s, isLoadingMore: loading }));
    },

    /**
     * Select a commit by hash, clearing previous diff
     */
    selectCommit: (hash: string | null) => {
      update((s) => ({
        ...s,
        selectedCommitHash: hash,
        selectedCommitDiff: null,
      }));
    },

    /**
     * Set the diff result for the selected commit
     */
    setCommitDiff: (diff: CommitDiffResult | null) => {
      update((s) => ({
        ...s,
        selectedCommitDiff: diff,
        isLoadingDiff: false,
      }));
    },

    /**
     * Set loading state for commit log
     */
    setLoadingLog: (loading: boolean) => {
      update((s) => ({ ...s, isLoadingLog: loading }));
    },

    /**
     * Set loading state for commit diff
     */
    setLoadingDiff: (loading: boolean) => {
      update((s) => ({ ...s, isLoadingDiff: loading }));
    },

    /**
     * Set pushing state
     */
    setPushing: (pushing: boolean) => {
      update((s) => ({ ...s, isPushing: pushing }));
    },

    /**
     * Set error and clear all loading states
     */
    setError: (error: string | null) => {
      update((s) => ({
        ...s,
        error,
        isLoadingLog: false,
        isLoadingMore: false,
        isLoadingDiff: false,
        isPushing: false,
      }));
    },

    /**
     * Mark all commits as pushed after a successful push
     */
    markAllPushed: () => {
      update((s) => ({
        ...s,
        commits: s.commits.map((c) => ({ ...c, is_pushed: true })),
        isPushing: false,
      }));
    },
  };
}

export const commitHistoryStore = createCommitHistoryStore();

// Derived store for checking if commit history is open
export const isCommitHistoryOpen = derived(commitHistoryStore, ($store) => $store.isOpen);

// Derived store for counting unpushed commits
export const unpushedCount = derived(
  commitHistoryStore,
  ($store) => $store.commits.filter((c) => !c.is_pushed).length
);
