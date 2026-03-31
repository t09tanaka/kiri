import { writable, derived } from 'svelte/store';

export interface PrMetadata {
  number: number;
  title: string;
  branch: string;
  ciStatus: string;
}

export interface WorktreeViewState {
  isOpen: boolean;
  projectPath: string | null;
  autoCreateBranch: string | null;
  prMetadata: PrMetadata | null;
}

const initialState: WorktreeViewState = {
  isOpen: false,
  projectPath: null,
  autoCreateBranch: null,
  prMetadata: null,
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
        prMetadata: null,
      });
    },

    /**
     * Open the worktree panel and auto-create a worktree for the given branch
     */
    openAndCreate: (projectPath: string, branchName: string, prMetadata?: PrMetadata) => {
      set({
        isOpen: true,
        projectPath,
        autoCreateBranch: branchName,
        prMetadata: prMetadata ?? null,
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
