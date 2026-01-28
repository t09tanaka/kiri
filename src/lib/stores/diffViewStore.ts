import { writable, derived } from 'svelte/store';

export interface DiffViewState {
  isOpen: boolean;
  projectPath: string | null;
}

const initialState: DiffViewState = {
  isOpen: false,
  projectPath: null,
};

function createDiffViewStore() {
  const { subscribe, set } = writable<DiffViewState>(initialState);

  return {
    subscribe,

    /**
     * Open the diff view modal with the specified project path
     * @param projectPath - Path to the project to display diffs for
     */
    open: (projectPath: string) => {
      set({
        isOpen: true,
        projectPath,
      });
    },

    /**
     * Close the diff view modal
     */
    close: () => set(initialState),
  };
}

export const diffViewStore = createDiffViewStore();

// Derived store for checking if diff view is open
export const isDiffViewOpen = derived(diffViewStore, ($diffView) => $diffView.isOpen);
