import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type GitFileStatus =
  | 'Modified'
  | 'Added'
  | 'Deleted'
  | 'Renamed'
  | 'Untracked'
  | 'Ignored'
  | 'Conflicted';

export interface GitStatusEntry {
  path: string;
  status: GitFileStatus;
}

export interface GitFileDiff {
  path: string;
  status: GitFileStatus;
  diff: string;
  is_binary: boolean;
  /** Base64 encoded current file content (for binary/image files) */
  current_content_base64: string | null;
  /** Base64 encoded original file content from HEAD (for binary/image files) */
  original_content_base64: string | null;
}

export interface GitRepoInfo {
  root: string;
  branch: string | null;
  statuses: GitStatusEntry[];
  additions: number;
  deletions: number;
}

interface GitStoreState {
  repoInfo: GitRepoInfo | null;
  branchAheadCount: number;
  isLoading: boolean;
  error: string | null;
  allDiffs: GitFileDiff[];
  isDiffsLoading: boolean;
  currentVisibleFile: string | null;
}

function createGitStore() {
  const { subscribe, set, update } = writable<GitStoreState>({
    repoInfo: null,
    branchAheadCount: 0,
    isLoading: false,
    error: null,
    allDiffs: [],
    isDiffsLoading: false,
    currentVisibleFile: null,
  });

  return {
    subscribe,

    async refresh(path: string) {
      update((state) => ({ ...state, isLoading: true, error: null }));

      try {
        const repoInfo = await invoke<GitRepoInfo>('get_git_status', { path });
        let branchAheadCount = 0;
        try {
          branchAheadCount = await invoke<number>('get_branch_ahead_count', { repoPath: path });
        } catch {
          // Ignore errors (e.g., not a git repo or no default branch)
        }
        update((state) => ({
          ...state,
          repoInfo,
          branchAheadCount,
          isLoading: false,
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          repoInfo: null,
          isLoading: false,
          error: error instanceof Error ? error.message : String(error),
        }));
      }
    },

    async loadAllDiffs() {
      let repoRoot: string | null = null;

      update((state) => {
        repoRoot = state.repoInfo?.root ?? null;
        return { ...state, isDiffsLoading: true, error: null };
      });

      if (!repoRoot) {
        update((state) => ({
          ...state,
          isDiffsLoading: false,
          error: 'Repository root not found',
        }));
        return;
      }

      try {
        const allDiffs = await invoke<GitFileDiff[]>('get_all_git_diffs', {
          repoPath: repoRoot,
        });
        update((state) => ({
          ...state,
          allDiffs,
          isDiffsLoading: false,
        }));
      } catch (error) {
        update((state) => ({
          ...state,
          allDiffs: [],
          isDiffsLoading: false,
          error: error instanceof Error ? error.message : String(error),
        }));
      }
    },

    clear() {
      set({
        repoInfo: null,
        branchAheadCount: 0,
        isLoading: false,
        error: null,
        allDiffs: [],
        isDiffsLoading: false,
      });
    },

    clearDiffs() {
      update((state) => ({
        ...state,
        allDiffs: [],
        isDiffsLoading: false,
        currentVisibleFile: null,
      }));
    },

    setCurrentVisibleFile(path: string | null) {
      update((state) => ({
        ...state,
        currentVisibleFile: path,
      }));
    },
  };
}

export const gitStore = createGitStore();

export const gitStatusMap = derived(gitStore, ($gitStore) => {
  const map = new Map<string, GitFileStatus>();

  if ($gitStore.repoInfo) {
    for (const entry of $gitStore.repoInfo.statuses) {
      map.set(entry.path, entry.status);
    }
  }

  return map;
});

export const currentBranch = derived(gitStore, ($gitStore) => $gitStore.repoInfo?.branch ?? null);

export const branchAheadCount = derived(gitStore, ($gitStore) => $gitStore.branchAheadCount);

export function getStatusIcon(status: GitFileStatus): string {
  switch (status) {
    case 'Modified':
      return 'M';
    case 'Added':
      return 'A';
    case 'Deleted':
      return 'D';
    case 'Renamed':
      return 'R';
    case 'Untracked':
      return 'U';
    case 'Ignored':
      return '?';
    case 'Conflicted':
      return '!';
    default:
      return '';
  }
}

export function getStatusColor(status: GitFileStatus): string {
  switch (status) {
    case 'Modified':
      return 'var(--git-modified)';
    case 'Added':
      return 'var(--git-added)';
    case 'Deleted':
      return 'var(--git-deleted)';
    case 'Renamed':
      return 'var(--git-renamed)';
    case 'Untracked':
      return 'var(--git-untracked)';
    case 'Ignored':
      return 'var(--git-ignored)';
    case 'Conflicted':
      return 'var(--git-conflicted)';
    default:
      return 'inherit';
  }
}

// Priority order for status (higher = more important)
const STATUS_PRIORITY: Record<GitFileStatus, number> = {
  Conflicted: 6,
  Deleted: 5,
  Modified: 4,
  Added: 3,
  Renamed: 2,
  Untracked: 1,
  Ignored: 0,
};

export function getDirectoryStatusColor(
  dirRelativePath: string,
  gitStatusMap: Map<string, GitFileStatus>
): string {
  let highestPriorityStatus: GitFileStatus | null = null;
  let highestPriority = -1;

  // Ensure path ends with separator for proper prefix matching
  const prefix = dirRelativePath ? dirRelativePath + '/' : '';

  for (const [filePath, status] of gitStatusMap) {
    // Skip Ignored status - it shouldn't propagate to parent directories
    // Only actual changes (Modified, Added, etc.) should affect directory color
    if (status === 'Ignored') continue;

    // Check if file is inside this directory
    // Note: When prefix is empty, startsWith('') is always true for any string,
    // so we only need to check startsWith(prefix)
    if (filePath.startsWith(prefix)) {
      const priority = STATUS_PRIORITY[status] ?? 0;
      if (priority > highestPriority) {
        highestPriority = priority;
        highestPriorityStatus = status;
      }
    }
  }

  return highestPriorityStatus ? getStatusColor(highestPriorityStatus) : '';
}
