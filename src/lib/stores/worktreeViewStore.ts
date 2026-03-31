import { writable, derived } from 'svelte/store';

export interface WorktreeViewState {
  isOpen: boolean;
  projectPath: string | null;
  autoCreateBranch: string | null;
}

const initialState: WorktreeViewState = {
  isOpen: false,
  projectPath: null,
  autoCreateBranch: null,
};

function createWorktreeViewStore() {
  const { subscribe, set } = writable<WorktreeViewState>(initialState);

  return {
    subscribe,

    /**
     * Open the worktree panel modal with the specified project path
     */
    open: (projectPath: string) => {
      set({
        isOpen: true,
        projectPath,
        autoCreateBranch: null,
      });
    },

    /**
     * Open the worktree panel and auto-create a worktree for the given branch
     */
    openAndCreate: (projectPath: string, branchName: string) => {
      set({
        isOpen: true,
        projectPath,
        autoCreateBranch: branchName,
      });
    },

    /**
     * Close the worktree panel modal
     */
    close: () => set(initialState),
  };
}

export const worktreeViewStore = createWorktreeViewStore();

// Derived store for checking if worktree view is open
export const isWorktreeViewOpen = derived(
  worktreeViewStore,
  ($worktreeView) => $worktreeView.isOpen
);
