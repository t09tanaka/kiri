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

export interface GitRepoInfo {
  root: string;
  branch: string | null;
  statuses: GitStatusEntry[];
}

interface GitStoreState {
  repoInfo: GitRepoInfo | null;
  isLoading: boolean;
  error: string | null;
}

function createGitStore() {
  const { subscribe, set, update } = writable<GitStoreState>({
    repoInfo: null,
    isLoading: false,
    error: null,
  });

  return {
    subscribe,

    async refresh(path: string) {
      update((state) => ({ ...state, isLoading: true, error: null }));

      try {
        const repoInfo = await invoke<GitRepoInfo>('get_git_status', { path });
        update((state) => ({
          ...state,
          repoInfo,
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

    clear() {
      set({
        repoInfo: null,
        isLoading: false,
        error: null,
      });
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
